use std::io;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error(transparent)]
    Core(#[from] atelier_core::Error),
    #[error("{}", unsupported_message(.0))]
    Unsupported(String),
    #[error("{archive} can't be opened: {reason}")]
    CorruptArchive { archive: String, reason: String },
    #[error("{archive} has an unsafe entry '{entry}', so nothing from it is installed")]
    UnsafePath { archive: String, entry: String },
    #[error("{archive} unpacks to more than {limit} bytes, so it is refused")]
    TooLarge { archive: String, limit: u64 },
    #[error("{archive} holds more than {limit} files, so it is refused")]
    TooManyEntries { archive: String, limit: usize },
    #[error("{0} is empty")]
    EmptySource(String),
    #[error("{0} can't be read as a file or folder")]
    MissingSource(PathBuf),
    #[error("install preview '{0}' no longer exists; drop the files again")]
    PreviewNotFound(String),
    #[error("there is no backup of '{0}' to roll back to")]
    NoBackup(String),
    #[error("{path} changed while it was copied; the original is kept")]
    CopyMismatch { path: PathBuf },
    #[error("{cause}; these paths could not be restored and need a manual check: {paths:?}")]
    RestoreIncomplete { cause: String, paths: Vec<PathBuf> },
}

// RAR is refused until the UnRAR licence is cleared (Angelo's call), not for lack of code.
fn unsupported_message(kind: &str) -> String {
    if kind == "rar" {
        "RAR archives aren't supported yet: extract it and drop the folder instead".into()
    } else {
        format!("'{kind}' files can't be installed: drop a .zip, a .7z, a folder or a modinfo.json")
    }
}

/// Attaches the path to an I/O error, which `io::Error` alone never says.
pub(crate) fn io_at(path: &Path) -> impl FnOnce(io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}
