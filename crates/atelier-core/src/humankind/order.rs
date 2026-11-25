//! The load order as the game stores it: Registry.xml, read and written through the rules.

use std::iter;
use std::path::Path;

use super::module_name::short_name;
use super::registry::BASE_MODULE;
use super::{detect, load_order, registry};
use crate::activation::Snapshots;
use crate::{Error, Mod, Result};

/// Short names, the base game first; the base game alone until the game writes a registry.
pub fn read(game_dir: &Path) -> Result<Vec<String>> {
    let modules = match detect::registry_path(game_dir)? {
        Some(path) => registry::read_modules(&path)?,
        None => Vec::new(),
    };
    Ok(load_order::normalize(&modules))
}

/// Checks the order as given, so a mistake is reported rather than silently fixed, and refuses
/// a list naming an uninstalled module, which the game would reset to vanilla.
pub fn write(
    game_dir: &Path,
    order: &[String],
    installed: &[Mod],
    snapshots: &Snapshots,
) -> Result<Vec<String>> {
    let path = detect::registry_path(game_dir)?
        .ok_or_else(|| Error::RegistryNotFound(game_dir.to_path_buf()))?;
    let order = with_base(order);
    load_order::validate(&order, &load_order::module_types(installed))?;
    let missing = load_order::missing(&order, installed);
    if !missing.is_empty() {
        return Err(Error::MissingModules(missing));
    }
    registry::write_modules(&path, &order, snapshots)?;
    Ok(order)
}

/// Dropping a module never adds a missing one, so a stale entry elsewhere mustn't block it.
pub fn remove(game_dir: &Path, module: &str, snapshots: &Snapshots) -> Result<Vec<String>> {
    let current = read(game_dir)?;
    let order = load_order::disable(&current, module)?;
    if order == current {
        return Ok(order);
    }
    let path = detect::registry_path(game_dir)?
        .ok_or_else(|| Error::RegistryNotFound(game_dir.to_path_buf()))?;
    registry::write_modules(&path, &order, snapshots)?;
    Ok(order)
}

/// The UI lists only mods, so a missing base game goes first; one listed elsewhere is an error.
fn with_base(order: &[String]) -> Vec<String> {
    let names: Vec<String> = order
        .iter()
        .map(|entry| short_name(entry.trim()))
        .filter(|name| !name.is_empty())
        .collect();
    if names.iter().any(|name| name == BASE_MODULE) {
        return names;
    }
    iter::once(BASE_MODULE.to_string()).chain(names).collect()
}
