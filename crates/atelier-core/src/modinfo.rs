//! `modinfo.json` parsing for both games (117 nests dependencies, 1800 keeps them flat).

use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::error::io_at;
use crate::text::{decode, strip_jsonc};
use crate::{
    Difficulty, Error, GameId, GameSetup, ModDependencies, ModOption, ModOptionType, OptionValue,
    Result,
};

const ANNO_117_MARKER: u8 = 8;
const DEFAULT_VERSION: &str = "1.0.0";
const MODINFO_FILE_NAMES: [&str; 2] = ["modinfo.json", "modinfo.jsonc"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlcDependency {
    pub dlc: String,
    pub dependant: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModInfo {
    /// `Anno: 8` marks a 117 mod; 1800 mods usually omit it.
    pub anno: Option<u8>,
    /// Normalised; `None` when the file has no ModID (callers fall back to the folder name).
    pub mod_id: Option<String>,
    pub version: String,
    pub name: Option<String>,
    pub category: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub difficulty: Option<Difficulty>,
    pub setup: GameSetup,
    pub dependencies: ModDependencies,
    pub dlc_dependencies: Vec<DlcDependency>,
    pub known_issues: Vec<String>,
    pub options: Vec<ModOption>,
    pub modio_id: Option<u64>,
}

impl ModInfo {
    pub fn game(&self) -> GameId {
        if self.anno == Some(ANNO_117_MARKER) {
            GameId::Anno117
        } else {
            GameId::Anno1800
        }
    }
}

/// Lowercase, runs of anything but `[a-z0-9*]` become one `-`, edges trimmed.
pub fn normalize_mod_id(raw: &str) -> String {
    let mut normalized = String::with_capacity(raw.len());
    for character in raw.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() || character == '*' {
            normalized.push(character);
        } else if !normalized.ends_with('-') {
            normalized.push('-');
        }
    }
    normalized.trim_matches('-').to_string()
}

pub fn parse(bytes: &[u8]) -> Result<ModInfo> {
    let value: Value = serde_json::from_str(&strip_jsonc(&decode(bytes)))?;
    let Value::Object(root) = value else {
        return Err(Error::InvalidModInfo("the root is not an object".into()));
    };
    Ok(ModInfo {
        anno: root
            .get("Anno")
            .and_then(Value::as_u64)
            .and_then(|n| u8::try_from(n).ok()),
        mod_id: root
            .get("ModID")
            .and_then(Value::as_str)
            .map(normalize_mod_id),
        version: root
            .get("Version")
            .and_then(localized)
            .unwrap_or_else(|| DEFAULT_VERSION.into()),
        name: root.get("ModName").and_then(localized),
        category: root.get("Category").and_then(localized),
        description: root.get("Description").and_then(localized),
        author: root.get("CreatorName").and_then(localized),
        difficulty: root.get("Difficulty").and_then(parse_difficulty),
        setup: parse_setup(root.get("GameSetup")),
        dependencies: parse_dependencies(&root),
        dlc_dependencies: parse_dlc_dependencies(root.get("DLCDependencies")),
        known_issues: localized_list(root.get("KnownIssues")),
        options: parse_options(root.get("Options")),
        modio_id: root.get("ModioResourceId").and_then(as_u64_lenient),
    })
}

/// Reads `modinfo.json` (or `.jsonc`) in `dir`; `Ok(None)` when neither exists.
pub fn read_dir(dir: &Path) -> Result<Option<ModInfo>> {
    let Some(path) = MODINFO_FILE_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
    else {
        return Ok(None);
    };
    let bytes = fs::read(&path).map_err(io_at(&path))?;
    parse(&bytes).map(Some)
}

/// A plain string, or a language map where English wins, then the first entry.
fn localized(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Object(languages) => languages
            .get("English")
            .and_then(Value::as_str)
            .or_else(|| languages.values().find_map(Value::as_str))
            .map(str::to_string),
        _ => None,
    }
}

fn localized_list(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items.iter().filter_map(localized).collect(),
        Some(other) => localized(other).into_iter().collect(),
        None => Vec::new(),
    }
}

fn id_list(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_str)
        .map(normalize_mod_id)
        .filter(|id| !id.is_empty())
        .collect()
}

fn as_u64_lenient(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.trim().parse().ok()))
}

fn parse_difficulty(value: &Value) -> Option<Difficulty> {
    match value.as_str()?.to_ascii_lowercase().as_str() {
        "cheat" => Some(Difficulty::Cheat),
        "easier" => Some(Difficulty::Easier),
        "unchanged" => Some(Difficulty::Unchanged),
        "harder" => Some(Difficulty::Harder),
        _ => None,
    }
}

