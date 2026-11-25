//! Named Humankind load orders. The game has no profiles of its own, so Atelier keeps them in
//! its data folder.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::activation::{Snapshots, same_profile_name, validate_profile_name};
use crate::humankind::load_order::normalize;
use crate::snapshots::write_with_snapshot;
use crate::text::read_optional;
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HumankindProfile {
    pub name: String,
    /// Short names in load order, the base game first.
    pub modules: Vec<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct ProfilesFile {
    profiles: Vec<HumankindProfile>,
}

pub fn profiles_path(data_dir: &Path) -> PathBuf {
    data_dir.join("humankind").join("profiles.json")
}

/// A corrupt file is an error, so a later save can't silently replace the player's profiles.
pub fn list(path: &Path) -> Result<Vec<HumankindProfile>> {
    Ok(read(path)?.profiles)
}

pub fn find(path: &Path, name: &str) -> Result<HumankindProfile> {
    list(path)?
        .into_iter()
        .find(|profile| same_profile_name(&profile.name, name))
        .ok_or_else(|| Error::ProfileNotFound(name.to_string()))
}

/// Names are unique ignoring case; saving over one keeps its original spelling.
pub fn save(
    path: &Path,
    name: &str,
    modules: &[String],
    snapshots: &Snapshots,
) -> Result<HumankindProfile> {
    validate_profile_name(name)?;
    let mut file = read(path)?;
    let modules = normalize(modules);
    let saved = match file
        .profiles
        .iter_mut()
        .find(|profile| same_profile_name(&profile.name, name))
    {
        Some(existing) => {
            existing.modules = modules;
            existing.clone()
        }
        None => {
            let profile = HumankindProfile {
                name: name.to_string(),
                modules,
            };
            file.profiles.push(profile.clone());
            profile
        }
    };
    write(path, &file, snapshots)?;
    Ok(saved)
}

pub fn delete(path: &Path, name: &str, snapshots: &Snapshots) -> Result<()> {
    let mut file = read(path)?;
    let before = file.profiles.len();
    file.profiles
        .retain(|profile| !same_profile_name(&profile.name, name));
    if file.profiles.len() == before {
        return Err(Error::ProfileNotFound(name.to_string()));
    }
    write(path, &file, snapshots)
}

fn read(path: &Path) -> Result<ProfilesFile> {
    match read_optional(path)? {
        Some(text) => Ok(serde_json::from_str(&text)?),
        None => Ok(ProfilesFile::default()),
    }
}

fn write(path: &Path, file: &ProfilesFile, snapshots: &Snapshots) -> Result<()> {
    write_with_snapshot(path, &serde_json::to_string_pretty(file)?, snapshots)
}
