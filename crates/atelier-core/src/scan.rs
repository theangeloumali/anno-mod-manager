//! Walks the mods folders the loader reads and turns each modinfo into a [`Mod`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::io_at;
use crate::modinfo::{self, ModInfo};
use crate::{GameId, Mod, ModLocation, ModOrigin, Result, normalize_mod_id};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModsFolder {
    pub path: PathBuf,
    pub location: ModLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    MissingModinfo,
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFolder {
    pub path: PathBuf,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    pub mods: Vec<Mod>,
    pub skipped: Vec<SkippedFolder>,
}

/// Scans folders in the given order; duplicates across folders are kept for `issues`.
pub fn scan(game: GameId, folders: &[ModsFolder]) -> Result<ScanResult> {
    let mut result = ScanResult::default();
    for folder in folders {
        for dir in mod_candidates(game, &folder.path)? {
            scan_top_level(game, folder, &dir, &mut result)?;
        }
    }
    Ok(result)
}

/// Absolute directory of `target` on disk.
pub fn mod_dir(folders: &[ModsFolder], target: &Mod) -> Option<PathBuf> {
    let folder = folders.iter().find(|f| f.location == target.location)?;
    Some(
        target
            .folder
            .split('/')
            .fold(folder.path.clone(), |path, part| path.join(part)),
    )
}

fn scan_top_level(
    game: GameId,
    folder: &ModsFolder,
    dir: &Path,
    result: &mut ScanResult,
) -> Result<()> {
    let info = match modinfo::read_dir(dir) {
        Ok(Some(info)) => info,
        // Anno 1800 loads a bare data/ folder, and the installer accepts one, so list it too.
        Ok(None) if loads_bare_data_folders(game) && dir.join("data").is_dir() => inferred_info(),
        Ok(None) => {
            skip(result, dir, SkipReason::MissingModinfo);
            return Ok(());
        }
        Err(error) => {
            skip(result, dir, SkipReason::Invalid(error.to_string()));
            return Ok(());
        }
    };
    let parent_index = result.mods.len();
    let mut parent = build_mod(game, folder, &file_name(dir), dir, info)?;
    result.mods.push(parent.clone());

    for child_dir in mod_candidates(game, dir)? {
        match modinfo::read_dir(&child_dir) {
            Ok(Some(info)) => {
                let relative = format!("{}/{}", parent.folder, file_name(&child_dir));
                let mut child = build_mod(game, folder, &relative, &child_dir, info)?;
                child.parent_id = Some(parent.id.clone());
                parent.sub_mod_ids.push(child.id.clone());
                result.mods.push(child);
            }
            Ok(None) => {}
            Err(error) => skip(result, &child_dir, SkipReason::Invalid(error.to_string())),
        }
    }
    result.mods[parent_index] = parent;
    Ok(())
}

/// A manifest-less mod: build_mod names it after its folder.
fn inferred_info() -> ModInfo {
    ModInfo {
        anno: None,
        mod_id: None,
        version: String::new(),
        name: None,
        category: None,
        description: None,
        author: None,
        difficulty: None,
        setup: Default::default(),
        dependencies: Default::default(),
        dlc_dependencies: Vec::new(),
        known_issues: Vec::new(),
        options: Vec::new(),
        modio_id: None,
    }
}

fn skip(result: &mut ScanResult, dir: &Path, reason: SkipReason) {
    result.skipped.push(SkippedFolder {
        path: dir.to_path_buf(),
        reason,
    });
}

/// Child directories, sorted; 117 ignores `-`/`.` folders, 1800 shows `-` ones as disabled.
fn mod_candidates(game: GameId, dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut dirs = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let path = entry.map_err(io_at(dir))?.path();
        let name = file_name(&path);
        let hidden = name.starts_with('.') || (dash_hides_folders(game) && name.starts_with('-'));
        if path.is_dir() && !hidden {
            dirs.push(path);
        }
    }
    dirs.sort();
    Ok(dirs)
}

fn build_mod(
    game: GameId,
    folder: &ModsFolder,
    relative: &str,
    dir: &Path,
    info: ModInfo,
) -> Result<Mod> {
    let folder_name = file_name(dir);
    let origin = if folder.location == ModLocation::Ubi || info.modio_id.is_some() {
        ModOrigin::Modio
    } else {
        ModOrigin::Local
    };
    Ok(Mod {
        id: info
            .mod_id
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| normalize_mod_id(&folder_name)),
        game,
        folder: relative.to_string(),
        name: info.name.unwrap_or(folder_name),
        category: info.category.unwrap_or_default(),
        version: info.version,
        author: info.author,
        description: info.description,
        difficulty: info.difficulty,
        location: folder.location,
        origin,
        modio_id: info.modio_id,
        active: true,
        size_bytes: dir_size(dir)?,
        setup: info.setup,
        dependencies: info.dependencies,
        known_issues: info.known_issues,
        has_options: !info.options.is_empty(),
        parent_id: None,
        sub_mod_ids: Vec::new(),
        update: None,
    })
}

fn dir_size(dir: &Path) -> Result<u64> {
    let mut total = 0;
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let entry = entry.map_err(io_at(dir))?;
        let metadata = entry.metadata().map_err(io_at(&entry.path()))?;
        total += if metadata.is_dir() {
            dir_size(&entry.path())?
        } else {
            metadata.len()
        };
    }
    Ok(total)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn loads_bare_data_folders(game: GameId) -> bool {
    match game {
        GameId::Anno1800 => true,
        GameId::Anno117 | GameId::Humankind => false,
    }
}

/// 117's loader skips `-Name` folders; 1800 treats the prefix as "disabled" instead.
fn dash_hides_folders(game: GameId) -> bool {
    match game {
        GameId::Anno117 => true,
        GameId::Anno1800 | GameId::Humankind => false,
    }
}
