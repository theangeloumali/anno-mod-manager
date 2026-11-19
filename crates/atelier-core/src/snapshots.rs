//! Copies a file aside before every write so a bad toggle can be rolled back.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Result;
use crate::error::io_at;
use crate::scan::ModsFolder;
use crate::{Error, ModLocation};

#[derive(Debug, Clone)]
pub struct Snapshots {
    dir: PathBuf,
    keep: usize,
}

impl Snapshots {
    pub fn new(dir: impl Into<PathBuf>, keep: usize) -> Self {
        Self {
            dir: dir.into(),
            keep: keep.max(1),
        }
    }

    /// Copies `file` to `<dir>/<name>.<nanos>.bak` and prunes older copies; `None` if absent.
    pub fn take(&self, file: &Path) -> Result<Option<PathBuf>> {
        if !file.is_file() {
            return Ok(None);
        }
        fs::create_dir_all(&self.dir).map_err(io_at(&self.dir))?;
        let name = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        // Zero-padded so lexical order is age order; bumped when two writes share a tick.
        let target = loop {
            let candidate = self.dir.join(format!("{name}.{stamp:020}.bak"));
            if !candidate.exists() {
                break candidate;
            }
            stamp += 1;
        };
        fs::copy(file, &target).map_err(io_at(file))?;
        self.prune(&name)?;
        Ok(Some(target))
    }

    fn prune(&self, name: &str) -> Result<()> {
        let prefix = format!("{name}.");
        let mut copies: Vec<PathBuf> = fs::read_dir(&self.dir)
            .map_err(io_at(&self.dir))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                file_name.starts_with(&prefix) && file_name.ends_with(".bak")
            })
            .collect();
        copies.sort();
        let excess = copies.len().saturating_sub(self.keep);
        for old in &copies[..excess] {
            fs::remove_file(old).map_err(io_at(old))?;
        }
        Ok(())
    }
}

/// Only the Documents folder is ours; game and `.ubi` folders belong to the launcher.
pub(crate) fn ensure_writable(folder: &ModsFolder) -> Result<()> {
    if folder.location == ModLocation::Documents {
        Ok(())
    } else {
        Err(Error::ReadOnly(folder.path.clone()))
    }
}

/// Snapshots then writes, creating the parent folder on first use.
pub(crate) fn write_with_snapshot(
    path: &Path,
    contents: &str,
    snapshots: &Snapshots,
) -> Result<()> {
    snapshots.take(path)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    fs::write(path, contents).map_err(io_at(path))
}
