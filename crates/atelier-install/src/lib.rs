//! Archive staging, mod-root detection, install preview, transactional install, backups and rollback.

mod backup;
mod classify;
mod error;
mod extract;
mod fsops;
mod install;
mod roots;
mod stage;
mod types;
mod version;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use atelier_core::{GameId, Mod};

use classify::Classifier;
use install::StagedItem;

pub use error::{Error, Result};
pub use types::*;

const GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub staging_dir: PathBuf,
    pub backup_dir: PathBuf,
    pub retention_days: Option<u32>,
}

/// Caps per dropped source, nested archives included, so a zip bomb fails before it fills the disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_total_bytes: u64,
    pub max_entries: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_total_bytes: 4 * GIB,
            max_entries: 50_000,
        }
    }
}

struct StagedPreview {
    dir: PathBuf,
    game: GameId,
    items: Vec<StagedItem>,
}

pub struct InstallEngine {
    config: EngineConfig,
    limits: Limits,
    previews: Mutex<HashMap<String, StagedPreview>>,
}

impl InstallEngine {
    pub fn new(config: EngineConfig) -> Self {
        Self::with_limits(config, Limits::default())
    }

    pub fn with_limits(config: EngineConfig, limits: Limits) -> Self {
        Self {
            config,
            limits,
            previews: Mutex::new(HashMap::new()),
        }
    }

    /// Stages `sources` and describes what installing them would do. Writes nothing to any mods folder.
    pub fn preview(
        &self,
        game: GameId,
        sources: &[PathBuf],
        installed: &[Mod],
    ) -> Result<InstallPreview> {
        let id = uuid::Uuid::new_v4().to_string();
        let dir = self.config.staging_dir.join(&id);
        let items = match self.stage_and_classify(&dir, game, sources, installed) {
            Ok(items) => items,
            Err(error) => {
                let _ = fs::remove_dir_all(&dir);
                return Err(error);
            }
        };
        let preview = InstallPreview {
            id: id.clone(),
            source_name: sources
                .iter()
                .map(|source| fsops::file_name(source))
                .collect::<Vec<_>>()
                .join(", "),
            items: items.iter().map(|staged| staged.item.clone()).collect(),
        };
        self.lock_previews()
            .insert(id, StagedPreview { dir, game, items });
        Ok(preview)
    }

    /// Installs the preview's eligible items into `target_mods_dir`; on failure nothing changes.
    /// `installed` is a fresh scan: if any item's installed copy changed since the preview, the
    /// player's decision no longer holds and nothing is installed.
    pub fn confirm(
        &self,
        preview_id: &str,
        target_mods_dir: &Path,
        installed: &[Mod],
        include_downgrades: bool,
    ) -> Result<InstallResult> {
        let staged = self
            .lock_previews()
            .remove(preview_id)
            .ok_or_else(|| Error::PreviewNotFound(preview_id.to_string()))?;
        if let Some(changed) = classify::changed_since_preview(
            staged.game,
            &staged.items,
            installed,
            include_downgrades,
        ) {
            self.lock_previews().insert(preview_id.to_string(), staged);
            return Err(Error::PreviewStale(changed));
        }
        match install::install_all(
            &staged.items,
            target_mods_dir,
            &self.game_backups(staged.game),
            include_downgrades,
        ) {
            Ok(result) => {
                // Leftovers are skipped items; the install itself already succeeded.
                let _ = fs::remove_dir_all(&staged.dir);
                Ok(result)
            }
            Err(error) => {
                // Keep the preview so the player can retry once the cause is fixed.
                self.lock_previews().insert(preview_id.to_string(), staged);
                Err(error)
            }
        }
    }

    /// Drops the preview and its staging. Unknown ids are a no-op, so a double cancel is harmless.
    pub fn cancel(&self, preview_id: &str) -> Result<()> {
        let Some(staged) = self.lock_previews().remove(preview_id) else {
            return Ok(());
        };
        fs::remove_dir_all(&staged.dir).map_err(error::io_at(&staged.dir))
    }

    /// Restores the newest backup of `game`'s `mod_id` into `target_mods_dir`.
    pub fn rollback(&self, game: GameId, mod_id: &str, target_mods_dir: &Path) -> Result<()> {
        backup::rollback(&self.game_backups(game), target_mods_dir, mod_id)
    }

    /// Lets a same-version item install as an update, for a new mod.io file whose author
    /// didn't bump the manifest version. Other statuses are left alone.
    pub fn allow_reinstall(&self, preview_id: &str, mod_id: &str) -> Result<()> {
        let mut previews = self.lock_previews();
        let staged = previews
            .get_mut(preview_id)
            .ok_or_else(|| Error::PreviewNotFound(preview_id.to_string()))?;
        for item in staged.items.iter_mut().map(|staged| &mut staged.item) {
            if item.mod_id == mod_id && item.status == InstallItemStatus::Same {
                item.status = InstallItemStatus::Update;
            }
        }
        Ok(())
    }

    /// Moves an installed mod's folder into the backups, where `rollback` can find it.
    pub fn remove_to_backup(&self, game: GameId, mod_dir: &Path) -> Result<()> {
        let backup = backup::backup_path(&self.game_backups(game), mod_dir)?;
        fsops::move_dir(mod_dir, &backup)
    }

    pub fn prune_backups(&self) -> Result<()> {
        for game in [GameId::Anno117, GameId::Anno1800] {
            backup::prune(&self.game_backups(game), self.config.retention_days)?;
        }
        Ok(())
    }

    /// One folder per game: the same ModID in both games must never share a backup.
    fn game_backups(&self, game: GameId) -> PathBuf {
        let folder = match game {
            GameId::Anno117 => "anno117",
            GameId::Anno1800 => "anno1800",
        };
        self.config.backup_dir.join(folder)
    }

    fn stage_and_classify(
        &self,
        dir: &Path,
        game: GameId,
        sources: &[PathBuf],
        installed: &[Mod],
    ) -> Result<Vec<StagedItem>> {
        let mut found = Vec::new();
        for (index, source) in sources.iter().enumerate() {
            let staged = dir.join(format!("src-{index}"));
            stage::stage_source(source, &staged, self.limits)?;
            let mut roots = roots::find_roots(&staged)?;
            roots.sort_by_key(roots::FoundRoot::mod_id);
            found.extend(roots.into_iter().map(|root| (staged.clone(), root)));
        }
        let mut classifier = Classifier::new(game, installed, found.iter().map(|(_, root)| root));
        Ok(found
            .iter()
            .map(|(staged, root)| StagedItem {
                item: classifier.describe(staged, root),
                staged_dir: root.dir.clone(),
            })
            .collect())
    }

    fn lock_previews(&self) -> MutexGuard<'_, HashMap<String, StagedPreview>> {
        // A panic mid-insert can't leave the map half-updated, so a poisoned lock is still usable.
        self.previews.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
