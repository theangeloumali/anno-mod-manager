//! Moves staged mods into the mods folder, backing up what they replace, all or nothing.

use std::fs;
use std::path::{Component, Path, PathBuf};

use atelier_core::humankind::maps;
use atelier_core::{GameId, modinfo, normalize_mod_id};

use crate::backup::backup_path;
use crate::error::io_at;
use crate::fsops::{child_dirs, file_name, is_windows_safe_name, move_dir};
use crate::roots::{map_in, modules_in};
use crate::{Error, InstallItemStatus, InstallPreviewItem, InstallResult, Result};

/// Humankind's Community folder holds Atelier's installs here, apart from mod.io's own folder.
pub(crate) const ATELIER_FOLDER: &str = "Atelier";

pub(crate) struct StagedItem {
    pub item: InstallPreviewItem,
    pub staged_dir: PathBuf,
    /// The installed copy as the preview saw it, checked again at confirm.
    pub seen: Option<crate::classify::CopyStamp>,
    pub placement: Placement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Placement {
    /// A direct child of the mods folder, named by `target_folder`.
    ModsFolder,
    /// `<target>/Atelier/<folder>`, replacing the module's copy wherever it sits under Atelier.
    Module { folder: PathBuf },
    /// `<plugins_dir>/<folder>`, outside the mods folder.
    Plugin {
        plugins_dir: PathBuf,
        folder: String,
    },
}

/// One completed move, kept so a later failure can put everything back.
enum Step {
    BackedUp { original: PathBuf, backup: PathBuf },
    Placed { staged: PathBuf, dest: PathBuf },
}

pub(crate) fn install_all(
    game: GameId,
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
        if let Err(cause) = place(game, staged, target, backup_dir, &mut journal) {
            return Err(undo(journal, cause));
        }
        result.installed.push(item.mod_id.clone());
    }
    Ok(result)
}

pub(crate) fn is_eligible(status: InstallItemStatus, include_downgrades: bool) -> bool {
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
    game: GameId,
    staged: &StagedItem,
    target: &Path,
    backup_dir: &Path,
    journal: &mut Vec<Step>,
) -> Result<()> {
    let mod_id = &staged.item.mod_id;
    match &staged.placement {
        Placement::ModsFolder => {
            let folder = target_folder(target, mod_id)?;
            let dest = target.join(&folder);
            // Defence in depth: never back up or replace anything but a direct child of the mods folder.
            if mod_id.is_empty()
                || matches!(folder.as_str(), "" | "." | "..")
                || dest.parent() != Some(target)
            {
                return Err(Error::UnsafeTarget(dest));
            }
            move_in(game, staged, &dest, backup_dir, journal)
        }
        Placement::Module { folder } => {
            let atelier = target.join(ATELIER_FOLDER);
            let dest = atelier.join(folder);
            let plain = folder.components().all(|part| match part {
                Component::Normal(name) => is_windows_safe_name(&name.to_string_lossy()),
                _ => false,
            });
            if mod_id.is_empty() || !plain || folder.as_os_str().is_empty() {
                return Err(Error::UnsafeTarget(dest));
            }
            let old = find_module(&atelier, mod_id)?.filter(|old| *old != dest);
            if let Some(old) = &old {
                back_up(game, old, backup_dir, journal)?;
            }
            move_in(game, staged, &dest, backup_dir, journal)?;
            // A replaced mod.io file leaves its `<modId>_<modfileId>` folder empty; remove_dir keeps a full one.
            if let Some(parent) = old
                .as_deref()
                .and_then(Path::parent)
                .filter(|dir| *dir != atelier)
            {
                let _ = fs::remove_dir(parent);
            }
            Ok(())
        }
        Placement::Plugin {
            plugins_dir,
            folder,
        } => {
            let dest = plugins_dir.join(folder);
            if !plugins_dir.is_absolute()
                || matches!(folder.as_str(), "" | "." | "..")
                || !is_windows_safe_name(folder)
                || dest.parent() != Some(plugins_dir.as_path())
            {
                return Err(Error::UnsafeTarget(dest));
            }
            move_in(game, staged, &dest, backup_dir, journal)
        }
    }
}

/// Backs up whatever sits at `dest`, then moves the staged root there.
fn move_in(
    game: GameId,
    staged: &StagedItem,
    dest: &Path,
    backup_dir: &Path,
    journal: &mut Vec<Step>,
) -> Result<()> {
    if dest.exists() {
        back_up(game, dest, backup_dir, journal)?;
    }
    move_dir(&staged.staged_dir, dest)?;
    journal.push(Step::Placed {
        staged: staged.staged_dir.clone(),
        dest: dest.to_path_buf(),
    });
    Ok(())
}

fn back_up(game: GameId, dir: &Path, backup_dir: &Path, journal: &mut Vec<Step>) -> Result<()> {
    let backup = backup_path(game, backup_dir, dir)?;
    move_dir(dir, &backup)?;
    journal.push(Step::BackedUp {
        original: dir.to_path_buf(),
        backup,
    });
    Ok(())
}

/// The folder under `root` holding the module or map whose id is `short_name`, at any depth.
pub(crate) fn find_module(root: &Path, short_name: &str) -> Result<Option<PathBuf>> {
    if !root.is_dir() {
        return Ok(None);
    }
    for dir in child_dirs(root)? {
        if file_name(&dir).starts_with('.') {
            continue;
        }
        let modules = modules_in(&dir)?;
        let map = map_in(&dir)?;
        if modules
            .iter()
            .any(|(module, _)| module.short_name() == short_name)
            || map.as_deref().map(maps::map_id).as_deref() == Some(short_name)
        {
            return Ok(Some(dir));
        }
        // A module or map folder's subfolders belong to it, never to another item.
        if modules.is_empty()
            && map.is_none()
            && let Some(found) = find_module(&dir, short_name)?
        {
            return Ok(Some(found));
        }
    }
    Ok(None)
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
        .filter(|id| !id.is_empty())
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

/// Normalised ModIDs may hold `*`, which Windows forbids, or be a device name like `con`.
/// `~` never appears in a normalised ModID, so no two ModIDs share a folder or backup slot.
pub(crate) fn new_folder_name(mod_id: &str) -> String {
    let name = mod_id.replace('*', "_");
    if crate::fsops::is_windows_safe_name(&name) {
        name
    } else {
        format!("{name}~")
    }
}
