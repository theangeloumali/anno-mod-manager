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
    #[error("the {0:?} activation scheme is not wired up yet")]
    Unsupported(ActivationScheme),
}

/// Attaches the path to an I/O error, which `io::Error` alone never says.
pub(crate) fn io_at(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}
