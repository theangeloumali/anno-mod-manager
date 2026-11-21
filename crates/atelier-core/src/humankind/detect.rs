//! Finds the player's Registry.xml under `Documents/Humankind/Users/<id>/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::Result;
use crate::error::io_at;

const USERS_DIR: &str = "Users";
const REGISTRY_FILE: &str = "Registry.xml";

/// The game writes the signed-in account's file, so the newest one is the live one.
pub fn registry_path(game_dir: &Path) -> Result<Option<PathBuf>> {
    let users = game_dir.join(USERS_DIR);
    if !users.is_dir() {
        return Ok(None);
    }
    let mut newest: Option<(SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(&users).map_err(io_at(&users))? {
        let candidate = entry.map_err(io_at(&users))?.path().join(REGISTRY_FILE);
        let Ok(metadata) = fs::metadata(&candidate) else {
            continue;
        };
        let modified = metadata.modified().map_err(io_at(&candidate))?;
        if newest.as_ref().is_none_or(|(time, _)| modified > *time) {
            newest = Some((modified, candidate));
        }
    }
    Ok(newest.map(|(_, path)| path))
}
