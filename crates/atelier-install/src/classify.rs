//! Turns found roots into preview items: status against what's installed, plus the checks the UI shows.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::Path;

use atelier_core::{GameId, Mod, ModInfo, ModLocation, Severity, normalize_mod_id};

use crate::fsops::file_name;
use crate::install::{StagedItem, is_eligible, new_folder_name};
use crate::roots::{FoundRoot, Manifest, detect_game};
use crate::version::compare_versions;
use crate::{InstallCheck, InstallItemStatus, InstallPreviewItem};

pub(crate) struct Classifier<'a> {
    game: GameId,
    installed: &'a [Mod],
    /// Every ModID in the drop, sub-mods included, so a dependency shipped alongside isn't "missing".
    dropped_ids: HashSet<String>,
    seen_ids: HashSet<String>,
}

impl<'a> Classifier<'a> {
    pub fn new<'r>(
        game: GameId,
        installed: &'a [Mod],
        roots: impl Iterator<Item = &'r FoundRoot>,
    ) -> Self {
        let dropped_ids = roots
            .flat_map(|root| std::iter::once(root.mod_id()).chain(root.sub_mod_ids.iter().cloned()))
            .collect();
        Self {
            game,
            installed,
            dropped_ids,
            seen_ids: HashSet::new(),
        }
    }

    /// `staged` is the source's staging folder, which `root_path` is relative to.
    pub fn describe(&mut self, staged: &Path, root: &FoundRoot) -> InstallPreviewItem {
        let mod_id = root.mod_id();
        let folder = file_name(&root.dir);
        let info = match &root.manifest {
            Manifest::Parsed(info) => Some(info.as_ref()),
            _ => None,
        };
        let mut item = InstallPreviewItem {
            root_path: relative_path(staged, &root.dir),
            name: info
                .and_then(|info| info.name.clone())
                .unwrap_or_else(|| folder.clone()),
            category: info
                .and_then(|info| info.category.clone())
                .unwrap_or_default(),
            version: info.map(|info| info.version.clone()).unwrap_or_default(),
            previous_version: None,
            target_game: detect_game(root),
            status: InstallItemStatus::New,
            checks: Vec::new(),
            sub_mod_ids: root.sub_mod_ids.clone(),
            mod_id,
        };
        if let Some(reason) = self.invalid_reason(root) {
            item.status = InstallItemStatus::Invalid;
            item.checks.push(check(Severity::Error, reason));
        } else if let Some(built_for) = item.target_game.filter(|game| *game != self.game) {
            item.status = InstallItemStatus::WrongGame;
            item.checks.push(check(
                Severity::Error,
                self.wrong_game_message(built_for, info),
            ));
        } else if !self.seen_ids.insert(item.mod_id.clone()) {
            item.status = InstallItemStatus::Invalid;
            let message = format!("Another folder in this drop has the ModID {}", item.mod_id);
            item.checks.push(check(Severity::Error, message));
        } else {
            self.compare_with_installed(&mut item);
            if matches!(root.manifest, Manifest::Inferred) {
                let message = "No modinfo.json, so the folder name is used as its ModID";
                item.checks.push(check(Severity::Warning, message.into()));
            }
            item.checks.extend(
                info.into_iter()
                    .flat_map(|info| self.missing_requirements(info)),
            );
        }
        item
    }

    /// Mirrors `install::target_folder` from the scan, so the preview names the folder confirm will use.
    fn free_folder_name(&self, mod_id: &str) -> String {
        let taken: HashSet<&str> = self
            .installed
            .iter()
            .filter(|installed| installed.location == ModLocation::Documents)
            .map(|installed| installed.folder.as_str())
            .collect();
        let base = new_folder_name(mod_id);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while taken.contains(candidate.as_str()) {
            candidate = format!("{base}-{suffix}");
            suffix += 1;
        }
        candidate
    }

    fn invalid_reason(&self, root: &FoundRoot) -> Option<String> {
        // An empty ModID would name the mods folder itself as the install target.
        if root.mod_id().is_empty() {
            return Some("Its ModID is empty once normalised, so the game can't load it".into());
        }
        match &root.manifest {
            Manifest::Parsed(_) => None,
            Manifest::Broken(reason) => Some(format!("modinfo.json can't be read: {reason}")),
            Manifest::Inferred if self.game == GameId::Anno117 => {
                Some("No modinfo.json, which Anno 117 needs to load a mod".into())
            }
            Manifest::Inferred => None,
            Manifest::Unrecognised => {
                Some("No modinfo.json or data folder, so this isn't a mod the game can load".into())
            }
        }
    }

    fn wrong_game_message(&self, built_for: GameId, info: Option<&ModInfo>) -> String {
        let evidence = match info.and_then(|info| info.anno) {
            Some(anno) => format!("modinfo \"Anno\": {anno}"),
            None => "by its folder layout".into(),
        };
        format!(
            "Built for {} ({evidence}). {} won't load it.",
            built_for.display_name(),
            self.game.display_name()
        )
    }

    fn compare_with_installed(&self, item: &mut InstallPreviewItem) {
        let existing = installed_copy(self.game, self.installed, &item.mod_id);
        let Some(existing) = existing else {
            let folder = self.free_folder_name(&item.mod_id);
            item.checks
                .push(check(Severity::Ok, format!("Installs to mods\\{folder}")));
            return;
        };
        let previous = existing.version.clone();
        let (status, level, message) = match compare_versions(&item.version, &previous) {
            Ordering::Greater => (
                InstallItemStatus::Update,
                Severity::Ok,
                format!("Same ModID, newer version; {previous} is backed up"),
            ),
            Ordering::Less => (
                InstallItemStatus::Downgrade,
                Severity::Warning,
                format!("Older than the installed {previous}, so it's skipped"),
            ),
            Ordering::Equal => (
                InstallItemStatus::Same,
                Severity::Ok,
                "Already installed at this version, so it's skipped".into(),
            ),
        };
        item.status = status;
        item.previous_version = Some(previous);
        item.checks.push(check(level, message));
    }

    fn missing_requirements(&self, info: &ModInfo) -> Vec<InstallCheck> {
        info.dependencies
            .require
            .iter()
            .map(|raw| normalize_mod_id(raw))
            .filter(|id| {
                !self.dropped_ids.contains(id) && !self.installed.iter().any(|m| &m.id == id)
            })
            .map(|id| {
                check(
                    Severity::Warning,
                    format!("Needs {id}, which isn't installed."),
                )
            })
            .collect()
    }
}

