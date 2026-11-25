//! Humankind has no modinfo: each module file with its `.assetbundle` beside it is one module,
//! and each other folder holding a `.hmap` is one map.

use std::fs;
use std::path::Path;

use super::bundle::read_module_info;
use super::maps;
use super::module_name::ModuleName;
use crate::error::io_at;
use crate::scan::{ModsFolder, ScanResult, SkipReason, dir_size, file_name, skip};
use crate::{GameId, GameSetup, Mod, ModDependencies, ModLocation, ModOrigin, Result};

const MODIO_DIR: &str = "mod.io";

pub(crate) fn scan_folder(folder: &ModsFolder, result: &mut ScanResult) -> Result<()> {
    visit(folder, &folder.path, result)
}

fn visit(folder: &ModsFolder, dir: &Path, result: &mut ScanResult) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut subdirs = Vec::new();
    let mut files = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let entry = entry.map_err(io_at(dir))?;
        // file_type() does not follow symlinks, so linked folders are never walked.
        let kind = entry.file_type().map_err(io_at(&entry.path()))?;
        let name = file_name(&entry.path());
        if kind.is_dir() && !name.starts_with('.') {
            subdirs.push(entry.path());
        } else if kind.is_file() {
            files.push(name);
        }
    }
    files.sort();
    let items = maps::folder_items(&files);
    for (module, bundle) in items.modules {
        add_module(folder, dir, module, bundle, result)?;
    }
    if let Some(name) = items.map {
        add_map(folder, dir, name, result)?;
    }
    subdirs.sort();
    for subdir in subdirs {
        visit(folder, &subdir, result)?;
    }
    Ok(())
}

fn add_module(
    folder: &ModsFolder,
    dir: &Path,
    module: ModuleName,
    bundle: Option<&String>,
    result: &mut ScanResult,
) -> Result<()> {
    let parts = relative_parts(&folder.path, dir);
    if parts.is_empty() {
        let reason = "a module needs its own folder inside the mods folder".to_string();
        skip(
            result,
            &dir.join(module.to_string()),
            SkipReason::Invalid(reason),
        );
        return Ok(());
    }
    // The game can't load a module without its bundle, so it never counts as installed.
    let Some(bundle) = bundle else {
        let reason = format!(
            "{module} has no {} next to it, so Humankind can't load it",
            module.bundle_name()
        );
        skip(
            result,
            &dir.join(module.to_string()),
            SkipReason::Invalid(reason),
        );
        return Ok(());
    };
    // An unreadable bundle leaves the category blank: "type not verified", never a guessed type.
    let info = read_module_info(&dir.join(bundle)).ok();
    let mut item = community_mod(folder, dir, &parts, module.short_name(), &module.name)?;
    item.category = info
        .as_ref()
        .map(|info| info.module_type.as_str().to_string())
        .unwrap_or_default();
    item.version = module.version();
    item.author = info
        .as_ref()
        .map(|info| info.author.clone())
        .filter(|text| !text.is_empty());
    item.description = info
        .map(|info| info.description)
        .filter(|text| !text.is_empty());
    item.module_guid = module.guid().map(str::to_lowercase);
    result.mods.push(item);
    Ok(())
}

fn add_map(folder: &ModsFolder, dir: &Path, name: &str, result: &mut ScanResult) -> Result<()> {
    let parts = relative_parts(&folder.path, dir);
    if parts.is_empty() {
        let reason = "a map needs its own folder inside the mods folder".to_string();
        let path = dir.join(format!("{name}.hmap"));
        skip(result, &path, SkipReason::Invalid(reason));
        return Ok(());
    }
    let mut item = community_mod(folder, dir, &parts, maps::map_id(name), name)?;
    item.category = maps::MAP_CATEGORY.to_string();
    result.mods.push(item);
    Ok(())
}

/// A Community folder's item, blank where only a module's bundle can say more.
fn community_mod(
    folder: &ModsFolder,
    dir: &Path,
    parts: &[String],
    id: String,
    name: &str,
) -> Result<Mod> {
    let from_modio = parts[0].eq_ignore_ascii_case(MODIO_DIR);
    Ok(Mod {
        id,
        game: GameId::Humankind,
        folder: parts.join("/"),
        name: name.to_string(),
        category: String::new(),
        version: String::new(),
        author: None,
        description: None,
        difficulty: None,
        location: match folder.location {
            ModLocation::Documents if from_modio => ModLocation::GameModio,
            other => other,
        },
        origin: if from_modio {
            ModOrigin::Modio
        } else {
            ModOrigin::Local
        },
        modio_id: if from_modio { modio_id(parts) } else { None },
        active: true,
        size_bytes: dir_size(dir)?,
        setup: GameSetup::default(),
        dependencies: ModDependencies::default(),
        known_issues: Vec::new(),
        has_options: false,
        parent_id: None,
        sub_mod_ids: Vec::new(),
        update: None,
        module_guid: None,
    })
}

fn relative_parts(root: &Path, dir: &Path) -> Vec<String> {
    dir.strip_prefix(root)
        .map(|relative| {
            relative
                .iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// mod.io installs live in `mod.io/<modId>_<modfileId>/…`.
fn modio_id(parts: &[String]) -> Option<u64> {
    let (mod_id, _) = parts.get(1)?.split_once('_')?;
    mod_id.parse().ok()
}
