//! Humankind health: the mod-list wipe cause, duplicate guids, late-game rendering setup, BepInEx.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{Environment, check, writable};
use crate::games::{Probe, mod_folders, vdf_values};
use crate::humankind::maps::folder_items;
use crate::humankind::module_name::ModuleName;
use crate::humankind::{detect, load_order, registry};
use crate::scan::{ModsFolder, mod_dir};
use crate::{GameId, HealthCheck, Mod, ModLocation, Severity};

const PAGEFILE_TARGET_MB: u64 = 32 * 1024;
const PAGEFILE_ADVICE: &str = "Set a custom pagefile of 32 GB or more in Windows (System, Advanced, Performance, Virtual memory). Players report it helps against the late-game flat-tile bug on Huge maps.";
const FORCE_VULKAN: &str = "-force-vulkan";
const MODLISTGUARD_DLL: &str = "ModListGuard.dll";

pub(super) fn checks(env: &Environment) -> Vec<HealthCheck> {
    // The mods folder is `<Documents>/Humankind/Community`.
    let Some(game_dir) = env.mods_dir.parent() else {
        return Vec::new();
    };
    let mut checks = vec![writable(
        "documents-writable",
        r"Documents\Humankind",
        game_dir,
    )];
    checks.extend(listed_modules(game_dir, env.mods));
    if let Some(documents) = game_dir.parent() {
        let folders = mod_folders(GameId::Humankind, env.install_path, documents);
        checks.push(duplicate_guids(&installed_modules(&folders, env.mods)));
    }
    checks.extend(paging_files().map(|entries| pagefile(&entries)));
    checks.extend(force_vulkan(&steam_user_configs(&steam_roots(
        env.install_path,
    ))));
    if let Some(install) = env.install_path {
        checks.push(bepinex(install));
        checks.push(modlistguard(install));
    }
    checks
}

/// No Registry.xml yet means the game never ran, so there is no list to check.
fn listed_modules(game_dir: &Path, installed: &[Mod]) -> Option<HealthCheck> {
    let listed = detect::registry_path(game_dir)
        .and_then(|path| path.map(|path| registry::read_modules(&path)).transpose());
    match listed {
        Ok(listed) => listed.map(|listed| missing_modules(&listed, installed)),
        Err(error) => Some(check(
            "missing-modules",
            Severity::Warning,
            "Couldn't read the mod list",
            error.to_string(),
        )),
    }
}

