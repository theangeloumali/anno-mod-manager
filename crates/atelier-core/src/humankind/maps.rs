//! Maps: `.hmap` files the game lists from anywhere under Community
//! (`MapListHelper.FindMapFiles`). They never go in Registry.xml.

use std::path::Path;

use super::module_name::ModuleName;
use crate::{GameId, Mod};

pub const MAP_CATEGORY: &str = "Map";
const MAP_EXTENSION: &str = "hmap";
/// Keeps map ids apart from module short names, so a map never stands in for a module.
const MAP_ID_PREFIX: &str = "map~";

/// What one folder's files hold: the rule the scan and the installer share.
pub struct FolderItems<'f> {
    /// Each module with its bundle, if the bundle sits beside it.
    pub modules: Vec<(ModuleName, Option<&'f String>)>,
    /// A module's folder ships its maps inside the module, so only a folder without one is a map.
    pub map: Option<&'f str>,
}

pub fn folder_items(files: &[String]) -> FolderItems<'_> {
    let modules: Vec<_> = files
        .iter()
        .filter_map(|file| ModuleName::detect(file, files))
        .map(|module| {
            let bundle = module.bundle_in(files);
            (module, bundle)
        })
        .collect();
    let map = if modules.is_empty() {
        map_name(files)
    } else {
        None
    };
    FolderItems { modules, map }
}

/// The first `.hmap` by name, without its extension, as the game names the map.
fn map_name(files: &[String]) -> Option<&str> {
    files
        .iter()
        .map(Path::new)
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case(MAP_EXTENSION))
        })
        .filter_map(|path| path.file_stem()?.to_str())
        .filter(|stem| !stem.is_empty())
        .min()
}

pub fn map_id(name: &str) -> String {
    format!("{MAP_ID_PREFIX}{}", name.to_lowercase())
}

pub fn is_map(item: &Mod) -> bool {
    item.game == GameId::Humankind && item.category == MAP_CATEGORY
}
