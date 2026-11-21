//! The ModListGuard plugin's log of every change to the game's saved mod list, one JSON object
//! per line, so a list the game reset to vanilla can be put back.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::Result;
use crate::text::read_capped;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// ISO 8601 UTC, as the plugin wrote it.
    pub time: String,
    pub reason: String,
    /// The mod the plugin blames for the change, when it could tell.
    pub culprit: Option<String>,
    /// Module names as the game stored them, in load order.
    pub old: Vec<String>,
    pub new: Vec<String>,
}

#[derive(Deserialize)]
struct RawEntry {
    time: String,
    reason: String,
    #[serde(default)]
    culprit: Option<String>,
    old: String,
    new: String,
}

pub fn history_path(game_dir: &Path) -> PathBuf {
    game_dir.join("ModListGuard").join("history.jsonl")
}

/// Newest first. A line the plugin cut short or that isn't an entry is skipped, not an error.
pub fn read_history(path: &Path) -> Result<Vec<HistoryEntry>> {
    let Some(bytes) = read_capped(path)? else {
        return Ok(Vec::new());
    };
    let mut entries: Vec<HistoryEntry> = bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| std::str::from_utf8(line).ok())
        .filter_map(|line| {
            serde_json::from_str::<RawEntry>(line.trim_start_matches('\u{feff}')).ok()
        })
        .map(|raw| HistoryEntry {
            time: raw.time,
            reason: raw.reason,
            culprit: raw.culprit,
            old: split(&raw.old),
            new: split(&raw.new),
        })
        .collect();
    entries.reverse();
    Ok(entries)
}

fn split(list: &str) -> Vec<String> {
    list.split('/')
        .filter(|module| !module.is_empty())
        .map(str::to_string)
        .collect()
}
