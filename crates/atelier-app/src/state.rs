//! What survives a restart: settings, chosen game folders and which mods came from mod.io.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use atelier_core::GameId;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result};

const STATE_FILE: &str = "state.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NewModsStart {
    Active,
    Inactive,
    Profile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateCheck {
    Hourly,
    Startup,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub new_mods_start: NewModsStart,
    pub install_dependencies: bool,
    /// None keeps replaced versions forever.
    pub backup_retention_days: Option<u32>,
    pub update_check: UpdateCheck,
    pub language: String,
    pub update_channel: UpdateChannel,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_mods_start: NewModsStart::Active,
            install_dependencies: true,
            backup_retention_days: Some(30),
            update_check: UpdateCheck::Startup,
            language: "en".to_owned(),
            update_channel: UpdateChannel::Stable,
        }
    }
}

/// The mod.io file a local mod was installed from, so updates can be checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ModioLink {
    pub modio_id: u64,
    pub modfile_id: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Persisted {
    pub settings: Settings,
    pub game_paths: HashMap<GameId, PathBuf>,
    pub modio_links: HashMap<GameId, HashMap<String, ModioLink>>,
    /// Links of uninstalled mods, so restoring one from its backup keeps its updates.
    pub uninstalled_links: HashMap<GameId, HashMap<String, ModioLink>>,
    pub terms_accepted: bool,
}

impl Persisted {
    /// A missing file is a first run; a corrupt one is reported rather than silently reset.
    pub fn load(data_dir: &Path) -> Result<Self> {
        let path = data_dir.join(STATE_FILE);
        match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                AppError::new(
                    crate::ErrorCode::Io,
                    format!("{} is damaged: {error}", path.display()),
                )
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(AppError::io(&error, &path)),
        }
    }

    /// Write-then-rename so a crash mid-save never leaves half a file.
    pub fn save(&self, data_dir: &Path) -> Result<()> {
        fs::create_dir_all(data_dir).map_err(|error| AppError::io(&error, data_dir))?;
        let path = data_dir.join(STATE_FILE);
        let temp = data_dir.join(format!("{STATE_FILE}.tmp"));
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| AppError::new(crate::ErrorCode::Io, error.to_string()))?;
        fs::write(&temp, bytes).map_err(|error| AppError::io(&error, &temp))?;
        fs::rename(&temp, &path).map_err(|error| AppError::io(&error, &path))
    }
}
