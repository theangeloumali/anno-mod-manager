//! Humankind saves: what each was played with, and putting those mod versions back.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use atelier_core::humankind::saves::{self, ModCheck, ModStatus, Save, SaveList, SaveMod};
use atelier_core::humankind::{detect, order};
use atelier_core::{GameId, Mod, ModLocation, games, scan};
use atelier_install::KeptVersion;
use serde::Serialize;

use crate::App;
use crate::error::{AppError, ErrorCode, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInspection {
    #[serde(flatten)]
    pub save: Save,
    /// One per save mod, in load order.
    pub checks: Vec<ModCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveLoadReport {
    /// Put back from the backups at the version the save was played with.
    pub restored: Vec<SaveMod>,
    /// Still not at the save's version: not installed, or no backup of that version.
    pub missing: Vec<ModCheck>,
    pub registry_written: bool,
}

impl App {
    pub fn list_saves(&self, game: GameId) -> Result<SaveList> {
        Ok(saves::list_saves(&humankind_dir(game, &self.0.documents)?)?)
    }

    pub fn inspect_save(&self, game: GameId, path: &Path) -> Result<SaveInspection> {
        let save = self.read_save(game, path)?;
        let checks = saves::check_mods(&save.descriptor.mods, &self.scan(game)?);
        Ok(SaveInspection { save, checks })
    }

    /// The load order is written only when every mod matches, so the game is never pointed at
    /// a save whose mods it can't load.
    pub fn make_save_loadable(&self, game: GameId, path: &Path) -> Result<SaveLoadReport> {
        let _writes = self.write_guard(game)?;
        let save = self.read_save(game, path)?;
        let game_dir = humankind_dir(game, &self.0.documents)?;
        if !save.descriptor.mod_list_found {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "This save's mod list couldn't be read, so Atelier won't change your mod list for it.",
            ));
        }
        // Checked before anything is restored, so a game that never ran is left untouched.
        if detect::registry_path(&game_dir)?.is_none() {
            return Err(atelier_core::Error::RegistryNotFound(game_dir).into());
        }
        let installed = self.scan(game)?;
        let checks = saves::check_mods(&save.descriptor.mods, &installed);
        if let Some(copy) = checks
            .iter()
            .find_map(|check| read_only_copy(check, &installed))
        {
            return Err(AppError::new(
                ErrorCode::ReadOnly,
                format!(
                    "{} is in the {}, which Atelier can't change, so the version this save was \
                     played with wasn't put back. Remove that copy first.",
                    copy.name,
                    copy.location.place()
                ),
            ));
        }
        let mut restored = Vec::new();
        for check in checks {
            if self.restore_played_version(game, &check, &installed)? {
                restored.push(check.module);
            }
        }
        let installed = self.scan(game)?;
        let missing: Vec<ModCheck> = saves::check_mods(&save.descriptor.mods, &installed)
            .into_iter()
            .filter(|check| check.status != ModStatus::Exact)
            .collect();
        let registry_written = missing.is_empty();
        if registry_written {
            let modules: Vec<String> = save
                .descriptor
                .mods
                .iter()
                .map(|module| module.name.clone())
                .collect();
            order::write(&game_dir, &modules, &installed, &self.0.snapshots)?;
        }
        Ok(SaveLoadReport {
            restored,
            missing,
            registry_written,
        })
    }

    /// None unless every save was read: pruning without the full set could delete a version
    /// a save still needs.
    pub(crate) fn versions_saves_need(&self) -> Option<HashSet<KeptVersion>> {
        let game = GameId::Humankind;
        let listing =
            saves::list_saves(&games::documents_game_dir(game, &self.0.documents)).ok()?;
        if !listing.skipped.is_empty() {
            return None;
        }
        Some(
            listing
                .saves
                .into_iter()
                .flat_map(|save| save.descriptor.mods)
                .filter_map(|module| {
                    Some(KeptVersion {
                        game,
                        version: module.version?,
                        mod_id: module.name,
                    })
                })
                .collect(),
        )
    }

    /// The path comes from the UI, so it must resolve inside Save Files. A relative or network
    /// path is refused unresolved: resolving it would touch the cwd or a remote share.
    fn read_save(&self, game: GameId, path: &Path) -> Result<Save> {
        let save_files = humankind_dir(game, &self.0.documents)?.join(saves::SAVE_FILES_DIR);
        let refused = || {
            AppError::new(
                ErrorCode::InvalidInput,
                "Choose a save from Humankind's Save Files folder.",
            )
        };
        let text = path.to_string_lossy();
        if !path.is_absolute() || text.starts_with("\\\\") || text.starts_with("//") {
            return Err(refused());
        }
        let file = match (save_files.canonicalize(), path.canonicalize()) {
            (Ok(root), Ok(file)) if file.starts_with(&root) && file.is_file() => file,
            _ => return Err(refused()),
        };
        saves::read_save(&file)
            .map_err(|error| AppError::new(ErrorCode::InvalidInput, format!("This save {error}.")))
    }

    /// Replaces the Documents copy in place, else adds the version in its own folder; a
    /// version with no backup is left for the report.
    fn restore_played_version(
        &self,
        game: GameId,
        check: &ModCheck,
        installed: &[Mod],
    ) -> Result<bool> {
        let Some(version) = check.module.version.as_deref() else {
            return Ok(false);
        };
        if check.status == ModStatus::Exact {
            return Ok(false);
        }
        // A backup of a same-named mod from another author is not the one the save used.
        if let Some(wanted) = check.module.guid.as_deref() {
            let stored = self
                .0
                .engine
                .backup_guid(game, &check.module.name, version)?;
            if stored.is_some_and(|stored| !stored.eq_ignore_ascii_case(wanted)) {
                return Ok(false);
            }
        }
        let writable = [self.writable(game)];
        // Swapping a folder that holds another module too would take that module with it.
        let shares_folder = installed.iter().any(|item| {
            item.id == check.module.name
                && installed.iter().any(|other| {
                    other.id != item.id
                        && other.location == item.location
                        && other.folder == item.folder
                })
        });
        if shares_folder {
            return Ok(false);
        }
        let copy = installed
            .iter()
            .filter(|item| item.id == check.module.name)
            .find_map(|item| scan::mod_dir(&writable, item));
        match self.0.engine.restore_version(
            game,
            &check.module.name,
            version,
            &writable[0].path,
            copy.as_deref(),
        ) {
            Ok(()) => {
                self.relink_after_restore(game, &check.module.name)?;
                Ok(true)
            }
            Err(atelier_install::Error::NoBackup(_)) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

/// A Workshop or game-folder copy of a mod the save needs at another version: a restored copy
/// would load beside it.
fn read_only_copy<'a>(check: &ModCheck, installed: &'a [Mod]) -> Option<&'a Mod> {
    if check.status == ModStatus::Exact || check.module.version.is_none() {
        return None;
    }
    installed
        .iter()
        .find(|item| item.id == check.module.name && item.location != ModLocation::Documents)
}

fn humankind_dir(game: GameId, documents: &Path) -> Result<PathBuf> {
    match game {
        GameId::Humankind => Ok(games::documents_game_dir(game, documents)),
        GameId::Anno117 | GameId::Anno1800 => Err(AppError::new(
            ErrorCode::NotSupported,
            format!("{} has no save inspector.", game.display_name()),
        )),
    }
}
