//! Drop → preview → confirm, always into the Documents mods folder.

use std::path::{Path, PathBuf};

use atelier_core::{GameId, ModLocation, activation, scan};
use atelier_install::{InstallPreview, InstallResult};

use crate::error::{AppError, ErrorCode, Result};
use crate::state::NewModsStart;
use crate::{App, lock};

const EXPIRED_PREVIEW: &str = "That preview has expired. Drop the file again.";

impl App {
    pub fn preview_install(&self, game: GameId, paths: &[String]) -> Result<InstallPreview> {
        if paths.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "Drop a mod folder or archive to install.",
            ));
        }
        // Paths come from the webview: only full paths to ordinary files, never a bare name
        // that would resolve against the app's working folder, nor a \\.\ or \\?\ device path.
        if let Some(bad) = paths.iter().find(|path| {
            path.starts_with(r"\\.\") || path.starts_with(r"\\?\") || !Path::new(path).is_absolute()
        }) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                format!("{bad} isn't a full path to a file or folder."),
            ));
        }
        let sources: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();
        let installed = self.scan(game)?;
        let preview = self.0.engine.preview(game, &sources, &installed)?;
        lock(&self.0.previews).insert(preview.id.clone(), game);
        Ok(preview)
    }

    pub fn confirm_install(
        &self,
        preview_id: &str,
        include_downgrades: bool,
    ) -> Result<InstallResult> {
        let game = lock(&self.0.previews)
            .get(preview_id)
            .copied()
            .ok_or_else(|| AppError::new(ErrorCode::Install, EXPIRED_PREVIEW))?;
        let _writes = self.write_guard(game)?;
        let installed = self.scan(game)?;
        let result = self.0.engine.confirm(
            preview_id,
            &self.documents_mods(game),
            &installed,
            include_downgrades,
        )?;
        lock(&self.0.previews).remove(preview_id);
        // Only mods that weren't here before are "new"; an update keeps the player's on/off.
        let fresh: Vec<String> = result
            .installed
            .iter()
            .filter(|id| !installed.iter().any(|item| &item.id == *id))
            .cloned()
            .collect();
        // The new mods and their on/off rule land together, or the mods go to backups.
        if let Err(error) = self.apply_new_mods_start(game, &fresh) {
            return Err(match self.withdraw(game, &fresh) {
                Ok(()) => error,
                Err(_) => AppError::new(
                    ErrorCode::Install,
                    format!(
                        "{} The mod is installed, but its on/off setting wasn't applied.",
                        error.message
                    ),
                ),
            });
        }
        // Each install adds a backup, so expired ones go now; a failure here loses nothing.
        let _ = self.prune_backups();
        Ok(result)
    }

    pub fn cancel_install(&self, preview_id: &str) -> Result<()> {
        lock(&self.0.previews).remove(preview_id);
        Ok(self.0.engine.cancel(preview_id)?)
    }

    /// "Turned on" and "Turned off" override the active profile; "As the profile says" leaves
    /// fresh installs to its EnableNewMods line.
    pub(crate) fn apply_new_mods_start(&self, game: GameId, installed: &[String]) -> Result<()> {
        let active = match self.settings().new_mods_start {
            NewModsStart::Active => true,
            NewModsStart::Inactive => false,
            NewModsStart::Profile => return Ok(()),
        };
        if installed.is_empty() {
            return Ok(());
        }
        let writable = self.writable(game);
        for item in self.scan(game)?.iter().filter(|item| {
            installed.contains(&item.id) && item.parent_id.is_none() && item.active != active
        }) {
            activation::set_active(game, &writable, item, active, &self.0.snapshots)?;
        }
        Ok(())
    }

    /// Moves just-installed mods out of the Documents mods folder into the backups.
    fn withdraw(&self, game: GameId, mod_ids: &[String]) -> Result<()> {
        let writable = [self.writable(game)];
        for item in self.scan(game)?.iter().filter(|item| {
            item.location == ModLocation::Documents
                && item.parent_id.is_none()
                && mod_ids.contains(&item.id)
        }) {
            if let Some(dir) = scan::mod_dir(&writable, item) {
                self.0.engine.remove_to_backup(game, &dir)?;
            }
        }
        Ok(())
    }
}