fn parse_setup(value: Option<&Value>) -> GameSetup {
    let flag = |key: &str| {
        value
            .and_then(|setup| setup.get(key))
            .and_then(Value::as_bool)
    };
    GameSetup {
        requires_new_game: flag("RequiresNewGame"),
        safe_to_remove: flag("SafeToRemove"),
        multiplayer: flag("Multiplayer"),
        campaign: flag("Campaign"),
    }
}

/// 117 nests lists under `Dependencies`; 1800 uses flat keys. Both are merged.
fn parse_dependencies(root: &Map<String, Value>) -> ModDependencies {
    let nested = root.get("Dependencies");
    let merged = |nested_key: &str, flat_key: &str| {
        let mut ids = id_list(nested.and_then(|deps| deps.get(nested_key)));
        ids.extend(id_list(root.get(flat_key)));
        ids
    };
    ModDependencies {
        require: merged("Require", "ModDependencies"),
        optional: merged("Optional", "OptionalDependencies"),
        load_after: merged("LoadAfter", "LoadAfterIds"),
        incompatible: merged("Incompatible", "IncompatibleIds"),
        deprecate: merged("Deprecate", "DeprecateIds"),
    }
}

fn parse_dlc_dependencies(value: Option<&Value>) -> Vec<DlcDependency> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            Some(DlcDependency {
                dlc: item.get("DLC")?.as_str()?.to_string(),
                dependant: item
                    .get("Dependant")
                    .and_then(Value::as_str)
                    .unwrap_or("required")
                    .to_string(),
            })
        })
        .collect()
}

fn parse_options(value: Option<&Value>) -> Vec<ModOption> {
    let Some(Value::Object(definitions)) = value else {
        return Vec::new();
    };
    definitions
        .iter()
        .filter_map(|(key, definition)| parse_option(key, definition.as_object()?))
        .collect()
}

fn parse_option(key: &str, definition: &Map<String, Value>) -> Option<ModOption> {
    let number = |name: &str| definition.get(name).and_then(Value::as_f64);
    let declared_default = definition.get("default").and_then(option_value);
    let option_type = definition
        .get("type")
        .and_then(Value::as_str)
        .and_then(parse_option_type)
        .unwrap_or(match declared_default {
            Some(OptionValue::Bool(_)) => ModOptionType::Toggle,
            Some(OptionValue::Number(_)) => ModOptionType::Slider,
            _ => ModOptionType::Text,
        });
    let values: Option<Vec<String>> = match definition.get("values") {
        Some(Value::Array(items)) => Some(items.iter().filter_map(localized).collect()),
        _ => None,
    };
    let min = number("min");
    let default_value = declared_default.unwrap_or_else(|| match option_type {
        ModOptionType::Toggle => OptionValue::Bool(false),
        ModOptionType::Slider => OptionValue::Number(min.unwrap_or(0.0)),
        ModOptionType::Enum => OptionValue::Text(
            values
                .as_ref()
                .and_then(|items| items.first().cloned())
                .unwrap_or_default(),
        ),
        ModOptionType::Text | ModOptionType::Color => OptionValue::Text(String::new()),
    });
    Some(ModOption {
        key: key.to_string(),
        label: definition
            .get("label")
            .and_then(localized)
            .unwrap_or_else(|| key.to_string()),
        hint: definition
            .get("description")
            .or_else(|| definition.get("hint"))
            .and_then(localized),
        group: definition.get("group").and_then(localized),
        option_type,
        value: default_value.clone(),
        default_value,
        values,
        min,
        max: number("max"),
        step: number("step").or_else(|| number("steps")),
    })
}

fn parse_option_type(raw: &str) -> Option<ModOptionType> {
    match raw.to_ascii_lowercase().as_str() {
        "toggle" | "bool" | "boolean" | "checkbox" => Some(ModOptionType::Toggle),
        "enum" | "select" | "dropdown" => Some(ModOptionType::Enum),
        "slider" | "range" | "number" | "int" | "float" => Some(ModOptionType::Slider),
        "text" | "string" => Some(ModOptionType::Text),
        "color" | "colour" => Some(ModOptionType::Color),
        _ => None,
    }
}

pub(crate) fn option_value(value: &Value) -> Option<OptionValue> {
    match value {
        Value::Bool(flag) => Some(OptionValue::Bool(*flag)),
        Value::Number(number) => number.as_f64().map(OptionValue::Number),
        Value::String(text) => Some(OptionValue::Text(text.clone())),
        _ => None,
    }
}
