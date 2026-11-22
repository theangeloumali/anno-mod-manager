//! Humankind has no modinfo: each `<Name>.<guid>.<major>.<minor>` file is one module.

use std::fs;
use std::path::Path;

use super::bundle::read_module_info;
use super::module_name::ModuleName;
use crate::error::io_at;
use crate::scan::{ModsFolder, ScanResult, SkipReason, dir_size, file_name, skip};
use crate::{GameId, GameSetup, Mod, ModDependencies, ModLocation, ModOrigin, Result};

const MODIO_DIR: &str = "mod.io";
const BUNDLE_EXTENSION: &str = ".assetbundle";

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
    for name in &files {
        if let Some(module) = ModuleName::parse(name) {
            add_module(folder, dir, module, &files, result)?;
        }
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
    siblings: &[String],
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
    let Some(bundle) = bundle_name(&module, siblings) else {
        let reason = format!(
            "{module} has no {}.assetbundle next to it, so Humankind can't load it",
            module.to_string().to_lowercase()
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
    let from_modio = parts[0].eq_ignore_ascii_case(MODIO_DIR);
    result.mods.push(Mod {
        id: module.short_name(),
        game: GameId::Humankind,
        folder: parts.join("/"),
        name: module.name.clone(),
        category: info
            .as_ref()
            .map(|info| info.module_type.as_str().to_string())
            .unwrap_or_default(),
        version: module.version(),
        author: info
            .as_ref()
            .map(|info| info.author.clone())
            .filter(|text| !text.is_empty()),
        description: info
            .map(|info| info.description)
            .filter(|text| !text.is_empty()),
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
        modio_id: if from_modio { modio_id(&parts) } else { None },
        active: true,
        size_bytes: dir_size(dir)?,
        setup: GameSetup::default(),
        dependencies: ModDependencies::default(),
        known_issues: Vec::new(),
        has_options: false,
        parent_id: None,
        sub_mod_ids: Vec::new(),
        update: None,
    });
    Ok(())
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

/// The game writes the bundle name lowercased, so match it without case.
fn bundle_name<'a>(module: &ModuleName, siblings: &'a [String]) -> Option<&'a String> {
    let wanted = format!("{module}{BUNDLE_EXTENSION}").to_lowercase();
    siblings.iter().find(|name| name.to_lowercase() == wanted)
}

/// mod.io installs live in `mod.io/<modId>_<modfileId>/…`.
fn modio_id(parts: &[String]) -> Option<u64> {
    let (mod_id, _) = parts.get(1)?.split_once('_')?;
    mod_id.parse().ok()
}