/// The copy an item is compared with, at preview and again at confirm: the Documents copy an
/// install replaces, else a shipped one.
fn installed_copy<'m>(game: GameId, installed: &'m [Mod], mod_id: &str) -> Option<&'m Mod> {
    installed
        .iter()
        .filter(|installed| {
            installed.id == mod_id && installed.game == game && installed.parent_id.is_none()
        })
        .min_by_key(|installed| installed.location != ModLocation::Documents)
}

/// Where, which version and how big: a copy swapped for another with the same version still
/// changes its folder or size.
pub(crate) type CopyStamp = (ModLocation, String, String, u64);

pub(crate) fn copy_stamp(game: GameId, installed: &[Mod], mod_id: &str) -> Option<CopyStamp> {
    installed_copy(game, installed, mod_id).map(|copy| {
        (
            copy.location,
            copy.folder.clone(),
            copy.version.clone(),
            copy.size_bytes,
        )
    })
}

/// The first item that would install whose installed copy is not what the preview saw.
pub(crate) fn changed_since_preview(
    game: GameId,
    items: &[StagedItem],
    installed: &[Mod],
    include_downgrades: bool,
) -> Option<String> {
    items
        .iter()
        .filter(|staged| is_eligible(staged.item.status, include_downgrades))
        .find(|staged| copy_stamp(game, installed, &staged.item.mod_id) != staged.seen)
        .map(|staged| staged.item.mod_id.clone())
}

fn check(level: Severity, message: String) -> InstallCheck {
    InstallCheck { level, message }
}

fn relative_path(base: &Path, dir: &Path) -> String {
    dir.strip_prefix(base)
        .unwrap_or(dir)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
