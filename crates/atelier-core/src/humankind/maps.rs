//! Maps: `.hmap` files the game lists from anywhere under Community
//! (`MapListHelper.FindMapFiles`). They never go in Registry.xml.

use crate::{GameId, Mod};

pub const MAP_EXTENSION: &str = ".hmap";
pub const MAP_CATEGORY: &str = "Map";

/// The map a folder's files hold: the first `.hmap` by name, as the game names it.
pub fn map_name(files: &[String]) -> Option<&str> {
    files
        .iter()
        .filter_map(|file| {
            let stem_end = file.len().checked_sub(MAP_EXTENSION.len())?;
            let extension = file.get(stem_end..)?;
            let stem = &file[..stem_end];
            (extension.eq_ignore_ascii_case(MAP_EXTENSION) && !stem.is_empty()).then_some(stem)
        })
        .min()
}

pub fn map_id(name: &str) -> String {
    name.to_lowercase()
}

pub fn is_map(item: &Mod) -> bool {
    item.game == GameId::Humankind && item.category == MAP_CATEGORY
}
