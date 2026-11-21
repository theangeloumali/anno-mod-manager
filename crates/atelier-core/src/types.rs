//! The UI contract: each type mirrors `src/data/types.ts` field for field.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameId {
    Anno117,
    Anno1800,
    Humankind,
}

impl GameId {
    pub const ALL: [GameId; 3] = [GameId::Anno117, GameId::Anno1800, GameId::Humankind];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Store {
    Steam,
    Ubisoft,
    Epic,
    Manual,
}

/// Where the loader found a mod. Only `Documents` is ours to write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModLocation {
    Documents,
    Game,
    Ubi,
    /// Steam Workshop content; Steam owns it, so it stays read-only.
    Workshop,
}

impl ModLocation {
    /// The folder as players know it, for messages.
    pub fn place(self) -> &'static str {
        match self {
            Self::Documents => "Documents mods folder",
            Self::Game => "game folder",
            Self::Ubi => "Ubisoft-managed .ubi folder",
            Self::Workshop => "Steam Workshop folder",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModOrigin {
    Modio,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Ok,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueLevel {
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Cheat,
    Easier,
    Unchanged,
    Harder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActivationScheme {
    Profile,
    Legacy,
    ActivationJson,
    /// Humankind: one ordered module list in the user's Registry.xml.
    RegistryList,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameInfo {
    pub id: GameId,
    pub display_name: String,
    pub install_path: Option<String>,
    pub store: Option<Store>,
    pub mods_path: String,
    pub log_path: String,
    pub running: bool,
    pub activation_scheme: ActivationScheme,
    /// The old community 1800 loader is in place of the game's python35.dll.
    pub legacy_mod_loader: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSetup {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_new_game: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safe_to_remove: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplayer: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub campaign: Option<bool>,
}

/// Normalised ModIDs; `load_after` may hold `*` (load in the late phase).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModDependencies {
    pub require: Vec<String>,
    pub optional: Vec<String>,
    pub load_after: Vec<String>,
    pub incompatible: Vec<String>,
    pub deprecate: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModUpdate {
    pub version: String,
    pub modfile_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mod {
    /// Normalised ModID, as the game writes it.
    pub id: String,
    pub game: GameId,
    /// Path relative to its mods folder, `/`-separated (sub-mods include the parent).
    pub folder: String,
    pub name: String,
    pub category: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub difficulty: Option<Difficulty>,
    pub location: ModLocation,
    pub origin: ModOrigin,
    pub modio_id: Option<u64>,
    pub active: bool,
    pub size_bytes: u64,
    pub setup: GameSetup,
    pub dependencies: ModDependencies,
    pub known_issues: Vec<String>,
    pub has_options: bool,
    pub parent_id: Option<String>,
    pub sub_mod_ids: Vec<String>,
    pub update: Option<ModUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IssueKind {
    MissingDependency,
    InactiveDependency,
    Incompatible,
    Deprecated,
    Dlc,
    DuplicateId,
    LoadOrderCycle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub mod_id: String,
    pub level: IssueLevel,
    pub kind: IssueKind,
    pub related_mod_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Normal,
    Late,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LoadReason {
    LoadAfter,
    Late,
    Alphabetical,
    /// Humankind: the player set this order; nothing computes it.
    Player,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrderEntry {
    /// 1-based.
    pub position: usize,
    pub mod_id: String,
    pub phase: Phase,
    pub reason: LoadReason,
    pub after: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOrder {
    pub entries: Vec<LoadOrderEntry>,
    pub cycles: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub built_in: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModOptionType {
    Toggle,
    Enum,
    Slider,
    Text,
    Color,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionValue {
    Bool(bool),
    Number(f64),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModOption {
    pub key: String,
    pub label: String,
    pub hint: Option<String>,
    pub group: Option<String>,
    #[serde(rename = "type")]
    pub option_type: ModOptionType,
    pub value: OptionValue,
    pub default_value: OptionValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub id: String,
    pub level: Severity,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub time: String,
    pub level: LogLevel,
    pub mod_id: Option<String>,
    pub message: String,
    pub source: Option<String>,
}
