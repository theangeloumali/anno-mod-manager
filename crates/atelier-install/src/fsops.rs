//! Directory moves that survive a cross-device rename, and tree copies that refuse symlinks.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::error::io_at;
use crate::{Error, Result};

/// Renames `from` to `to` (which must not exist); across volumes it copies, verifies, then deletes.
pub(crate) fn move_dir(from: &Path, to: &Path) -> Result<()> {
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::CrossesDevices => copy_verify_delete(from, to),
        Err(error) => Err(io_at(from)(error)),
    }
}

fn copy_verify_delete(from: &Path, to: &Path) -> Result<()> {
    let copied = copy_tree(from, to, &|_| false).and_then(|()| {
        if tree_signature(from)? == tree_signature(to)? {
            Ok(())
        } else {
            Err(Error::CopyMismatch {
                path: from.to_path_buf(),
            })
        }
    });
    if let Err(error) = copied {
        // The source is untouched, so dropping the partial copy loses nothing.
        let _ = fs::remove_dir_all(to);
        return Err(error);
    }
    fs::remove_dir_all(from).map_err(io_at(from))
}

/// Copies a tree, leaving out what `skip` (given the relative path) rejects. Symlinks are refused.
pub(crate) fn copy_tree(from: &Path, to: &Path, skip: &dyn Fn(&Path) -> bool) -> Result<()> {
    copy_level(from, from, to, skip)
}

fn copy_level(base: &Path, from: &Path, to: &Path, skip: &dyn Fn(&Path) -> bool) -> Result<()> {
    fs::create_dir_all(to).map_err(io_at(to))?;
    for entry in fs::read_dir(from).map_err(io_at(from))? {
        let source = entry.map_err(io_at(from))?.path();
        let relative = source.strip_prefix(base).unwrap_or(&source);
        if skip(relative) {
            continue;
        }
        let kind = fs::symlink_metadata(&source)
            .map_err(io_at(&source))?
            .file_type();
        let target = to.join(source.file_name().unwrap_or_default());
        if kind.is_symlink() {
            return Err(Error::UnsafePath {
                archive: file_name(base),
                entry: relative.to_string_lossy().replace('\\', "/"),
            });
        } else if kind.is_dir() {
            copy_level(base, &source, &target, skip)?;
        } else {
            fs::copy(&source, &target).map_err(io_at(&source))?;
        }
    }
    Ok(())
}

/// Relative path and size of every file: enough to prove a copy is complete.
fn tree_signature(dir: &Path) -> Result<Vec<(PathBuf, u64)>> {
    let mut files = Vec::new();
    walk_files(dir, &mut |path| {
        let size = fs::metadata(path).map_err(io_at(path))?.len();
        files.push((path.strip_prefix(dir).unwrap_or(path).to_path_buf(), size));
        Ok(())
    })?;
    files.sort();
    Ok(files)
}

/// Calls `visit` for every file under `dir`, depth first.
pub(crate) fn walk_files(dir: &Path, visit: &mut dyn FnMut(&Path) -> Result<()>) -> Result<()> {
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let path = entry.map_err(io_at(dir))?.path();
        if path.is_dir() {
            walk_files(&path, visit)?;
        } else {
            visit(&path)?;
        }
    }
    Ok(())
}

/// Child directories of `dir`, sorted.
pub(crate) fn child_dirs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let path = entry.map_err(io_at(dir))?.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cross_device_fallback_copies_verifies_and_deletes_the_source() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from");
        fs::create_dir_all(from.join("data")).unwrap();
        fs::write(from.join("modinfo.json"), "{}").unwrap();
        fs::write(from.join("data/a.xml"), "<a/>").unwrap();
        let to = tmp.path().join("to");

        copy_verify_delete(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(fs::read_to_string(to.join("data/a.xml")).unwrap(), "<a/>");
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_fallback_copy_keeps_the_source_and_drops_the_partial_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let from = tmp.path().join("from");
        fs::create_dir_all(&from).unwrap();
        fs::write(from.join("modinfo.json"), "{}").unwrap();
        std::os::unix::fs::symlink("/etc/passwd", from.join("link")).unwrap();
        let to = tmp.path().join("to");

        assert!(copy_verify_delete(&from, &to).is_err());

        assert!(from.join("modinfo.json").is_file());
        assert!(!to.exists());
    }
}
