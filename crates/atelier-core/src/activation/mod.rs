//! Which mods are switched on. 117 reads `profile.txt` → `profiles/<name>.txt` (or the
//! legacy `active-profile.txt`); 1800 reads `activation.json` plus `-` folder prefixes.

mod profile_file;
mod profiles;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::error::io_at;
use crate::humankind::{self, maps};
use crate::scan::ModsFolder;
pub use crate::snapshots::Snapshots;
use crate::snapshots::{ensure_writable, write_with_snapshot};
use crate::text::{read_optional, strip_jsonc};
use crate::{ActivationScheme, Error, GameId, Mod, ModLocation, Result, normalize_mod_id};
use profile_file::ProfileFile;
pub(crate) use profiles::validate_name as validate_profile_name;
pub use profiles::{
    delete_profile, export_profile, list_profiles, save_profile, set_in_profile, switch_profile,
};

const PROFILE_POINTER: &str = "profile.txt";
const LEGACY_PROFILE: &str = "active-profile.txt";
const ACTIVATION_JSON: &str = "activation.json";
const PROFILES_DIR: &str = "profiles";
const ACTIVE_PROFILE_KEY: &str = "ActiveProfile";
const DISABLED_IDS_KEY: &str = "disabledIds";
const DEFAULT_PROFILE: &str = "default";
const VANILLA_PROFILE: &str = "vanilla";

#[derive(Debug, Clone)]
enum Rules {
    AllOn,
    AllOff,
    File(ProfileFile),
    DisabledIds(HashSet<String>),
    /// Humankind's ordered Registry.xml list, as short names; listed means on.
    RegistryList(Vec<String>),
}

#[derive(Debug, Clone)]
pub struct ActivationState {
    pub scheme: ActivationScheme,
    active_profile: Option<String>,
    rules: Rules,
}

impl ActivationState {
    /// `folder` matters for 1800, where a leading `-` disables the mod.
    pub fn is_active(&self, mod_id: &str, folder: &str) -> bool {
        match &self.rules {
            Rules::AllOn => true,
            Rules::AllOff => false,
            Rules::File(file) => file.is_active(mod_id),
            Rules::DisabledIds(disabled) => !disabled.contains(mod_id) && !folder.starts_with('-'),
            Rules::RegistryList(modules) => modules.iter().any(|module| module == mod_id),
        }
    }

    /// Humankind's load order, base game first; `None` for the Anno schemes.
    pub fn registry_modules(&self) -> Option<&[String]> {
        match &self.rules {
            Rules::RegistryList(modules) => Some(modules),
            _ => None,
        }
    }

    pub fn active_profile(&self) -> Option<&str> {
        self.active_profile.as_deref()
    }
}

/// 117 uses the profile scheme unless only the legacy file is present.
pub fn detect_scheme(game: GameId, mods_dir: &Path) -> ActivationScheme {
    match game {
        GameId::Anno1800 => ActivationScheme::ActivationJson,
        GameId::Anno117
            if !mods_dir.join(PROFILE_POINTER).exists()
                && mods_dir.join(LEGACY_PROFILE).exists() =>
        {
            ActivationScheme::Legacy
        }
        GameId::Anno117 => ActivationScheme::Profile,
        GameId::Humankind => ActivationScheme::RegistryList,
    }
}

pub fn read(game: GameId, mods_dir: &Path) -> Result<ActivationState> {
    let scheme = detect_scheme(game, mods_dir);
    let (active_profile, rules) = match scheme {
        ActivationScheme::ActivationJson => {
            (None, Rules::DisabledIds(read_disabled_ids(mods_dir)?))
        }
        ActivationScheme::Legacy => {
            let text = read_optional(&mods_dir.join(LEGACY_PROFILE))?.unwrap_or_default();
            (None, Rules::File(ProfileFile::parse(&text)))
        }
        ActivationScheme::Profile => {
            let name = read_active_profile_name(mods_dir)?;
            let effective = name.as_deref().unwrap_or(DEFAULT_PROFILE);
            (name.clone(), profile_rules(mods_dir, effective)?)
        }
        ActivationScheme::RegistryList => (
            None,
            Rules::RegistryList(humankind::order::read(humankind_dir(mods_dir))?),
        ),
    };
    Ok(ActivationState {
        scheme,
        active_profile,
        rules,
    })
}

