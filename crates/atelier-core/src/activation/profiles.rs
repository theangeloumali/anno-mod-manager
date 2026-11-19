//! Named 117 profiles: `profiles/<name>.txt`, selected by `ActiveProfile <name>` in `profile.txt`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::json;

use super::profile_file::ProfileFile;
use super::{
    ACTIVE_PROFILE_KEY, DEFAULT_PROFILE, PROFILE_POINTER, PROFILES_DIR, VANILLA_PROFILE,
    detect_scheme, edit_profile_file, find_profile, profile_files, read_active_profile_name,
    same_profile_name,
};
use crate::error::io_at;
use crate::scan::ModsFolder;
use crate::snapshots::{Snapshots, ensure_writable, write_with_snapshot};
use crate::text::read_optional;
use crate::{ActivationScheme, Error, GameId, Mod, Profile, Result};

const BUILT_IN_PROFILES: [&str; 2] = ["Default", "Vanilla"];
const MAX_PROFILE_NAME: usize = 64;

pub fn list_profiles(mods_dir: &Path) -> Result<Vec<Profile>> {
    require_profile_scheme(mods_dir)?;
    let active = active_name(mods_dir)?;
    let built_in = BUILT_IN_PROFILES.iter().map(|name| Profile {
        name: (*name).to_string(),
        built_in: true,
        active: same_profile_name(name, &active),
    });
    let saved = profile_files(mods_dir)?
        .into_iter()
        .filter(|(stem, _)| built_in_name(stem).is_none())
        .map(|(stem, _)| Profile {
            active: same_profile_name(&stem, &active),
            name: stem,
            built_in: false,
        });
    Ok(built_in.chain(saved).collect())
}

/// Points `profile.txt` at `name`, keeping every other line of the file.
pub fn switch_profile(writable: &ModsFolder, name: &str, snapshots: &Snapshots) -> Result<()> {
    let dir = prepare(writable, name)?;
    let canonical = match built_in_name(name) {
        Some(built_in) => built_in.to_ascii_lowercase(),
        None => saved_stem(dir, name)?,
    };
    let pointer = dir.join(PROFILE_POINTER);
    let text = read_optional(&pointer)?.unwrap_or_default();
    let new_line = format!("{ACTIVE_PROFILE_KEY} {canonical}");
    let mut replaced = false;
    let mut lines: Vec<String> = text
        .lines()
        .map(|line| {
            if !replaced && line.trim().starts_with(ACTIVE_PROFILE_KEY) {
                replaced = true;
                new_line.clone()
            } else {
                line.to_string()
            }
        })
        .collect();
    if !replaced {
        lines.push(new_line);
    }
    write_with_snapshot(&pointer, &(lines.join("\n") + "\n"), snapshots)
}

/// Saves the current on/off state of every top-level mod as a new or replaced profile.
pub fn save_profile(
    writable: &ModsFolder,
    name: &str,
    mods: &[Mod],
    snapshots: &Snapshots,
) -> Result<()> {
    let dir = prepare(writable, name)?;
    if built_in_name(name).is_some() {
        return Err(Error::BuiltInProfile(name.to_string()));
    }
    let mut file = ProfileFile::new(true);
    let mut seen = HashSet::new();
    for target in mods.iter().filter(|m| m.parent_id.is_none()) {
        if seen.insert(target.id.as_str()) {
            file.set(&target.id, target.active);
        }
    }
    let path = find_profile(dir, name)?
        .unwrap_or_else(|| dir.join(PROFILES_DIR).join(format!("{name}.txt")));
    write_with_snapshot(&path, &file.render(), snapshots)
}

