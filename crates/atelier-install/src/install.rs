//! Moves staged mods into the mods folder, backing up what they replace, all or nothing.

use std::fs;
use std::path::{Path, PathBuf};

use atelier_core::{modinfo, normalize_mod_id};

use crate::backup::backup_path;
use crate::error::io_at;
use crate::fsops::{child_dirs, file_name, move_dir};
use crate::{Error, InstallItemStatus, InstallPreviewItem, InstallResult, Result};

pub(crate) struct StagedItem {
    pub item: InstallPreviewItem,
    pub staged_dir: PathBuf,
}

/// One completed move, kept so a later failure can put everything back.
enum Step {
    BackedUp { original: PathBuf, backup: PathBuf },
    Placed { staged: PathBuf, dest: PathBuf },
}

pub(crate) fn install_all(
    items: &[StagedItem],
    target: &Path,
    backup_dir: &Path,
    include_downgrades: bool,
) -> Result<InstallResult> {
    fs::create_dir_all(target).map_err(io_at(target))?;
    let mut journal = Vec::new();
    let mut result = InstallResult::default();
    for staged in items {
        let item = &staged.item;
        if !is_eligible(item.status, include_downgrades) {
            result.skipped.push(item_label(item));
            continue;
        }
        if let Err(cause) = place(staged, target, backup_dir, &mut journal) {
            return Err(undo(journal, cause));
        }
        result.installed.push(item.mod_id.clone());
    }
    Ok(result)
}

fn is_eligible(status: InstallItemStatus, include_downgrades: bool) -> bool {
    match status {
        InstallItemStatus::New | InstallItemStatus::Update => true,
        InstallItemStatus::Downgrade => include_downgrades,
        InstallItemStatus::Same | InstallItemStatus::WrongGame | InstallItemStatus::Invalid => {
            false
        }
    }
}

fn item_label(item: &InstallPreviewItem) -> String {
    if item.mod_id.is_empty() {
        item.root_path.clone()
    } else {
        item.mod_id.clone()
    }
}

fn place(
    staged: &StagedItem,
    target: &Path,
    backup_dir: &Path,
    journal: &mut Vec<Step>,
) -> Result<()> {
    let mod_id = &staged.item.mod_id;
    let folder = target_folder(target, mod_id)?;
    let dest = target.join(&folder);
    // Defence in depth: never back up or replace anything but a direct child of the mods folder.
    if mod_id.is_empty()
        || matches!(folder.as_str(), "" | "." | "..")
        || dest.parent() != Some(target)
    {
        return Err(Error::UnsafeTarget(dest));
    }
    if dest.exists() {
        let backup = backup_path(backup_dir, &dest)?;
        move_dir(&dest, &backup)?;
        journal.push(Step::BackedUp {
            original: dest.clone(),
            backup,
        });
    }
    move_dir(&staged.staged_dir, &dest)?;
    journal.push(Step::Placed {
        staged: staged.staged_dir.clone(),
        dest,
    });
    Ok(())
}

/// Reverses the journal newest first; anything that won't move back is named for a manual check.
fn undo(journal: Vec<Step>, cause: Error) -> Error {
    let mut stuck = Vec::new();
    for step in journal.into_iter().rev() {
        let (from, to, was_backup) = match step {
            Step::Placed { staged, dest } => (dest, staged, false),
            Step::BackedUp { original, backup } => (backup, original, true),
        };
        if move_dir(&from, &to).is_err() {
            stuck.push(from);
        } else if let Some(mod_backups) = from.parent().filter(|_| was_backup) {
            // Removes the per-mod backup folder only if this emptied it.
            let _ = fs::remove_dir(mod_backups);
        }
    }
    if stuck.is_empty() {
        cause
    } else {
        Error::RestoreIncomplete {
            cause: cause.to_string(),
            paths: stuck,
        }
    }
}

/// The top-level folder in `target` whose mod has this ModID.
pub(crate) fn existing_folder(target: &Path, mod_id: &str) -> Result<Option<String>> {
    if !target.is_dir() {
        return Ok(None);
    }
    Ok(child_dirs(target)?
        .into_iter()
        .filter(|dir| !file_name(dir).starts_with('.'))
        .find(|dir| identify(dir).0 == mod_id)
        .map(|dir| file_name(&dir)))
}

/// ModID and version of an installed folder, as scan.rs would name it.
pub(crate) fn identify(dir: &Path) -> (String, String) {
    let info = modinfo::read_dir(dir).ok().flatten();
    let id = info
        .as_ref()
        .and_then(|info| info.mod_id.clone())
        .unwrap_or_else(|| normalize_mod_id(&file_name(dir)));
    let version = info
        .map(|info| info.version)
        .unwrap_or_else(|| "unknown".into());
    (id, version)
}

/// The mod's current folder (an update keeps the player's name), else the first free
/// `<id>`, `<id>-2`, … so a folder that holds a different mod is never displaced.
pub(crate) fn target_folder(target: &Path, mod_id: &str) -> Result<String> {
    if let Some(existing) = existing_folder(target, mod_id)? {
        return Ok(existing);
    }
    let base = new_folder_name(mod_id);
    let mut candidate = base.clone();
    let mut suffix = 2;
    while target.join(&candidate).exists() {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    Ok(candidate)
}

/// Normalised ModIDs may hold `*`, which Windows forbids in folder names.
pub(crate) fn new_folder_name(mod_id: &str) -> String {
    mod_id.replace('*', "_")
}
