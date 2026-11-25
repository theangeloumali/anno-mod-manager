//! Humankind's load order rules: the base game first, then at most one Standalone, at most one
//! Conversion, then Extensions in the player's order.

use std::collections::{HashMap, HashSet};

use crate::humankind::bundle::ModuleType;
use crate::humankind::maps::is_map;
use crate::humankind::module_name::short_name;
use crate::humankind::registry::BASE_MODULE;
use crate::{Error, Mod, Result};

/// Short name → module type, from a scan.
pub type ModuleTypes = HashMap<String, ModuleType>;

type OrderResult<T> = std::result::Result<T, OrderError>;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OrderError {
    #[error("'{}' (the base game) must come first", BASE_MODULE)]
    BaseNotFirst,
    #[error("'{0}' is listed twice")]
    Duplicate(String),
    #[error("'{0}' and '{1}' are both Standalone modules; only one can be on")]
    TwoStandalones(String, String),
    #[error("'{0}' and '{1}' are both Conversion modules; only one can be on")]
    TwoConversions(String, String),
    #[error("Standalone module '{0}' must come right after the base game")]
    StandaloneMisplaced(String),
    #[error("Conversion module '{0}' must come right after the base game and any Standalone")]
    ConversionMisplaced(String),
}

/// A blank category means the bundle couldn't be read; [`module_type`] treats it as an Extension.
pub fn module_types(installed: &[Mod]) -> ModuleTypes {
    installed
        .iter()
        .map(|item| {
            let kind = match item.category.as_str() {
                "Standalone" => ModuleType::Standalone,
                "Conversion" => ModuleType::Conversion,
                _ => ModuleType::Extension,
            };
            (item.id.clone(), kind)
        })
        .collect()
}

/// Unverified and uninstalled modules count as Extensions, the game's least restrictive type.
pub fn module_type(types: &ModuleTypes, module: &str) -> ModuleType {
    types.get(module).copied().unwrap_or(ModuleType::Extension)
}

/// Short names, the base game first, each module once (first wins), blanks dropped.
pub fn normalize(order: &[String]) -> Vec<String> {
    let mut seen = HashSet::from([BASE_MODULE.to_string()]);
    let rest = order
        .iter()
        .map(|entry| short_name(entry.trim()))
        .filter(|name| !name.is_empty() && seen.insert(name.clone()));
    std::iter::once(BASE_MODULE.to_string())
        .chain(rest)
        .collect()
}

/// Checks an order as given, so the first rule it breaks is reported, not silently fixed.
pub fn validate(order: &[String], types: &ModuleTypes) -> OrderResult<()> {
    let names: Vec<String> = order.iter().map(|entry| short_name(entry)).collect();
    if names.first().map(String::as_str) != Some(BASE_MODULE) {
        return Err(OrderError::BaseNotFirst);
    }
    let mut seen = HashSet::new();
    if let Some(duplicate) = names.iter().find(|name| !seen.insert(name.as_str())) {
        return Err(OrderError::Duplicate(duplicate.clone()));
    }
    let positions = |wanted: ModuleType| -> Vec<(usize, &String)> {
        names
            .iter()
            .enumerate()
            .filter(|(_, name)| module_type(types, name) == wanted)
            .collect()
    };
    let standalones = positions(ModuleType::Standalone);
    let conversions = positions(ModuleType::Conversion);
    if let [(_, first), (_, second), ..] = standalones[..] {
        return Err(OrderError::TwoStandalones(first.clone(), second.clone()));
    }
    if let [(_, first), (_, second), ..] = conversions[..] {
        return Err(OrderError::TwoConversions(first.clone(), second.clone()));
    }
    if let Some(&(index, name)) = standalones.first()
        && index != 1
    {
        return Err(OrderError::StandaloneMisplaced(name.clone()));
    }
    if let Some(&(index, name)) = conversions.first()
        && index != 1 + standalones.len()
    {
        return Err(OrderError::ConversionMisplaced(name.clone()));
    }
    Ok(())
}

/// A Standalone or Conversion replaces the one already on, as the game allows only one each.
pub fn enable(order: &[String], module: &str, types: &ModuleTypes) -> Vec<String> {
    let mut order = normalize(order);
    let module = short_name(module.trim());
    if module.is_empty() || order.contains(&module) {
        return order;
    }
    let wanted = module_type(types, &module);
    if wanted == ModuleType::Extension {
        order.push(module);
        return order;
    }
    order.retain(|name| module_type(types, name) != wanted);
    let standalones = order
        .iter()
        .filter(|name| module_type(types, name) == ModuleType::Standalone)
        .count();
    let slot = match wanted {
        ModuleType::Standalone => 1,
        _ => 1 + standalones,
    };
    order.insert(slot.min(order.len()), module);
    order
}

pub fn disable(order: &[String], module: &str) -> Result<Vec<String>> {
    let module = short_name(module.trim());
    if module == BASE_MODULE {
        return Err(Error::BaseModule(module));
    }
    let mut order = normalize(order);
    order.retain(|name| *name != module);
    Ok(order)
}

/// Listed modules that aren't installed, each once; the base game ships with the game.
/// A map is not a module, so listing one would make the game reset the list too.
pub fn missing(order: &[String], installed: &[Mod]) -> Vec<String> {
    let present: HashSet<&str> = installed
        .iter()
        .filter(|item| !is_map(item))
        .map(|item| item.id.as_str())
        .collect();
    normalize(order)
        .into_iter()
        .skip(1)
        .filter(|name| !present.contains(name.as_str()))
        .collect()
}
