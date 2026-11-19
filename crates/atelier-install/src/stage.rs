//! Copies or unpacks each dropped path into its own staging folder.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::io_at;
use crate::extract::{Budget, extract_7z, extract_zip, is_mac_metadata};
use crate::fsops::{child_dirs, copy_tree, file_name, walk_files};
use crate::roots::{has_data_folder, has_modinfo};
use crate::{Error, Limits, Result};

const MODINFO_NAMES: [&str; 2] = ["modinfo.json", "modinfo.jsonc"];

enum SourceKind {
    Folder(PathBuf),
    Zip,
    SevenZip,
}

/// Stages `source` into `dest` (created here). Every file under `dest` came from the drop.
pub(crate) fn stage_source(source: &Path, dest: &Path, limits: Limits) -> Result<()> {
    let name = file_name(source);
    let mut budget = Budget::new(name.clone(), limits);
    let kind = classify(source)?;
    fs::create_dir_all(dest).map_err(io_at(dest))?;
    match kind {
        SourceKind::Folder(folder) => {
            copy_tree(&folder, &dest.join(file_name(&folder)), &is_mac_metadata)?
        }
        SourceKind::Zip => extract_zip(source, dest, &mut budget)?,
        SourceKind::SevenZip => extract_7z(source, dest, &mut budget)?,
    }
    if !has_modinfo_anywhere(dest)? {
        unpack_nested_archives(dest, &mut budget)?;
    }
    wrap_loose_root(dest, &archive_stem(source))?;
    if count_files(dest)? == 0 {
        return Err(Error::EmptySource(name));
    }
    Ok(())
}

fn classify(source: &Path) -> Result<SourceKind> {
    if source.is_dir() {
        return Ok(SourceKind::Folder(source.to_path_buf()));
    }
    let name = file_name(source).to_lowercase();
    let extension = Path::new(&name)
        .extension()
        .map(|ext| ext.to_string_lossy().into_owned())
        .unwrap_or_default();
    if extension == "rar" {
        return Err(Error::Unsupported("rar".into()));
    }
    if !source.is_file() {
        return Err(Error::MissingSource(source.to_path_buf()));
    }
    // A dropped modinfo.json stands for the mod folder that holds it.
    if MODINFO_NAMES.contains(&name.as_str()) {
        let parent = source
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| Error::MissingSource(source.to_path_buf()))?;
        return Ok(SourceKind::Folder(parent.to_path_buf()));
    }
    match extension.as_str() {
        "zip" => Ok(SourceKind::Zip),
        "7z" => Ok(SourceKind::SevenZip),
        other => Err(Error::Unsupported(other.to_string())),
    }
}

/// Mod authors often zip a mod inside a zip; unpack those one level, then stop.
fn unpack_nested_archives(dest: &Path, budget: &mut Budget) -> Result<()> {
    let mut archives = Vec::new();
    walk_files(dest, &mut |path| {
        let extension = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase());
        if matches!(extension.as_deref(), Some("zip" | "7z")) {
            archives.push(path.to_path_buf());
        }
        Ok(())
    })?;
    for archive in archives {
        let target = archive.with_file_name(archive_stem(&archive));
        if archive
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        {
            extract_zip(&archive, &target, budget)?;
        } else {
            extract_7z(&archive, &target, budget)?;
        }
        fs::remove_file(&archive).map_err(io_at(&archive))?;
    }
    Ok(())
}

/// An archive whose modinfo or `data/` sits at its top level gets a folder named after the archive,
/// so every root has a folder name to install under.
fn wrap_loose_root(dest: &Path, stem: &str) -> Result<()> {
    if !has_modinfo(dest) && !has_data_folder(dest) {
        return Ok(());
    }
    let wrapper = dest.join(format!(".{stem}.wrapping"));
    fs::create_dir(&wrapper).map_err(io_at(&wrapper))?;
    for entry in fs::read_dir(dest).map_err(io_at(dest))? {
        let path = entry.map_err(io_at(dest))?.path();
        if path != wrapper {
            let moved = wrapper.join(file_name(&path));
            fs::rename(&path, &moved).map_err(io_at(&path))?;
        }
    }
    let final_dir = dest.join(stem);
    fs::rename(&wrapper, &final_dir).map_err(io_at(&wrapper))
}

fn has_modinfo_anywhere(dir: &Path) -> Result<bool> {
    if has_modinfo(dir) {
        return Ok(true);
    }
    for child in child_dirs(dir)? {
        if has_modinfo_anywhere(&child)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn count_files(dir: &Path) -> Result<usize> {
    let mut count = 0;
    walk_files(dir, &mut |_| {
        count += 1;
        Ok(())
    })?;
    Ok(count)
}

/// A folder name from the archive's stem; a stem like ".." or "a/b" would escape, so it becomes "mod".
fn archive_stem(path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Windows drops trailing dots and spaces, so "..." would also resolve to "..".
    let trimmed = stem.trim_end_matches(['.', ' ']);
    if trimmed.is_empty() || trimmed.contains(['/', '\\', ':']) {
        "mod".into()
    } else {
        trimmed.into()
    }
}
