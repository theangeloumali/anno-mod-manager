//! Unpacks zip and 7z archives into staging, refusing unsafe paths and bombs before writing.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use sevenz_rust2::{ArchiveEntry, ArchiveReader, EntryDecision, Password};
use zip::ZipArchive;

use crate::error::io_at;
use crate::{Error, Limits, Result};

const MAC_METADATA_DIR: &str = "__MACOSX";
const MAC_FOLDER_FILE: &str = ".DS_Store";
// 7z keeps unix modes in the high 16 bits when this attribute bit is set.
const UNIX_EXTENSION_BIT: u32 = 0x8000;
const REPARSE_POINT_BIT: u32 = 0x400;
const UNIX_FILE_TYPE_MASK: u32 = 0o170_000;
const UNIX_SYMLINK: u32 = 0o120_000;

/// Running totals for one dropped source, nested archives included.
pub(crate) struct Budget {
    pub archive: String,
    limits: Limits,
    bytes: u64,
    entries: usize,
}

impl Budget {
    pub fn new(archive: String, limits: Limits) -> Self {
        Self {
            archive,
            limits,
            bytes: 0,
            entries: 0,
        }
    }

    pub(crate) fn admit(&mut self, size: u64) -> Result<()> {
        self.entries += 1;
        if self.entries > self.limits.max_entries {
            return Err(Error::TooManyEntries {
                archive: self.archive.clone(),
                limit: self.limits.max_entries,
            });
        }
        self.bytes = self.bytes.saturating_add(size);
        if self.bytes > self.limits.max_total_bytes {
            return Err(self.too_large());
        }
        Ok(())
    }

    fn too_large(&self) -> Error {
        Error::TooLarge {
            archive: self.archive.clone(),
            limit: self.limits.max_total_bytes,
        }
    }

    fn unsafe_entry(&self, entry: &str) -> Error {
        Error::UnsafePath {
            archive: self.archive.clone(),
            entry: entry.to_string(),
        }
    }

    fn corrupt(&self, reason: impl ToString) -> Error {
        Error::CorruptArchive {
            archive: self.archive.clone(),
            reason: reason.to_string(),
        }
    }
}

/// `__MACOSX/` trees and `.DS_Store` files are Finder noise, never mod content.
pub(crate) fn is_mac_metadata(relative: &Path) -> bool {
    relative
        .components()
        .any(|part| part.as_os_str() == MAC_METADATA_DIR)
        || relative
            .file_name()
            .is_some_and(|name| name == MAC_FOLDER_FILE)
}

/// The entry as a relative path, or `None` when it could escape the staging folder.
fn safe_relative(raw: &str) -> Option<PathBuf> {
    let normalized = raw.replace('\\', "/");
    if normalized.starts_with('/') || normalized.contains('\0') {
        return None;
    }
    let mut relative = PathBuf::new();
    for part in normalized.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            // A colon is a drive letter or an NTFS stream on Windows.
            name if name.contains(':') || !crate::fsops::is_windows_safe_name(name) => return None,
            name => relative.push(name),
        }
    }
    relative
        .components()
        .all(|part| matches!(part, Component::Normal(_)))
        .then_some(relative)
}

pub(crate) fn extract_zip(archive_path: &Path, dest: &Path, budget: &mut Budget) -> Result<()> {
    let file = File::open(archive_path).map_err(io_at(archive_path))?;
    let mut archive = ZipArchive::new(file).map_err(|error| budget.corrupt(error))?;
    // Every entry is vetted before the first byte is written.
    let mut planned = Vec::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| budget.corrupt(error))?;
        let name = entry
            .name()
            .map_err(|error| budget.corrupt(error))?
            .into_owned();
        let relative = safe_relative(&name).ok_or_else(|| budget.unsafe_entry(&name))?;
        if entry.is_symlink() {
            return Err(budget.unsafe_entry(&name));
        }
        if relative.as_os_str().is_empty() || is_mac_metadata(&relative) {
            continue;
        }
        budget.admit(entry.size())?;
        planned.push((index, relative, entry.is_dir(), entry.size()));
    }
    for (index, relative, is_dir, size) in planned {
        let target = dest.join(relative);
        if is_dir {
            fs::create_dir_all(&target).map_err(io_at(&target))?;
            continue;
        }
        let mut entry = archive
            .by_index(index)
            .map_err(|error| budget.corrupt(error))?;
        write_capped(&mut entry, &target, size, budget)?;
    }
    Ok(())
}

pub(crate) fn extract_7z(archive_path: &Path, dest: &Path, budget: &mut Budget) -> Result<()> {
    let mut reader = ArchiveReader::open(archive_path, Password::empty())
        .map_err(|error| budget.corrupt(error))?;
    let mut planned = HashMap::new();
    for entry in &reader.archive().files {
        let name = entry.name();
        let relative = safe_relative(name).ok_or_else(|| budget.unsafe_entry(name))?;
        if is_7z_symlink(entry) {
            return Err(budget.unsafe_entry(name));
        }
        if relative.as_os_str().is_empty() || is_mac_metadata(&relative) {
            continue;
        }
        budget.admit(entry.size())?;
        planned.insert(name.to_string(), relative);
    }
    let mut failure = None;
    let decoded = reader.for_each_entries_with_decision(|entry, data| {
        let Some(relative) = planned.get(entry.name()) else {
            return Ok(EntryDecision::Continue);
        };
        let target = dest.join(relative);
        let written = if entry.is_directory() {
            fs::create_dir_all(&target).map_err(io_at(&target))
        } else {
            write_capped(data, &target, entry.size(), budget)
        };
        Ok(match written {
            Ok(()) => EntryDecision::Continue,
            Err(error) => {
                failure = Some(error);
                EntryDecision::Stop
            }
        })
    });
    if let Some(error) = failure {
        return Err(error);
    }
    decoded.map_err(|error| budget.corrupt(error))
}

fn is_7z_symlink(entry: &ArchiveEntry) -> bool {
    let attributes = entry.windows_attributes();
    let unix_symlink = attributes & UNIX_EXTENSION_BIT != 0
        && (attributes >> 16) & UNIX_FILE_TYPE_MASK == UNIX_SYMLINK;
    unix_symlink || attributes & REPARSE_POINT_BIT != 0
}

/// Writes at most the declared size, so a header that lies about it can't fill the disk.
fn write_capped(data: &mut dyn Read, target: &Path, declared: u64, budget: &Budget) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(io_at(parent))?;
    }
    let mut file = File::create(target).map_err(io_at(target))?;
    let copied =
        io::copy(&mut data.take(declared.saturating_add(1)), &mut file).map_err(|error| {
            if error.kind() == io::ErrorKind::InvalidData {
                budget.corrupt(error)
            } else {
                io_at(target)(error)
            }
        })?;
    if copied > declared {
        return Err(budget.too_large());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_relative_accepts_plain_and_rejects_escaping_paths() {
        assert_eq!(safe_relative("a/./b\\c"), Some(PathBuf::from("a/b/c")));
        for raw in [
            "../x",
            "a/../../x",
            "/x",
            "\\x",
            "C:/x",
            "a/b:stream",
            "a\0b",
            // Windows trims trailing dots and spaces and maps device names to devices.
            "a/b.",
            "a/b ",
            "CON",
            "x/nul.txt",
            "x/Com1.dds",
            "a<b",
            "a|b",
        ] {
            assert_eq!(safe_relative(raw), None, "{raw}");
        }
        assert!(safe_relative("console/conan.txt").is_some());
    }
}
