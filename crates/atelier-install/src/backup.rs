//! Backups live at `<backup_dir>/<modId>/<version>-<unix millis>/`; the millis order them.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use atelier_core::normalize_mod_id;

use crate::error::io_at;
use crate::fsops::{child_dirs, file_name, move_dir};
use crate::install::{existing_folder, identify, new_folder_name};
use crate::{Error, Result};

const MILLIS_PER_DAY: u64 = 86_400_000;

/// A fresh backup slot for `dir`, always newer than any existing backup of the same mod.
pub(crate) fn backup_path(backup_dir: &Path, dir: &Path) -> Result<PathBuf> {
    let (mod_id, version) = identify(dir);
    let mod_backups = backup_dir.join(new_folder_name(&mod_id));
    let newest = list_backups(&mod_backups)?
        .last()
        .map(|(millis, _)| millis + 1);
    let millis = newest.map_or(now_millis(), |next| next.max(now_millis()));
    Ok(mod_backups.join(format!("{}-{millis}", safe_version(&version))))
}

/// Puts the newest backup of `mod_id` back, first backing up the copy it replaces.
pub(crate) fn rollback(backup_dir: &Path, target: &Path, mod_id: &str) -> Result<()> {
    // The id comes from the UI; a normalised id can't climb out of backup_dir.
    if mod_id.is_empty() || normalize_mod_id(mod_id) != mod_id {
        return Err(Error::NoBackup(mod_id.to_string()));
    }
    let (_, newest) = list_backups(&backup_dir.join(new_folder_name(mod_id)))?
        .pop()
        .ok_or_else(|| Error::NoBackup(mod_id.to_string()))?;
    let folder = existing_folder(target, mod_id)?.unwrap_or_else(|| new_folder_name(mod_id));
    let dest = target.join(folder);
    let displaced = if dest.exists() {
        let slot = backup_path(backup_dir, &dest)?;
        move_dir(&dest, &slot)?;
        Some(slot)
    } else {
        None
    };
    if let Err(error) = move_dir(&newest, &dest) {
        if let Some(slot) = displaced {
            move_dir(&slot, &dest)?;
        }
        return Err(error);
    }
    Ok(())
}

/// Deletes backups older than the retention window, always keeping each mod's newest.
pub(crate) fn prune(backup_dir: &Path, retention_days: Option<u32>) -> Result<()> {
    let Some(days) = retention_days else {
        return Ok(());
    };
    if !backup_dir.is_dir() {
        return Ok(());
    }
    let cutoff = now_millis().saturating_sub(u64::from(days) * MILLIS_PER_DAY);
    for mod_backups in child_dirs(backup_dir)? {
        let mut backups = list_backups(&mod_backups)?;
        backups.pop();
        for (_, expired) in backups.into_iter().filter(|(millis, _)| *millis < cutoff) {
            fs::remove_dir_all(&expired).map_err(io_at(&expired))?;
        }
    }
    Ok(())
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
