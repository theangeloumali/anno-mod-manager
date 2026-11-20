//! Finds mod roots in a staged tree and tells which game a root was built for.

use std::path::{Path, PathBuf};

use atelier_core::{GameId, ModInfo, modinfo, normalize_mod_id};

use crate::Result;
use crate::fsops::{child_dirs, file_name};

const ANNO_117_MARKER: u8 = 8;
const ANNO_1800_MARKER: u8 = 7;

pub(crate) enum Manifest {
    Parsed(Box<ModInfo>),
    Broken(String),
    /// No modinfo, but a `data/` folder: how Anno 1800 mods used to ship.
    Inferred,
    /// Neither modinfo nor `data/`.
    Unrecognised,
}

pub(crate) struct FoundRoot {
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub sub_mod_ids: Vec<String>,
}

impl FoundRoot {
    /// The ModID, or the normalised folder name when modinfo has none.
    pub fn mod_id(&self) -> String {
        let declared = match &self.manifest {
            Manifest::Parsed(info) => info.mod_id.clone(),
            _ => None,
        };
        declared.unwrap_or_else(|| normalize_mod_id(&file_name(&self.dir)))
    }
}

pub(crate) fn has_modinfo(dir: &Path) -> bool {
    ["modinfo.json", "modinfo.jsonc"]
        .iter()
        .any(|name| dir.join(name).is_file())
}

pub(crate) fn has_data_folder(dir: &Path) -> bool {
    dir.join("data").is_dir()
}

/// Folders with a modinfo and none above them; without any, the topmost `data/` holders.
pub(crate) fn find_roots(staged: &Path) -> Result<Vec<FoundRoot>> {
    let mut dirs = Vec::new();
    collect_dirs(staged, &has_modinfo, &mut dirs)?;
    if !dirs.is_empty() {
        return dirs.into_iter().map(manifest_root).collect();
    }
    collect_dirs(staged, &has_data_folder, &mut dirs)?;
    let inferred = !dirs.is_empty();
    if !inferred {
        dirs = visible_children(staged)?;
    }
    Ok(dirs
        .into_iter()
        .map(|dir| FoundRoot {
            dir,
            manifest: if inferred {
                Manifest::Inferred
            } else {
                Manifest::Unrecognised
            },
            sub_mod_ids: Vec::new(),
        })
        .collect())
}

/// The game a root targets: modinfo's `Anno` first, then the asset layout.
pub(crate) fn detect_game(root: &FoundRoot) -> Option<GameId> {
    if let Manifest::Parsed(info) = &root.manifest {
        match info.anno {
            Some(ANNO_117_MARKER) => return Some(GameId::Anno117),
            Some(ANNO_1800_MARKER) => return Some(GameId::Anno1800),
            _ => {}
        }
    }
    if root.dir.join("data/base/config").is_dir() {
        Some(GameId::Anno117)
    } else if root.dir.join("data/config/export/main").is_dir() {
        Some(GameId::Anno1800)
    } else {
        None
    }
}

/// Stops descending at the first match, so a match's descendants are never roots.
fn collect_dirs(dir: &Path, is_root: &dyn Fn(&Path) -> bool, out: &mut Vec<PathBuf>) -> Result<()> {
    if is_root(dir) {
        out.push(dir.to_path_buf());
        return Ok(());
    }
    for child in visible_children(dir)? {
        collect_dirs(&child, is_root, out)?;
    }
    Ok(())
}

fn manifest_root(dir: PathBuf) -> Result<FoundRoot> {
    let manifest = match modinfo::read_dir(&dir) {
        Ok(Some(info)) => Manifest::Parsed(Box::new(info)),
        Ok(None) => Manifest::Unrecognised,
        Err(error) => Manifest::Broken(error.to_string()),
    };
    let mut sub_mod_ids = Vec::new();
    collect_sub_mod_ids(&dir, &mut sub_mod_ids)?;
    Ok(FoundRoot {
        dir,
        manifest,
        sub_mod_ids,
    })
}

fn collect_sub_mod_ids(dir: &Path, out: &mut Vec<String>) -> Result<()> {
    for child in visible_children(dir)? {
        if has_modinfo(&child) {
            let id = modinfo::read_dir(&child)
                .ok()
                .flatten()
                .and_then(|info| info.mod_id)
                .unwrap_or_else(|| normalize_mod_id(&file_name(&child)));
            out.push(id);
        }
        collect_sub_mod_ids(&child, out)?;
    }
    Ok(())
}

/// Child folders, minus `.`-prefixed ones the game ignores (as scan.rs does).
fn visible_children(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(child_dirs(dir)?
        .into_iter()
        .filter(|child| !file_name(child).starts_with('.'))
        .collect())
}
