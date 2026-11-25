//! Environment checks shown on the Health screen, plus the mod-loader log parser.

pub mod humankind;
pub mod log;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::activation::detect_scheme;
use crate::{ActivationScheme, GameId, HealthCheck, Mod, Severity};

const WRITE_PROBE: &str = ".atelier-write-probe";

pub struct Environment<'a> {
    pub game: GameId,
    pub install_path: Option<&'a Path>,
    pub mods_dir: &'a Path,
    pub mods: &'a [Mod],
}

pub fn check_environment(env: &Environment) -> Vec<HealthCheck> {
    let mut checks = vec![game_found(env)];
    match env.game {
        GameId::Anno1800 => {
            checks.push(writable("mods-writable", "Mods folder", env.mods_dir));
            checks.push(legacy_loader_check(env.install_path));
        }
        GameId::Anno117 => checks.push(writable("mods-writable", "Mods folder", env.mods_dir)),
        // Community/ only exists after the first install, so its parent is what gets probed.
        GameId::Humankind => checks.extend(humankind::checks(env)),
    }
    checks.push(duplicate_ids(env.mods));
    checks.push(activation_scheme(env.game, env.mods_dir));
    checks
}

fn check(id: &str, level: Severity, title: &str, detail: String) -> HealthCheck {
    HealthCheck {
        id: id.to_string(),
        level,
        title: title.to_string(),
        detail,
    }
}

fn game_found(env: &Environment) -> HealthCheck {
    match env.install_path {
        Some(install) if install.join(env.game.executable()).is_file() => check(
            "game-found",
            Severity::Ok,
            "Game found",
            install.display().to_string(),
        ),
        _ => check(
            "game-found",
            Severity::Error,
            "Game not found",
            format!("No {} install was detected", env.game.display_name()),
        ),
    }
}

/// Creates and removes a probe file, so permissions are tested rather than guessed.
fn writable(id: &str, what: &str, dir: &Path) -> HealthCheck {
    let probe = dir.join(WRITE_PROBE);
    match fs::write(&probe, b"") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            check(
                id,
                Severity::Ok,
                &format!("{what} is writable"),
                dir.display().to_string(),
            )
        }
        Err(error) => check(
            id,
            Severity::Warning,
            &format!("{what} is not writable"),
            format!("{}: {error}", dir.display()),
        ),
    }
}

/// The old community loader's installer renames the game's own `python35.dll` to this
/// and puts itself in its place.
pub const LEGACY_LOADER_ORIGINAL: &str = "python35_ubi.dll";
pub const LEGACY_LOADER_DLL: &str = "python35.dll";
/// The loader's version-info FileDescription; the game's own DLL says "Python Core".
const LEGACY_LOADER_MARKER: &str = "Anno 1800 Mod Loader";

/// The game's Python DLL folder, where the old 1800 loader installs.
pub fn loader_dir(install: &Path) -> std::path::PathBuf {
    install.join("Bin").join("Win64")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyLoader {
    Absent,
    /// The game's own DLL is beside it as `python35_ubi.dll`, so it can go back.
    Removable,
    /// The renamed original is gone; only verifying the game files restores it.
    Stranded,
}

/// Reads python35.dll, so the cheap [`legacy_loader_removable`] serves the game list.
/// A leftover original beside the game's own DLL (Steam restored it) is not the loader.
pub fn legacy_loader(install: &Path) -> LegacyLoader {
    let dir = loader_dir(install);
    let has_original = dir.join(LEGACY_LOADER_ORIGINAL).is_file();
    let is_loader = match fs::read(dir.join(LEGACY_LOADER_DLL)) {
        Ok(bytes) => contains_utf16(&bytes, LEGACY_LOADER_MARKER),
        // Deleted by hand: the game has no python35.dll until the original goes back.
        Err(error) => error.kind() == std::io::ErrorKind::NotFound && has_original,
    };
    match (is_loader, has_original) {
        (true, true) => LegacyLoader::Removable,
        (true, false) => LegacyLoader::Stranded,
        (false, _) => LegacyLoader::Absent,
    }
}

/// Only an install with the renamed original can be removable, so most never read the DLL.
pub fn legacy_loader_removable(install: &Path) -> bool {
    loader_dir(install).join(LEGACY_LOADER_ORIGINAL).is_file()
        && legacy_loader(install) == LegacyLoader::Removable
}

/// Windows version resources store their strings as UTF-16LE.
fn contains_utf16(haystack: &[u8], needle: &str) -> bool {
    let needle: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

fn legacy_loader_check(install: Option<&Path>) -> HealthCheck {
    let detail = match install.map(legacy_loader) {
        Some(LegacyLoader::Removable) => {
            "It no longer matches the game and stops it at start. Removing it puts the game's own python35.dll back, and the built-in mod loader takes over."
        }
        Some(LegacyLoader::Stranded) => {
            "It no longer matches the game, and the game's own python35.dll it replaced is gone. Verify the game files in Steam or Ubisoft Connect to put it back."
        }
        _ => return game_python_dll(install),
    };
    check(
        "legacy-mod-loader",
        Severity::Error,
        "Old Anno 1800 Mod Loader installed",
        detail.to_string(),
    )
}

/// With no loader in the way, the game still needs its own python35.dll to start.
fn game_python_dll(install: Option<&Path>) -> HealthCheck {
    let missing = install.is_some_and(|path| {
        let dir = loader_dir(path);
        dir.join(GameId::Anno1800.executable_name()).is_file()
            && !dir.join(LEGACY_LOADER_DLL).exists()
    });
    if missing {
        check(
            "legacy-mod-loader",
            Severity::Error,
            "Game file python35.dll is missing",
            "Anno 1800 can't start without it. Verify the game files in Steam or Ubisoft Connect to put it back.".to_string(),
        )
    } else {
        check(
            "legacy-mod-loader",
            Severity::Ok,
            "No old mod loader",
            String::new(),
        )
    }
}

fn duplicate_ids(mods: &[Mod]) -> HealthCheck {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for target in mods {
        *counts.entry(target.id.as_str()).or_default() += 1;
    }
    let mut seen = HashSet::new();
    let duplicates: Vec<&str> = mods
        .iter()
        .map(|m| m.id.as_str())
        .filter(|id| counts[id] > 1 && seen.insert(*id))
        .collect();
    if duplicates.is_empty() {
        check(
            "duplicate-ids",
            Severity::Ok,
            "No duplicate mods",
            String::new(),
        )
    } else {
        check(
            "duplicate-ids",
            Severity::Warning,
            "Mods installed more than once",
            duplicates.join(", "),
        )
    }
}

fn activation_scheme(game: GameId, mods_dir: &Path) -> HealthCheck {
    match detect_scheme(game, mods_dir) {
        ActivationScheme::Legacy => check(
            "activation-scheme",
            Severity::Warning,
            "Legacy activation file",
            "active-profile.txt is the pre-profiles format; switching a profile upgrades it".into(),
        ),
        scheme => check(
            "activation-scheme",
            Severity::Ok,
            "Activation file recognised",
            format!("{scheme:?}"),
        ),
    }
}
