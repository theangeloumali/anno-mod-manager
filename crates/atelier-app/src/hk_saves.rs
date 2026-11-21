//! Humankind saves: what each was played with, and putting those mod versions back.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use atelier_core::humankind::saves::{self, ModCheck, ModStatus, Save, SaveList, SaveMod};
use atelier_core::humankind::{detect, order};
use atelier_core::{GameId, Mod, games, scan};
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
        // Checked before anything is restored, so a game that never ran is left untouched.
        if detect::registry_path(&game_dir)?.is_none() {
            return Err(atelier_core::Error::RegistryNotFound(game_dir).into());
        }
        let installed = self.scan(game)?;
        let mut restored = Vec::new();
        for check in saves::check_mods(&save.descriptor.mods, &installed) {
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

    /// Fails rather than returning less: pruning without the full set could delete a version
    /// a save still needs.
    pub(crate) fn versions_saves_need(&self) -> Result<HashSet<KeptVersion>> {
        let game = GameId::Humankind;
        let listing = saves::list_saves(&games::documents_game_dir(game, &self.0.documents))?;
        Ok(listing
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
            .collect())
    }

    /// The path comes from the UI, so it must resolve inside Save Files.
    fn read_save(&self, game: GameId, path: &Path) -> Result<Save> {
        let save_files = humankind_dir(game, &self.0.documents)?.join(saves::SAVE_FILES_DIR);
        let file = match (save_files.canonicalize(), path.canonicalize()) {
            (Ok(root), Ok(file)) if file.starts_with(&root) && file.is_file() => file,
            _ => {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "Choose a save from Humankind's Save Files folder.",
                ));
            }
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
        let writable = [self.writable(game)];
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

fn humankind_dir(game: GameId, documents: &Path) -> Result<PathBuf> {
    match game {
        GameId::Humankind => Ok(games::documents_game_dir(game, documents)),
        GameId::Anno117 | GameId::Anno1800 => Err(AppError::new(
            ErrorCode::NotSupported,
            format!("{} has no save inspector.", game.display_name()),
        )),
    }
}
