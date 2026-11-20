//! Environment checks shown on the Health screen, plus the mod-loader log parser.

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
    let mut checks = vec![game_found(env), mods_writable(env.mods_dir)];
    if env.game == GameId::Anno1800 {
        checks.push(stale_python(env.install_path));
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
fn mods_writable(mods_dir: &Path) -> HealthCheck {
    let probe = mods_dir.join(WRITE_PROBE);
    match fs::write(&probe, b"") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            check(
                "mods-writable",
                Severity::Ok,
                "Mods folder is writable",
                mods_dir.display().to_string(),
            )
        }
        Err(error) => check(
            "mods-writable",
            Severity::Warning,
            "Mods folder is not writable",
            format!("{}: {error}", mods_dir.display()),
        ),
    }
}

/// The retired 1800 loader shipped as `python35.dll`; leaving it breaks the built-in loader.
fn stale_python(install: Option<&Path>) -> HealthCheck {
    let dll = install.map(|path| path.join("Bin").join("Win64").join("python35.dll"));
    match dll {
        Some(path) if path.is_file() => check(
            "stale-python35",
            Severity::Warning,
            "Old mod loader DLL found",
            format!("Delete {} to use the built-in mod loader", path.display()),
        ),
        _ => check(
            "stale-python35",
            Severity::Ok,
            "No old mod loader DLL",
            String::new(),
        ),
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
