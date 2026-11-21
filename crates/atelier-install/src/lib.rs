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

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};

use atelier_core::{GameId, Mod};

use classify::Classifier;
use install::StagedItem;

pub use backup::{BackupVersion, KeptVersion};
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

/// The most a declared unpacked size can raise the cap to.
const DECLARED_SIZE_CEILING: u64 = 64 * GIB;
/// Game mods unpack to a few times their archive; a zip bomb unpacks to thousands of times.
const MAX_UNPACK_RATIO: u64 = 10;

impl Limits {
    /// For a download whose unpacked size the host measured (mod.io's `filesize_uncompressed`):
    /// that size plus 10%, held to 10x the archive actually downloaded so a bomb gains nothing,
    /// and never below the default or above 64 GiB.
    pub fn for_download(declared_unpacked: u64, archive_bytes: u64) -> Self {
        let default = Self::default();
        let wanted = declared_unpacked
            .saturating_add(declared_unpacked / 10)
            .min(archive_bytes.saturating_mul(MAX_UNPACK_RATIO));
        Self {
            max_total_bytes: wanted.clamp(default.max_total_bytes, DECLARED_SIZE_CEILING),
            ..default
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_total_bytes: 4 * GIB,
            max_entries: 50_000,
        }
    }
}

/// Per-call preview settings: caps, Humankind's install folder for plugins, and the
/// `<modId>_<modfileId>` folder a mod.io download installs under.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreviewOptions {
    pub limits: Option<Limits>,
    pub game_dir: Option<PathBuf>,
    pub modio_folder: Option<String>,
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
        self.preview_with_limits(game, sources, installed, self.limits)
    }

    /// [`Self::preview`] with caps chosen per call, e.g. from a download's declared size.
    pub fn preview_with_limits(
        &self,
        game: GameId,
        sources: &[PathBuf],
        installed: &[Mod],
        limits: Limits,
    ) -> Result<InstallPreview> {
        let options = PreviewOptions {
            limits: Some(limits),
            ..PreviewOptions::default()
        };
        self.preview_with_options(game, sources, installed, &options)
    }

    /// [`Self::preview`] with every per-call setting.
    pub fn preview_with_options(
        &self,
        game: GameId,
        sources: &[PathBuf],
        installed: &[Mod],
        options: &PreviewOptions,
    ) -> Result<InstallPreview> {
        let id = uuid::Uuid::new_v4().to_string();
        let dir = self.config.staging_dir.join(&id);
        let items = match self.stage_and_classify(&dir, game, sources, installed, options) {
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
        if game != GameId::Humankind {
            return backup::rollback(&self.game_backups(game), target_mods_dir, mod_id);
        }
        // A module's folder is named by its long name, not its id, so find the copy it replaces.
        let newest = backup::list_versions(&self.game_backups(game), mod_id)?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NoBackup(mod_id.to_string()))?;
        let current = install::find_module(&target_mods_dir.join(install::ATELIER_FOLDER), mod_id)?;
        self.restore_version(
            game,
            mod_id,
            &newest.version,
            target_mods_dir,
            current.as_deref(),
        )
    }

    /// Restores the newest backup of `mod_id` at `version`: into `installed_dir` when the mod
    /// has a copy there (it must sit inside `target_mods_dir`), else into its own folder, which
    /// for Humankind sits under `Community/Atelier`.
    pub fn restore_version(
        &self,
        game: GameId,
        mod_id: &str,
        version: &str,
        target_mods_dir: &Path,
        installed_dir: Option<&Path>,
    ) -> Result<()> {
        let target = match (game, installed_dir) {
            (GameId::Humankind, None) => target_mods_dir.join(install::ATELIER_FOLDER),
            _ => target_mods_dir.to_path_buf(),
        };
        backup::restore_version(
            &self.game_backups(game),
            &target,
            mod_id,
            version,
            installed_dir,
        )
    }

    /// Every backed-up version of `game`'s `mod_id`, newest first.
    pub fn backup_versions(&self, game: GameId, mod_id: &str) -> Result<Vec<BackupVersion>> {
        backup::list_versions(&self.game_backups(game), mod_id)
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

    /// Deletes `game`'s expired backups, never a version in `kept`.
    pub fn prune_backups(&self, game: GameId, kept: &HashSet<KeptVersion>) -> Result<()> {
        let game_kept = kept
            .iter()
            .filter(|version| version.game == game)
            .map(|version| (version.mod_id.clone(), version.version.clone()))
            .collect();
        backup::prune(
            &self.game_backups(game),
            self.config.retention_days,
            &game_kept,
        )
    }

    /// One folder per game: the same ModID in both games must never share a backup.
    fn game_backups(&self, game: GameId) -> PathBuf {
        let folder = match game {
            GameId::Anno117 => "anno117",
            GameId::Anno1800 => "anno1800",
            GameId::Humankind => "humankind",
        };
        self.config.backup_dir.join(folder)
    }

    fn stage_and_classify(
        &self,
        dir: &Path,
        game: GameId,
        sources: &[PathBuf],
        installed: &[Mod],
        options: &PreviewOptions,
    ) -> Result<Vec<StagedItem>> {
        let limits = options.limits.unwrap_or(self.limits);
        let mut found = Vec::new();
        for (index, source) in sources.iter().enumerate() {
            let staged = dir.join(format!("src-{index}"));
            stage::stage_source(source, &staged, game, limits)?;
            let mut roots = roots::find_roots(&staged, game)?;
            roots.sort_by_key(roots::FoundRoot::mod_id);
            found.extend(roots.into_iter().map(|root| (staged.clone(), root)));
        }
        let mut classifier =
            Classifier::new(game, installed, options, found.iter().map(|(_, root)| root));
        Ok(found
            .iter()
            .map(|(staged, root)| {
                let item = classifier.describe(staged, root);
                StagedItem {
                    seen: classify::copy_stamp(game, installed, &item.mod_id),
                    item,
                    staged_dir: root.dir.clone(),
                    placement: classifier.placement(root),
                }
            })
            .collect())
    }

    fn lock_previews(&self) -> MutexGuard<'_, HashMap<String, StagedPreview>> {
        // A panic mid-insert can't leave the map half-updated, so a poisoned lock is still usable.
        self.previews.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
