//! The UI contract: each type mirrors `src/data/types.ts` field for field.

use atelier_core::{GameId, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallItemStatus {
    New,
    Update,
    Downgrade,
    Same,
    WrongGame,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallCheck {
    pub level: Severity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreviewItem {
    /// Path inside the drop, `/`-separated.
    pub root_path: String,
    pub mod_id: String,
    pub name: String,
    pub category: String,
    pub version: String,
    pub previous_version: Option<String>,
    /// `None` when neither modinfo nor the asset layout says.
    pub target_game: Option<GameId>,
    pub status: InstallItemStatus,
    pub checks: Vec<InstallCheck>,
    pub sub_mod_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPreview {
    pub id: String,
    pub source_name: String,
    pub items: Vec<InstallPreviewItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallResult {
    pub installed: Vec<String>,
    pub skipped: Vec<String>,
}
