//! Finds mod roots in a staged tree and tells which game a root was built for.

use std::fs;
use std::path::{Path, PathBuf};

use atelier_core::humankind::maps;
use atelier_core::humankind::module_name::ModuleName;
use atelier_core::{GameId, ModInfo, modinfo, normalize_mod_id};

use crate::Result;
use crate::error::io_at;
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
    /// A Humankind module file; the game loads it only with its lowercased bundle beside it.
    Module {
        module: ModuleName,
        has_bundle: bool,
        modules_in_folder: usize,
    },
    /// A Humankind map: a folder holding a `.hmap` and no module, named after the map.
    Map {
        name: String,
    },
    /// A Humankind BepInEx plugin, installed as `BepInEx/plugins/<name>`.
    Plugin {
        name: String,
    },
}

impl Manifest {
    pub fn is_anno(&self) -> bool {
        matches!(self, Self::Parsed(_) | Self::Broken(_) | Self::Inferred)
    }
}

pub(crate) struct FoundRoot {
    pub dir: PathBuf,
    pub manifest: Manifest,
    pub sub_mod_ids: Vec<String>,
}

impl FoundRoot {
    /// The ModID (a module's short name), or the normalised folder name when modinfo has none.
    pub fn mod_id(&self) -> String {
        match &self.manifest {
            Manifest::Parsed(info) if info.mod_id.is_some() => {
                info.mod_id.clone().unwrap_or_default()
            }
            Manifest::Module { module, .. } => module.short_name(),
            Manifest::Map { name } => maps::map_id(name),
            Manifest::Plugin { name } => normalize_mod_id(name),
            _ => normalize_mod_id(&file_name(&self.dir)),
        }
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

pub(crate) fn has_module_file(dir: &Path) -> bool {
    modules_in(dir).is_ok_and(|modules| !modules.is_empty())
}

/// The map a folder holds, by name; a module's folder ships its maps inside the module.
pub(crate) fn map_in(dir: &Path) -> Result<Option<String>> {
    let files = file_names(dir)?;
    let has_module = files
        .iter()
        .any(|name| ModuleName::detect(name, &files).is_some());
    Ok((!has_module)
        .then(|| maps::map_name(&files).map(str::to_string))
        .flatten())
}

/// Loose `.dll`s or a `BepInEx/` folder: what a plugin archive holds at its top level.
pub(crate) fn has_plugin_files(dir: &Path) -> bool {
    has_dll(dir) || child_dirs(dir).is_ok_and(|dirs| dirs.iter().any(|d| is_named(d, "BepInEx")))
}

/// The module files among `dir`'s files, each with whether its bundle sits beside it.
pub(crate) fn modules_in(dir: &Path) -> Result<Vec<(ModuleName, bool)>> {
    let files = file_names(dir)?;
    Ok(files
        .iter()
        .filter_map(|name| ModuleName::detect(name, &files))
        .map(|module| {
            // The game writes the bundle name lowercased, so match it without case.
            let bundle = module.bundle_name();
            let has_bundle = files.iter().any(|name| name.to_lowercase() == bundle);
            (module, has_bundle)
        })
        .collect())
}

/// Humankind roots, then Anno ones; where the two overlap, the previewed game's kind wins.
pub(crate) fn find_roots(staged: &Path, game: GameId) -> Result<Vec<FoundRoot>> {
    let mut humankind = module_roots(staged)?;
    let mut map_folders = map_roots(staged)?;
    map_folders.retain(|map| !overlaps_any(map, &humankind));
    humankind.extend(map_folders);
    if game == GameId::Humankind {
        // A dll inside a module's folder ships with the module; it isn't a second item.
        let mut plugins = plugin_roots(staged)?;
        plugins.retain(|plugin| !overlaps_any(plugin, &humankind));
        humankind.extend(plugins);
    }
    let mut anno = anno_roots(staged)?;
    if game == GameId::Humankind {
        anno.retain(|root| !overlaps_any(root, &humankind));
    } else {
        humankind.retain(|root| !overlaps_any(root, &anno));
    }
    anno.extend(humankind);
    if !anno.is_empty() {
        return Ok(anno);
    }
    Ok(visible_children(staged)?
        .into_iter()
        .map(|dir| plain_root(dir, Manifest::Unrecognised))
        .collect())
}

/// The game a root targets: modinfo's `Anno` first, then the asset layout.
pub(crate) fn detect_game(root: &FoundRoot) -> Option<GameId> {
    match &root.manifest {
        Manifest::Module { .. } | Manifest::Map { .. } | Manifest::Plugin { .. } => {
            return Some(GameId::Humankind);
        }
        Manifest::Parsed(info) => match info.anno {
            Some(ANNO_117_MARKER) => return Some(GameId::Anno117),
            Some(ANNO_1800_MARKER) => return Some(GameId::Anno1800),
            _ => {}
        },
        _ => {}
    }
    if root.dir.join("data/base/config").is_dir() {
        Some(GameId::Anno117)
    } else if root.dir.join("data/config/export/main").is_dir() {
        Some(GameId::Anno1800)
    } else {
        None
    }
}

/// A plugin root that is a `BepInEx/plugins` folder, whose BepInEx siblings aren't installed.
pub(crate) fn is_plugins_dir(dir: &Path) -> bool {
    is_named(dir, "plugins")
        && dir
            .parent()
            .is_some_and(|parent| is_named(parent, "BepInEx"))
}

/// Folders with a modinfo and none above them; without any, the topmost `data/` holders.
fn anno_roots(staged: &Path) -> Result<Vec<FoundRoot>> {
    let mut dirs = Vec::new();
    collect_dirs(staged, &has_modinfo, &mut dirs)?;
    if !dirs.is_empty() {
        return dirs.into_iter().map(manifest_root).collect();
    }
    collect_dirs(staged, &has_data_folder, &mut dirs)?;
    Ok(dirs
        .into_iter()
        .map(|dir| plain_root(dir, Manifest::Inferred))
        .collect())
}

fn module_roots(staged: &Path) -> Result<Vec<FoundRoot>> {
    let mut dirs = Vec::new();
    collect_dirs(staged, &has_module_file, &mut dirs)?;
    let mut roots = Vec::new();
    for dir in dirs {
        let modules = modules_in(&dir)?;
        let modules_in_folder = modules.len();
        if let Some((module, has_bundle)) = modules.into_iter().next() {
            let manifest = Manifest::Module {
                module,
                has_bundle,
                modules_in_folder,
            };
            roots.push(plain_root(dir, manifest));
        }
    }
    Ok(roots)
}

fn map_roots(staged: &Path) -> Result<Vec<FoundRoot>> {
    let mut dirs = Vec::new();
    collect_dirs(
        staged,
        &|dir| map_in(dir).is_ok_and(|map| map.is_some()),
        &mut dirs,
    )?;
    let mut roots = Vec::new();
    for dir in dirs {
        if let Some(name) = map_in(&dir)? {
            roots.push(plain_root(dir, Manifest::Map { name }));
        }
    }
    Ok(roots)
}

/// `…/BepInEx/plugins` trees, named after the folder holding BepInEx; without any, the
/// top-level folders holding loose `.dll`s.
fn plugin_roots(staged: &Path) -> Result<Vec<FoundRoot>> {
    let mut dirs = Vec::new();
    collect_dirs(staged, &is_plugins_dir, &mut dirs)?;
    if dirs.is_empty() {
        dirs = visible_children(staged)?
            .into_iter()
            .filter(|dir| has_dll(dir))
            .collect();
    }
    Ok(dirs
        .into_iter()
        .map(|dir| {
            let holder = if is_plugins_dir(&dir) {
                dir.ancestors().nth(2).unwrap_or(&dir)
            } else {
                &dir
            };
            let name = file_name(holder);
            plain_root(dir, Manifest::Plugin { name })
        })
        .collect())
}

fn overlaps_any(root: &FoundRoot, others: &[FoundRoot]) -> bool {
    others
        .iter()
        .any(|other| root.dir.starts_with(&other.dir) || other.dir.starts_with(&root.dir))
}

fn plain_root(dir: PathBuf, manifest: Manifest) -> FoundRoot {
    FoundRoot {
        dir,
        manifest,
        sub_mod_ids: Vec::new(),
    }
}

fn is_named(dir: &Path, name: &str) -> bool {
    file_name(dir).eq_ignore_ascii_case(name)
}

fn has_dll(dir: &Path) -> bool {
    file_names(dir).is_ok_and(|names| {
        names.iter().any(|name| {
            Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
        })
    })
}

/// Plain files directly in `dir`, sorted; symlinks are never followed (as scan.rs does).
fn file_names(dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let entry = entry.map_err(io_at(dir))?;
        if entry.file_type().map_err(io_at(&entry.path()))?.is_file() {
            names.push(file_name(&entry.path()));
        }
    }
    names.sort();
    Ok(names)
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
