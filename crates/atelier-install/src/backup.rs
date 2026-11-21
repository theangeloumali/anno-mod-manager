//! Backups live at `<backup_dir>/<modId>/<version>-<unix millis>/`; the millis order them.

use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use atelier_core::GameId;
use atelier_core::humankind::module_name::ModuleName;
use serde::Serialize;

use crate::error::io_at;
use crate::fsops::{child_dirs, file_name, move_dir};
use crate::install::{identify, new_folder_name, target_folder};
use crate::{Error, Result};

const MILLIS_PER_DAY: u64 = 86_400_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupVersion {
    /// As the folder names it: characters unsafe in a folder name read as `_`.
    pub version: String,
    /// Unix millis.
    pub backed_up_at: u64,
}

/// A backup pruning must never delete, such as a version a save was played with.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeptVersion {
    pub game: GameId,
    pub mod_id: String,
    pub version: String,
}

/// A fresh backup slot for `dir`, always newer than any existing backup of the same mod.
pub(crate) fn backup_path(backup_dir: &Path, dir: &Path) -> Result<PathBuf> {
    let (mod_id, version) = identity(dir);
    let mod_backups = backup_dir.join(new_folder_name(&mod_id));
    let newest = list_backups(&mod_backups)?
        .last()
        .map(|(millis, _)| millis + 1);
    let millis = newest.map_or(now_millis(), |next| next.max(now_millis()));
    Ok(mod_backups.join(format!("{}-{millis}", safe_version(&version))))
}

/// Puts the newest backup of `mod_id` back, first backing up the copy it replaces.
pub(crate) fn rollback(backup_dir: &Path, target: &Path, mod_id: &str) -> Result<()> {
    check_id(mod_id)?;
    let dest = target.join(target_folder(target, mod_id)?);
    restore(backup_dir, mod_id, None, &dest)
}

/// Puts the newest backup of `mod_id` at `version` back, into `installed` when the mod has a
/// copy there (it must sit inside `target`), else into the mod's folder in `target`.
pub(crate) fn restore_version(
    backup_dir: &Path,
    target: &Path,
    mod_id: &str,
    version: &str,
    installed: Option<&Path>,
) -> Result<()> {
    check_id(mod_id)?;
    let dest = match installed {
        Some(dir) if is_inside(target, dir) => dir.to_path_buf(),
        Some(dir) => return Err(Error::UnsafeTarget(dir.to_path_buf())),
        None => target.join(target_folder(target, mod_id)?),
    };
    restore(backup_dir, mod_id, Some(version), &dest)
}

/// Newest first; an id that isn't a plain folder name has no backups.
pub(crate) fn list_versions(backup_dir: &Path, mod_id: &str) -> Result<Vec<BackupVersion>> {
    if check_id(mod_id).is_err() {
        return Ok(Vec::new());
    }
    Ok(list_backups(&backup_dir.join(new_folder_name(mod_id)))?
        .into_iter()
        .rev()
        .map(|(backed_up_at, dir)| BackupVersion {
            version: backup_version(&dir),
            backed_up_at,
        })
        .collect())
}

/// Deletes backups older than the retention window, always keeping each mod's newest and
/// every `(mod id, version)` in `kept`.
pub(crate) fn prune(
    backup_dir: &Path,
    retention_days: Option<u32>,
    kept: &HashSet<(String, String)>,
) -> Result<()> {
    let Some(days) = retention_days else {
        return Ok(());
    };
    if !backup_dir.is_dir() {
        return Ok(());
    }
    let kept_slots: HashSet<(String, String)> = kept
        .iter()
        .map(|(mod_id, version)| (new_folder_name(mod_id), safe_version(version)))
        .collect();
    let cutoff = now_millis().saturating_sub(u64::from(days) * MILLIS_PER_DAY);
    for mod_backups in child_dirs(backup_dir)? {
        let folder = file_name(&mod_backups);
        let mut backups = list_backups(&mod_backups)?;
        backups.pop();
        for (_, expired) in backups.into_iter().filter(|(millis, dir)| {
            *millis < cutoff && !kept_slots.contains(&(folder.clone(), backup_version(dir)))
        }) {
            fs::remove_dir_all(&expired).map_err(io_at(&expired))?;
        }
    }
    Ok(())
}

/// Moves the chosen backup to `dest`, first backing up whatever is there.
fn restore(backup_dir: &Path, mod_id: &str, version: Option<&str>, dest: &Path) -> Result<()> {
    let wanted = version.map(safe_version);
    let (_, chosen) = list_backups(&backup_dir.join(new_folder_name(mod_id)))?
        .into_iter()
        .rev()
        .find(|(_, dir)| {
            wanted
                .as_deref()
                .is_none_or(|wanted| backup_version(dir) == wanted)
        })
        .ok_or_else(|| {
            Error::NoBackup(version.map_or(mod_id.to_string(), |v| format!("{mod_id} {v}")))
        })?;
    let displaced = if dest.exists() {
        let slot = backup_path(backup_dir, dest)?;
        move_dir(dest, &slot)?;
        Some(slot)
    } else {
        None
    };
    if let Err(error) = move_dir(&chosen, dest) {
        if let Some(slot) = displaced {
            move_dir(&slot, dest)?;
        }
        return Err(error);
    }
    Ok(())
}

/// The id comes from the UI; a single plain path component can't climb out of backup_dir.
fn check_id(mod_id: &str) -> Result<()> {
    let mut parts = Path::new(mod_id).components();
    let plain = matches!(parts.next(), Some(Component::Normal(_)))
        && parts.next().is_none()
        && !mod_id.contains(['/', '\\', ':']);
    if plain {
        Ok(())
    } else {
        Err(Error::NoBackup(mod_id.to_string()))
    }
}

fn is_inside(target: &Path, dir: &Path) -> bool {
    dir != target
        && dir.starts_with(target)
        && !dir
            .components()
            .any(|part| matches!(part, Component::ParentDir))
}

/// Humankind folders have no modinfo: their `<Name>.<guid>.<major>.<minor>` file names them.
fn identity(dir: &Path) -> (String, String) {
    let module = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .find_map(|entry| ModuleName::parse(&file_name(&entry.path())));
    match module {
        Some(module) => (module.short_name(), module.version()),
        None => identify(dir),
    }
}

/// Backups sorted oldest first; folders that don't follow the naming scheme are left alone.
fn list_backups(mod_backups: &Path) -> Result<Vec<(u64, PathBuf)>> {
    if !mod_backups.is_dir() {
        return Ok(Vec::new());
    }
    let mut backups: Vec<(u64, PathBuf)> = child_dirs(mod_backups)?
        .into_iter()
        .filter_map(|dir| backup_millis(&file_name(&dir)).map(|millis| (millis, dir)))
        .collect();
    backups.sort();
    Ok(backups)
}

fn backup_millis(name: &str) -> Option<u64> {
    name.rsplit_once('-')?.1.parse().ok()
}

/// `safe_version` maps `-` to `_`, so the last `-` always ends the version.
fn backup_version(dir: &Path) -> String {
    let name = file_name(dir);
    name.rsplit_once('-')
        .map_or(name.clone(), |(version, _)| version.to_string())
}

fn safe_version(version: &str) -> String {
    version
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '+' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn now_millis() -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}
