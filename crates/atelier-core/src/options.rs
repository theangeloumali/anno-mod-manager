//! Mod option values in `mods/active-options.jsonc`, nested as `{ "<mod-id>": { "<key>": value } }`.

use std::path::Path;

use serde_json::{Map, Value};

use crate::modinfo::option_value;
use crate::scan::ModsFolder;
use crate::snapshots::{Snapshots, ensure_writable, write_with_snapshot};
use crate::text::{read_optional, strip_jsonc};
use crate::{ModOption, OptionValue, Result};

const OPTIONS_FILE: &str = "active-options.jsonc";

/// The modinfo defaults with any saved values laid over them.
pub fn load(mods_dir: &Path, mod_id: &str, defaults: &[ModOption]) -> Result<Vec<ModOption>> {
    let root = read_root(mods_dir)?;
    let saved = root.get(mod_id).and_then(Value::as_object);
    Ok(defaults
        .iter()
        .map(|option| {
            let mut loaded = option.clone();
            if let Some(value) = saved
                .and_then(|values| values.get(&option.key))
                .and_then(option_value)
            {
                loaded.value = value;
            }
            loaded
        })
        .collect())
}

/// Merges `values` into the mod's entry; other mods' entries are kept.
/// Comments in the file are not preserved, so the previous copy goes to a snapshot.
pub fn save(
    writable: &ModsFolder,
    mod_id: &str,
    values: &[(String, OptionValue)],
    snapshots: &Snapshots,
) -> Result<()> {
    ensure_writable(writable)?;
    let mut root = read_root(&writable.path)?;
    let entry = root
        .entry(mod_id)
        .or_insert_with(|| Value::Object(Map::new()));
    if !entry.is_object() {
        *entry = Value::Object(Map::new());
    }
    if let Value::Object(saved) = entry {
        for (key, value) in values {
            saved.insert(key.clone(), serde_json::to_value(value)?);
        }
    }
    let text = serde_json::to_string_pretty(&Value::Object(root))?;
    write_with_snapshot(&writable.path.join(OPTIONS_FILE), &text, snapshots)
}

fn read_root(mods_dir: &Path) -> Result<Map<String, Value>> {
    let Some(text) = read_optional(&mods_dir.join(OPTIONS_FILE))? else {
        return Ok(Map::new());
    };
    match serde_json::from_str(&strip_jsonc(&text))? {
        Value::Object(root) => Ok(root),
        _ => Ok(Map::new()),
    }
}