/// Sets `active` on every mod; sub-mods follow their parent.
pub fn apply(state: &ActivationState, mods: &mut [Mod]) {
    let mut parents: HashMap<(ModLocation, String), bool> = HashMap::new();
    for target in mods.iter_mut().filter(|m| m.parent_id.is_none()) {
        target.active = maps::is_map(target) || state.is_active(&target.id, &target.folder);
        parents.insert((target.location, target.id.clone()), target.active);
    }
    for target in mods.iter_mut() {
        if let Some(parent) = &target.parent_id {
            target.active = parents
                .get(&(target.location, parent.clone()))
                .copied()
                .unwrap_or(false);
        }
    }
}

/// Writes only inside the Documents folder, snapshotting the file it changes first.
pub fn set_active(
    game: GameId,
    writable: &ModsFolder,
    target: &Mod,
    active: bool,
    installed: &[Mod],
    snapshots: &Snapshots,
) -> Result<()> {
    ensure_writable(writable)?;
    if target.parent_id.is_some() {
        return Err(Error::SubMod(target.id.clone()));
    }
    if maps::is_map(target) {
        return Err(Error::MapAlwaysOn(target.id.clone()));
    }
    let dir = &writable.path;
    match detect_scheme(game, dir) {
        ActivationScheme::ActivationJson => set_active_json(writable, target, active, snapshots),
        ActivationScheme::Legacy => edit_profile_file(
            &dir.join(LEGACY_PROFILE),
            true,
            |file| file.set(&target.id, active),
            snapshots,
        ),
        ActivationScheme::Profile => {
            let name = read_active_profile_name(dir)?.unwrap_or_else(|| DEFAULT_PROFILE.into());
            let path = find_profile(dir, &name)?
                .unwrap_or_else(|| dir.join(PROFILES_DIR).join(format!("{name}.txt")));
            let enable_new = !name.eq_ignore_ascii_case(VANILLA_PROFILE);
            edit_profile_file(
                &path,
                enable_new,
                |file| file.set(&target.id, active),
                snapshots,
            )
        }
        ActivationScheme::RegistryList => {
            set_in_registry(dir, target, active, installed, snapshots)
        }
    }
}

/// `mods_dir` is `Documents/Humankind/Community`; the registry sits beside it under `Users/`.
fn humankind_dir(mods_dir: &Path) -> &Path {
    mods_dir.parent().unwrap_or(mods_dir)
}

/// On appends to the end of the load order; off drops every entry naming the module.
fn set_in_registry(
    mods_dir: &Path,
    target: &Mod,
    active: bool,
    installed: &[Mod],
    snapshots: &Snapshots,
) -> Result<()> {
    let game_dir = humankind_dir(mods_dir);
    if !active {
        return humankind::order::remove(game_dir, &target.id, snapshots).map(drop);
    }
    let current = humankind::order::read(game_dir)?;
    // Through the load-order rules, so a second Standalone or Conversion replaces the first.
    let types = humankind::load_order::module_types(installed);
    let modules = humankind::load_order::enable(&current, &target.id, &types);
    if modules == current {
        return Ok(());
    }
    humankind::order::write(game_dir, &modules, installed, snapshots).map(drop)
}

fn edit_profile_file(
    path: &Path,
    enable_new_mods_if_created: bool,
    edit: impl FnOnce(&mut ProfileFile),
    snapshots: &Snapshots,
) -> Result<()> {
    let mut file = match read_optional(path)? {
        Some(text) => ProfileFile::parse(&text),
        None => ProfileFile::new(enable_new_mods_if_created),
    };
    edit(&mut file);
    write_with_snapshot(path, &file.render(), snapshots)
}

