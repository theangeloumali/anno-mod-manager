//! Drop → preview → confirm, always into the Documents mods folder.

use std::path::{Path, PathBuf};

use atelier_core::{GameId, activation};
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
        self.guard_writes(game)?;
        let installed = self.scan(game)?;
        let result = self.0.engine.confirm(
            preview_id,
            &self.documents_mods(game),
            &installed,
            include_downgrades,
        )?;
        lock(&self.0.previews).remove(preview_id);
        self.apply_new_mods_start(game, &result.installed)?;
        Ok(result)
    }

    pub fn cancel_install(&self, preview_id: &str) -> Result<()> {
        lock(&self.0.previews).remove(preview_id);
        Ok(self.0.engine.cancel(preview_id)?)
    }

    /// Fresh installs are active by default; "inactive" switches them off right after install.
    pub(crate) fn apply_new_mods_start(&self, game: GameId, installed: &[String]) -> Result<()> {
        if self.settings().new_mods_start != NewModsStart::Inactive || installed.is_empty() {
            return Ok(());
        }
        let writable = self.writable(game);
        for item in self
            .scan(game)?
            .iter()
            .filter(|item| installed.contains(&item.id) && item.parent_id.is_none())
        {
            activation::set_active(game, &writable, item, false, &self.0.snapshots)?;
        }
        Ok(())
    }
}