/// A listed module that isn't installed makes the game reset the whole list at launch.
pub fn missing_modules(listed: &[String], installed: &[Mod]) -> HealthCheck {
    let missing = load_order::missing(listed, installed);
    if missing.is_empty() {
        check(
            "missing-modules",
            Severity::Ok,
            "Every listed module is installed",
            String::new(),
        )
    } else {
        check(
            "missing-modules",
            Severity::Error,
            "The mod list names modules that aren't installed",
            format!(
                "At launch the game resets the mod list to the base game. Missing: {}",
                missing.join(", ")
            ),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledModule {
    pub module: ModuleName,
    pub location: ModLocation,
    pub folder: String,
}

/// The long name each scanned mod came from, read back from its folder.
pub fn installed_modules(folders: &[ModsFolder], mods: &[Mod]) -> Vec<InstalledModule> {
    mods.iter()
        .filter_map(|item| {
            let files: Vec<String> = fs::read_dir(mod_dir(folders, item)?)
                .ok()?
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            let module = folder_items(&files)
                .modules
                .into_iter()
                .map(|(module, _)| module)
                .find(|module| {
                    module.short_name() == item.id && module.version() == item.version
                })?;
            Some(InstalledModule {
                module,
                location: item.location,
                folder: item.folder.clone(),
            })
        })
        .collect()
}

/// A renamed copy keeps its guid, so the short-name check alone misses it.
pub fn duplicate_guids(modules: &[InstalledModule]) -> HealthCheck {
    let mut by_guid: BTreeMap<String, Vec<&InstalledModule>> = BTreeMap::new();
    // A plain-named module has no guid, so the short-name check alone covers it.
    for installed in modules {
        if let Some(guid) = installed.module.guid() {
            by_guid
                .entry(guid.to_lowercase())
                .or_default()
                .push(installed);
        }
    }
    let duplicates: Vec<String> = by_guid
        .iter()
        .filter(|(_, copies)| copies.len() > 1)
        .map(|(guid, copies)| {
            let places: Vec<String> = copies
                .iter()
                .map(|copy| {
                    format!(
                        "{} in {:?}/{}",
                        copy.module.name, copy.location, copy.folder
                    )
                })
                .collect();
            format!("{guid}: {}", places.join(", "))
        })
        .collect();
    if duplicates.is_empty() {
        check(
            "duplicate-guids",
            Severity::Ok,
            "No module installed twice",
            String::new(),
        )
    } else {
        check(
            "duplicate-guids",
            Severity::Warning,
            "The same module is installed in more than one place",
            duplicates.join("; "),
        )
    }
}

/// `PagingFiles` entries: `?:\pagefile.sys` or `C:\pagefile.sys 0 0` is system-managed,
/// `C:\pagefile.sys <initial MB> <max MB>` is fixed.
pub fn pagefile(entries: &[String]) -> HealthCheck {
    let mut system_managed = false;
    let mut fixed_mb = 0;
    let mut any = false;
    for entry in entries.iter().filter(|entry| !entry.trim().is_empty()) {
        any = true;
        let mut tokens = entry.split_whitespace();
        let path = tokens.next().unwrap_or_default();
        let sizes: Vec<u64> = tokens.filter_map(|token| token.parse().ok()).collect();
        if path.starts_with("?:") || sizes.iter().all(|size| *size == 0) {
            system_managed = true;
        } else {
            fixed_mb += sizes.last().copied().unwrap_or_default();
        }
    }
    let gigabytes = |mb: u64| format!("{} GB", (mb + 512) / 1024);
    let (level, title, detail) = if !any {
        (
            Severity::Warning,
            "No pagefile".to_string(),
            PAGEFILE_ADVICE.to_string(),
        )
    } else if fixed_mb >= PAGEFILE_TARGET_MB {
        (
            Severity::Ok,
            format!("Pagefile is {}", gigabytes(fixed_mb)),
            String::new(),
        )
    } else if system_managed {
        (
            Severity::Warning,
            "Pagefile is system-managed".to_string(),
            PAGEFILE_ADVICE.to_string(),
        )
    } else {
        (
            Severity::Warning,
            "Pagefile is under 32 GB".to_string(),
            format!("{} fixed. {PAGEFILE_ADVICE}", gigabytes(fixed_mb)),
        )
    };
    check("pagefile", level, &title, detail)
}

#[cfg(windows)]
fn paging_files() -> Option<Vec<String>> {
    use winreg::RegKey;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management")
        .ok()?
        .get_value("PagingFiles")
        .ok()
}

/// Only Windows has the registry value; elsewhere the row is left out rather than guessed.
#[cfg(not(windows))]
fn paging_files() -> Option<Vec<String>> {
    None
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamUserConfig {
    pub user: String,
    pub localconfig: String,
}

/// Launch options set in the app's own block of a `localconfig.vdf`.
pub fn launch_options(localconfig: &str, app_id: u32) -> Vec<String> {
    let key = format!("\"{app_id}\"");
    let mut options = Vec::new();
    let mut rest = localconfig;
    while let Some(at) = rest.find(&key) {
        rest = &rest[at + key.len()..];
        if let Some(block) = rest.trim_start().strip_prefix('{').and_then(block_body) {
            options.extend(vdf_values(block, "LaunchOptions"));
        }
    }
    options
}

/// The text before the `}` that closes an opened block; braces inside quotes don't count.
fn block_body(text: &str) -> Option<&str> {
    let (mut depth, mut quoted, mut escaped) = (0usize, false, false);
    for (index, character) in text.char_indices() {
        if quoted {
            match character {
                _ if escaped => escaped = false,
                '\\' => escaped = true,
                '"' => quoted = false,
                _ => {}
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '{' => depth += 1,
            '}' if depth == 0 => return Some(&text[..index]),
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// None when no Steam user's config could be read: skipped, not reported as fine.
pub fn force_vulkan(configs: &[SteamUserConfig]) -> Option<HealthCheck> {
    if configs.is_empty() {
        return None;
    }
    let users: Vec<&str> = configs
        .iter()
        .filter(|config| {
            launch_options(&config.localconfig, GameId::Humankind.steam_app_id())
                .iter()
                .any(|options| {
                    options
                        .split_whitespace()
                        .any(|option| option.eq_ignore_ascii_case(FORCE_VULKAN))
                })
        })
        .map(|config| config.user.as_str())
        .collect();
    Some(if users.is_empty() {
        check(
            "force-vulkan",
            Severity::Ok,
            "No -force-vulkan launch option",
            String::new(),
        )
    } else {
        check(
            "force-vulkan",
            Severity::Warning,
            "Steam starts Humankind with -force-vulkan",
            format!(
                "It causes missing harbor and wall graphics. Remove it from the game's launch options in Steam (Steam user {}).",
                users.join(", ")
            ),
        )
    })
}

/// `userdata/<id>/config/localconfig.vdf` of every Steam user, once per user id.
pub fn steam_user_configs(steam_roots: &[PathBuf]) -> Vec<SteamUserConfig> {
    let mut seen = HashSet::new();
    let mut configs = Vec::new();
    for root in steam_roots {
        let Ok(entries) = fs::read_dir(root.join("userdata")) else {
            continue;
        };
        let mut users: Vec<PathBuf> = entries
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();
        users.sort();
        for dir in users {
            let user = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let Ok(localconfig) = fs::read_to_string(dir.join("config").join("localconfig.vdf"))
            else {
                continue;
            };
            if seen.insert(user.clone()) {
                configs.push(SteamUserConfig { user, localconfig });
            }
        }
    }
    configs
}

/// The Steam install holds `userdata`; a game in the default library sits under it.
fn steam_roots(install: Option<&Path>) -> Vec<PathBuf> {
    let mut roots = Probe::system(GameId::Humankind).steam_libraries;
    // `<library>/steamapps/common/<game>`
    roots.extend(
        install
            .and_then(|path| path.ancestors().nth(3))
            .map(Path::to_path_buf),
    );
    roots
}

/// Doorstop's `winhttp.dll` loads BepInEx's core; both are needed for plugins to run.
pub fn bepinex_installed(install: &Path) -> bool {
    install.join("winhttp.dll").is_file()
        && install
            .join("BepInEx")
            .join("core")
            .join("BepInEx.dll")
            .is_file()
}

/// Where BepInEx loads plugins from; mods write here, so it is never the game folder.
pub fn plugins_dir(install: &Path) -> PathBuf {
    install.join("BepInEx").join("plugins")
}

pub fn modlistguard_installed(install: &Path) -> bool {
    contains_file(&plugins_dir(install), MODLISTGUARD_DLL)
}

/// Searches subfolders too, without following links.
fn contains_file(dir: &Path, name: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        let Ok(kind) = entry.file_type() else {
            return false;
        };
        if kind.is_dir() {
            contains_file(&entry.path(), name)
        } else {
            kind.is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(name)
        }
    })
}

pub fn bepinex(install: &Path) -> HealthCheck {
    if bepinex_installed(install) {
        check(
            "bepinex",
            Severity::Ok,
            "BepInEx is installed",
            install.join("BepInEx").display().to_string(),
        )
    } else {
        check(
            "bepinex",
            Severity::Ok,
            "BepInEx is not installed",
            "Optional. Plugins such as ModListGuard need it.".to_string(),
        )
    }
}

pub fn modlistguard(install: &Path) -> HealthCheck {
    let (title, detail) = if modlistguard_installed(install) {
        (
            "ModListGuard is installed",
            "It backs up the mod list before the game changes it; Ctrl+F11 on the main menu restores it.",
        )
    } else {
        (
            "ModListGuard is not installed",
            "Optional BepInEx plugin that backs up the mod list before the game changes it, so a reset can be undone.",
        )
    };
    check("modlistguard", Severity::Ok, title, detail.to_string())
}