fn set_active_json(
    writable: &ModsFolder,
    target: &Mod,
    active: bool,
    snapshots: &Snapshots,
) -> Result<()> {
    if active && target.location == ModLocation::Documents && target.folder.starts_with('-') {
        rename_enabled(&writable.path, &target.folder)?;
    }
    let path = writable.path.join(ACTIVATION_JSON);
    let mut root = read_json_object(&path)?;
    let mut disabled: Vec<String> = root
        .get(DISABLED_IDS_KEY)
        .map(string_list)
        .unwrap_or_default();
    disabled.retain(|id| *id != target.id);
    if !active {
        disabled.push(target.id.clone());
    }
    root.insert(DISABLED_IDS_KEY.into(), Value::from(disabled));
    let text = serde_json::to_string_pretty(&Value::Object(root))?;
    write_with_snapshot(&path, &text, snapshots)
}

/// 1800's loader skips `-Name` folders, so enabling one drops the prefix.
fn rename_enabled(mods_dir: &Path, folder: &str) -> Result<()> {
    let from = mods_dir.join(folder);
    let to = mods_dir.join(folder.trim_start_matches('-'));
    if to.exists() {
        let clash = io::Error::new(
            io::ErrorKind::AlreadyExists,
            "enabled folder already exists",
        );
        return Err(io_at(&to)(clash));
    }
    fs::rename(&from, &to).map_err(io_at(&from))
}

fn read_disabled_ids(mods_dir: &Path) -> Result<HashSet<String>> {
    let root = read_json_object(&mods_dir.join(ACTIVATION_JSON))?;
    Ok(root
        .get(DISABLED_IDS_KEY)
        .map(string_list)
        .unwrap_or_default()
        .into_iter()
        .collect())
}

/// Missing file → empty object, so unknown keys survive a rewrite.
fn read_json_object(path: &Path) -> Result<Map<String, Value>> {
    match read_optional(path)? {
        Some(text) => match serde_json::from_str(&strip_jsonc(&text))? {
            Value::Object(map) => Ok(map),
            _ => Ok(Map::new()),
        },
        None => Ok(Map::new()),
    }
}

fn string_list(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(normalize_mod_id)
                .collect()
        })
        .unwrap_or_default()
}

fn profile_rules(mods_dir: &Path, name: &str) -> Result<Rules> {
    if let Some(path) = find_profile(mods_dir, name)?
        && let Some(text) = read_optional(&path)?
    {
        return Ok(Rules::File(ProfileFile::parse(&text)));
    }
    // A missing (or ghost) profile loads everything, except the built-in vanilla one.
    Ok(if name.eq_ignore_ascii_case(VANILLA_PROFILE) {
        Rules::AllOff
    } else {
        Rules::AllOn
    })
}

fn read_active_profile_name(mods_dir: &Path) -> Result<Option<String>> {
    let text = read_optional(&mods_dir.join(PROFILE_POINTER))?.unwrap_or_default();
    Ok(text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(ACTIVE_PROFILE_KEY)?;
        let name = rest.trim();
        // The name becomes a file path, so one that fails the save-time rules (`../x`) is ignored.
        (rest.starts_with(char::is_whitespace) && profiles::validate_name(name).is_ok())
            .then(|| name.to_string())
    }))
}

/// NTFS ignores case for every letter, not just ASCII, so `Été` and `été` are one file.
pub fn same_profile_name(left: &str, right: &str) -> bool {
    left.to_lowercase() == right.to_lowercase()
}

/// Profile file names are matched case-insensitively, as on Windows.
fn find_profile(mods_dir: &Path, name: &str) -> Result<Option<PathBuf>> {
    Ok(profile_files(mods_dir)?
        .into_iter()
        .find(|(stem, _)| same_profile_name(stem, name))
        .map(|(_, path)| path))
}

fn profile_files(mods_dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let dir = mods_dir.join(PROFILES_DIR);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).map_err(io_at(&dir))? {
        let path = entry.map_err(io_at(&dir))?.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
            && let Some(stem) = path.file_stem()
        {
            files.push((stem.to_string_lossy().into_owned(), path));
        }
    }
    files.sort();
    Ok(files)
}
