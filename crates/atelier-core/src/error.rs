use std::io;
use std::path::{Path, PathBuf};

use crate::ActivationScheme;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid modinfo: {0}")]
    InvalidModInfo(String),
    #[error("{0} is read-only; only the Documents mods folder is writable")]
    ReadOnly(PathBuf),
    #[error("profile '{0}' not found")]
    ProfileNotFound(String),
    #[error("profile '{0}' is built in")]
    BuiltInProfile(String),
    #[error("profile '{0}' is active; switch away from it first")]
    ProfileActive(String),
    #[error("invalid profile name '{0}'")]
    InvalidProfileName(String),
    #[error("the {0:?} activation scheme has no profiles")]
    NoProfiles(ActivationScheme),
    #[error("sub-mod '{0}' is switched with its parent")]
    SubMod(String),
    #[error("no Users/<id>/Registry.xml under {0}; start Humankind once to create it")]
    RegistryNotFound(PathBuf),
    #[error("{0} is not a Registry.xml Atelier can edit (not UTF-8, or no </Registry> end tag)")]
    InvalidRegistry(PathBuf),
    #[error("'{0}' is the base game and stays on")]
    BaseModule(String),
    #[error(transparent)]
    LoadOrder(#[from] crate::humankind::load_order::OrderError),
    /// Humankind resets the whole list to vanilla when a listed module is gone.
    #[error(
        "not installed: {}; Humankind would reset the mod list to vanilla",
        .0.join(", ")
    )]
    MissingModules(Vec<String>),
    #[error("'{0}' is a map; Humankind lists every installed map, so it has no on/off switch")]
    MapAlwaysOn(String),
    #[error("{0} is too large to be a file the game wrote, so Atelier won't read it")]
    TooLarge(PathBuf),
}

/// Attaches the path to an I/O error, which `io::Error` alone never says.
pub(crate) fn io_at(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}