/// Records `mod_ids` as on or off in one profile, active or not. A built-in profile's file is
/// created on first write; a saved profile that is gone is an error, never recreated.
pub fn set_in_profile(
    writable: &ModsFolder,
    name: &str,
    mod_ids: &[String],
    active: bool,
    snapshots: &Snapshots,
) -> Result<()> {
    let dir = prepare(writable, name)?;
    let path = match (find_profile(dir, name)?, built_in_name(name)) {
        (Some(path), _) => path,
        (None, Some(built_in)) => dir
            .join(PROFILES_DIR)
            .join(format!("{}.txt", built_in.to_ascii_lowercase())),
        (None, None) => return Err(Error::ProfileNotFound(name.to_string())),
    };
    let enable_new = !name.eq_ignore_ascii_case(VANILLA_PROFILE);
    edit_profile_file(
        &path,
        enable_new,
        |file| {
            for mod_id in mod_ids {
                file.set(mod_id, active);
            }
        },
        snapshots,
    )
}

pub fn delete_profile(writable: &ModsFolder, name: &str, snapshots: &Snapshots) -> Result<()> {
    let dir = prepare(writable, name)?;
    if built_in_name(name).is_some() {
        return Err(Error::BuiltInProfile(name.to_string()));
    }
    let path = find_profile(dir, name)?.ok_or_else(|| Error::ProfileNotFound(name.into()))?;
    if same_profile_name(&active_name(dir)?, name) {
        return Err(Error::ProfileActive(name.to_string()));
    }
    snapshots.take(&path)?;
    fs::remove_file(&path).map_err(io_at(&path))
}

/// `{name, enableNewMods, mods: [{id, enabled}]}` for sharing a profile.
pub fn export_profile(mods_dir: &Path, name: &str) -> Result<String> {
    require_profile_scheme(mods_dir)?;
    validate_name(name)?;
    let file = match find_profile(mods_dir, name)? {
        Some(path) => ProfileFile::parse(&read_optional(&path)?.unwrap_or_default()),
        None if built_in_name(name).is_some() => {
            ProfileFile::new(!name.eq_ignore_ascii_case(VANILLA_PROFILE))
        }
        None => return Err(Error::ProfileNotFound(name.to_string())),
    };
    let mods: Vec<_> = file
        .entries()
        .map(|(id, enabled)| json!({ "id": id, "enabled": enabled }))
        .collect();
    let export = json!({ "name": name, "enableNewMods": file.enable_new_mods(), "mods": mods });
    Ok(serde_json::to_string_pretty(&export)?)
}

fn prepare<'a>(writable: &'a ModsFolder, name: &str) -> Result<&'a Path> {
    ensure_writable(writable)?;
    require_profile_scheme(&writable.path)?;
    validate_name(name)?;
    Ok(&writable.path)
}

fn require_profile_scheme(mods_dir: &Path) -> Result<()> {
    match detect_scheme(GameId::Anno117, mods_dir) {
        ActivationScheme::Profile => Ok(()),
        other => Err(Error::NoProfiles(other)),
    }
}

/// Names become file names, so path separators and dots are refused.
pub(super) fn validate_name(name: &str) -> Result<()> {
    let allowed = |c: char| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_');
    if name.trim().is_empty() || name.len() > MAX_PROFILE_NAME || !name.chars().all(allowed) {
        return Err(Error::InvalidProfileName(name.to_string()));
    }
    Ok(())
}

fn built_in_name(name: &str) -> Option<&'static str> {
    BUILT_IN_PROFILES
        .into_iter()
        .find(|built_in| built_in.eq_ignore_ascii_case(name))
}

fn saved_stem(mods_dir: &Path, name: &str) -> Result<String> {
    profile_files(mods_dir)?
        .into_iter()
        .find(|(stem, _)| same_profile_name(stem, name))
        .map(|(stem, _)| stem)
        .ok_or_else(|| Error::ProfileNotFound(name.to_string()))
}

fn active_name(mods_dir: &Path) -> Result<String> {
    Ok(read_active_profile_name(mods_dir)?.unwrap_or_else(|| DEFAULT_PROFILE.into()))
}
