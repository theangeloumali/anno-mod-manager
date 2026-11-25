//! Turns found roots into preview items: status against what's installed, plus the checks the UI shows.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use atelier_core::health::humankind::plugins_dir;
use atelier_core::humankind::maps::MAP_CATEGORY;
use atelier_core::{GameId, Mod, ModInfo, ModLocation, Severity, normalize_mod_id};

use crate::fsops::{child_dirs, file_name};
use crate::install::{ATELIER_FOLDER, Placement, StagedItem, is_eligible, new_folder_name};
use crate::roots::{FoundRoot, Manifest, detect_game, is_plugins_dir};
use crate::version::compare_versions;
use crate::{InstallCheck, InstallItemStatus, InstallPreviewItem, PreviewOptions};

const BEPINEX_CORE_DLL: &str = "BepInEx/core/BepInEx.dll";

pub(crate) struct Classifier<'a> {
    game: GameId,
    installed: &'a [Mod],
    options: &'a PreviewOptions,
    /// Every ModID in the drop, sub-mods included, so a dependency shipped alongside isn't "missing".
    dropped_ids: HashSet<String>,
    seen_ids: HashSet<String>,
}

impl<'a> Classifier<'a> {
    pub fn new<'r>(
        game: GameId,
        installed: &'a [Mod],
        options: &'a PreviewOptions,
        roots: impl Iterator<Item = &'r FoundRoot>,
    ) -> Self {
        let dropped_ids = roots
            .flat_map(|root| std::iter::once(root.mod_id()).chain(root.sub_mod_ids.iter().cloned()))
            .collect();
        Self {
            game,
            installed,
            options,
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
        let (name, version, category) = match &root.manifest {
            Manifest::Module { module, .. } => (module.name.clone(), module.version(), None),
            Manifest::Map { name } => (name.clone(), String::new(), Some(MAP_CATEGORY.into())),
            Manifest::Plugin { name } => (name.clone(), String::new(), None),
            _ => (
                info.and_then(|info| info.name.clone()).unwrap_or(folder),
                info.map(|info| info.version.clone()).unwrap_or_default(),
                info.and_then(|info| info.category.clone()),
            ),
        };
        let mut item = InstallPreviewItem {
            root_path: relative_path(staged, &root.dir),
            name,
            category: category.unwrap_or_default(),
            version,
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
        } else if let Some(message) = self.wrong_game(root, item.target_game, info) {
            item.status = InstallItemStatus::WrongGame;
            item.checks.push(check(Severity::Error, message));
        } else if !self.seen_ids.insert(item.mod_id.clone()) {
            item.status = InstallItemStatus::Invalid;
            let message = format!(
                "Another folder in this drop has the {} {}",
                self.id_noun(),
                item.mod_id
            );
            item.checks.push(check(Severity::Error, message));
        } else if let Manifest::Plugin { name } = &root.manifest {
            self.describe_plugin(root, name, &mut item);
        } else {
            self.compare_with_installed(root, &mut item);
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

    /// Where confirm puts this root; only Humankind roots leave the mods folder's top level.
    pub fn placement(&self, root: &FoundRoot) -> Placement {
        match &root.manifest {
            Manifest::Module { module, .. } => Placement::Module {
                folder: self.module_folder(&module.to_string()),
            },
            // Named after the map, not its folder, so two packs' `Earth/` never collide.
            Manifest::Map { name } => Placement::Module {
                folder: self.module_folder(name),
            },
            Manifest::Plugin { name } => Placement::Plugin {
                plugins_dir: self.plugins_dir().unwrap_or_default(),
                folder: new_folder_name(name),
            },
            _ => Placement::ModsFolder,
        }
    }

    fn module_folder(&self, long_name: &str) -> PathBuf {
        let mut folder = PathBuf::new();
        if let Some(modio) = &self.options.modio_folder {
            folder.push(modio);
        }
        folder.push(long_name);
        folder
    }

    /// A download into another `<modId>_<modfileId>` folder than the Atelier copy's: mod.io
    /// serves only a page's latest file, so a different one is newer.
    fn is_newer_modio_file(&self, existing: &Mod) -> bool {
        let Some(modio) = &self.options.modio_folder else {
            return false;
        };
        is_atelier_copy(existing)
            && !existing
                .folder
                .starts_with(&format!("{ATELIER_FOLDER}/{modio}/"))
    }

    fn plugins_dir(&self) -> Option<PathBuf> {
        let game_dir = self.options.game_dir.as_ref()?;
        Some(plugins_dir(game_dir))
    }

    fn id_noun(&self) -> &'static str {
        if self.game == GameId::Humankind {
            "module"
        } else {
            "ModID"
        }
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
        if self.game == GameId::Humankind {
            return self.humankind_invalid_reason(root);
        }
        match &root.manifest {
            Manifest::Parsed(_)
            | Manifest::Module { .. }
            | Manifest::Map { .. }
            | Manifest::Plugin { .. } => None,
            Manifest::Broken(reason) => Some(format!("modinfo.json can't be read: {reason}")),
            Manifest::Inferred => match self.game {
                GameId::Anno117 => {
                    Some("No modinfo.json, which Anno 117 needs to load a mod".into())
                }
                _ => None,
            },
            Manifest::Unrecognised => {
                Some("No modinfo.json or data folder, so this isn't a mod the game can load".into())
            }
        }
    }

    /// Anno roots fall through to the wrong-game check, so the player sees which game they're for.
    fn humankind_invalid_reason(&self, root: &FoundRoot) -> Option<String> {
        match &root.manifest {
            Manifest::Module {
                module,
                has_bundle,
                modules_in_folder,
            } => {
                if *modules_in_folder > 1 {
                    return Some(format!(
                        "{} holds {modules_in_folder} module files; Humankind needs one module per folder",
                        file_name(&root.dir)
                    ));
                }
                if !has_bundle {
                    return Some(format!(
                        "{module} has no {}.assetbundle next to it, so Humankind can't load it",
                        module.to_string().to_lowercase()
                    ));
                }
                self.copy_outside_atelier(&module.short_name())
            }
            Manifest::Map { .. } => self.copy_outside_atelier(&root.mod_id()),
            // A plugin is a DLL the game runs, so it never arrives unseen through a download.
            Manifest::Plugin { .. } if self.options.modio_folder.is_some() => Some(
                "BepInEx plugins run code inside the game, so Atelier won't install one from a download. Download it yourself and drop the file on Atelier to review it first."
                    .into(),
            ),
            Manifest::Plugin { .. } => {
                let Some(game_dir) = &self.options.game_dir else {
                    return Some(
                        "Humankind's install folder wasn't found, so this BepInEx plugin has nowhere to go"
                            .into(),
                    );
                };
                if game_dir.join(BEPINEX_CORE_DLL).is_file() {
                    None
                } else {
                    Some(
                        "BepInEx isn't installed in Humankind's folder. Install BepInEx first, then add this plugin."
                            .into(),
                    )
                }
            }
            Manifest::Unrecognised => Some(
                "No Humankind module, map or BepInEx plugin here, so Humankind can't load it"
                    .into(),
            ),
            Manifest::Parsed(_) | Manifest::Broken(_) | Manifest::Inferred => None,
        }
    }

    fn copy_outside_atelier(&self, mod_id: &str) -> Option<String> {
        let copy = copy_outside_atelier(self.installed, mod_id)?;
        let place = match copy.location {
            ModLocation::Documents | ModLocation::GameModio => {
                format!("Community\\{}", copy.folder.replace('/', "\\"))
            }
            other => format!("the {}", other.place()),
        };
        Some(format!(
            "Already installed in {place}; remove that copy first so Humankind doesn't load it twice"
        ))
    }

    fn wrong_game(
        &self,
        root: &FoundRoot,
        target_game: Option<GameId>,
        info: Option<&ModInfo>,
    ) -> Option<String> {
        match target_game {
            Some(built_for) if built_for != self.game => {
                Some(self.wrong_game_message(built_for, root, info))
            }
            // No marker or layout still means Anno: Humankind has no modinfo.json or data folder.
            None if self.game == GameId::Humankind && root.manifest.is_anno() => {
                let evidence = if matches!(root.manifest, Manifest::Inferred) {
                    "it has a data folder"
                } else {
                    "it has a modinfo.json"
                };
                Some(format!(
                    "Built for Anno ({evidence}). Humankind won't load it."
                ))
            }
            _ => None,
        }
    }

    fn wrong_game_message(
        &self,
        built_for: GameId,
        root: &FoundRoot,
        info: Option<&ModInfo>,
    ) -> String {
        let evidence = match (&root.manifest, info.and_then(|info| info.anno)) {
            (Manifest::Module { .. }, _) => "a module file, no modinfo.json".into(),
            (Manifest::Map { .. }, _) => "a .hmap map, no modinfo.json".into(),
            (_, Some(anno)) => format!("modinfo \"Anno\": {anno}"),
            (_, None) => "by its folder layout".into(),
        };
        format!(
            "Built for {} ({evidence}). {} won't load it.",
            built_for.display_name(),
            self.game.display_name()
        )
    }

    fn describe_plugin(&self, root: &FoundRoot, name: &str, item: &mut InstallPreviewItem) {
        let folder = new_folder_name(name);
        let exists = self
            .plugins_dir()
            .is_some_and(|plugins| plugins.join(&folder).exists());
        let message = if exists {
            item.status = InstallItemStatus::Update;
            format!("Replaces BepInEx\\plugins\\{folder}; the old one is backed up")
        } else {
            format!("Installs to BepInEx\\plugins\\{folder}")
        };
        item.checks.push(check(Severity::Ok, message));
        let left_out = left_out_bepinex_folders(root);
        if !left_out.is_empty() {
            let message = format!(
                "Only BepInEx\\plugins is installed; {} is left out, so copy it by hand if the plugin needs it",
                left_out.join(", ")
            );
            item.checks.push(check(Severity::Warning, message));
        }
    }

    fn compare_with_installed(&self, root: &FoundRoot, item: &mut InstallPreviewItem) {
        let existing = installed_copy(self.game, self.installed, &item.mod_id);
        let Some(existing) = existing else {
            let message = match self.placement(root) {
                Placement::Module { folder } => {
                    let folder = Path::new(ATELIER_FOLDER).join(folder);
                    format!("Installs to Community\\{}", windows_path(&folder))
                }
                _ => format!("Installs to mods\\{}", self.free_folder_name(&item.mod_id)),
            };
            item.checks.push(check(Severity::Ok, message));
            return;
        };
        let noun = self.id_noun();
        let previous = existing.version.clone();
        let (status, level, message) = match compare_versions(&item.version, &previous) {
            // A versionless item's only version is its mod.io file, named in its folder.
            Ordering::Equal if previous.is_empty() && self.is_newer_modio_file(existing) => (
                InstallItemStatus::Update,
                Severity::Ok,
                "A newer file from mod.io; the old one is backed up".into(),
            ),
            // Only a Documents copy is replaced; a shipped one stays where it is.
            Ordering::Greater if existing.location == ModLocation::Documents => (
                InstallItemStatus::Update,
                Severity::Ok,
                format!("Same {noun}, newer version; {previous} is backed up"),
            ),
            Ordering::Greater => (
                InstallItemStatus::Update,
                Severity::Ok,
                format!("Same {noun}, newer than the {previous} in the game folder"),
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

/// A Humankind copy Atelier didn't install (Documents, Workshop, game folder or the game's own
/// mod.io), which would load alongside a new one.
pub fn copy_outside_atelier<'m>(installed: &'m [Mod], mod_id: &str) -> Option<&'m Mod> {
    installed
        .iter()
        .find(|copy| copy.game == GameId::Humankind && copy.id == mod_id && !is_atelier_copy(copy))
}

/// A copy under `Community\Atelier`, the only Humankind copies Atelier installed.
fn is_atelier_copy(copy: &Mod) -> bool {
    copy.location == ModLocation::Documents
        && copy.folder.starts_with(&format!("{ATELIER_FOLDER}/"))
}

/// The BepInEx folders beside `plugins` (config, patchers…), which a plugin install doesn't copy.
fn left_out_bepinex_folders(root: &FoundRoot) -> Vec<String> {
    let Some(bepinex) = root.dir.parent().filter(|_| is_plugins_dir(&root.dir)) else {
        return Vec::new();
    };
    child_dirs(bepinex)
        .unwrap_or_default()
        .iter()
        .filter(|dir| **dir != root.dir)
        .map(|dir| file_name(dir))
        .collect()
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
        .find(|staged| {
            copy_stamp(game, installed, &staged.item.mod_id) != staged.seen
                || bepinex_gone(&staged.placement)
        })
        .map(|staged| staged.item.mod_id.clone())
}

/// A plugin's target was checked at preview; BepInEx may have been removed since.
fn bepinex_gone(placement: &Placement) -> bool {
    match placement {
        Placement::Plugin { plugins_dir, .. } => !plugins_dir
            .parent()
            .and_then(Path::parent)
            .is_some_and(|game_dir| game_dir.join(BEPINEX_CORE_DLL).is_file()),
        _ => false,
    }
}

fn check(level: Severity, message: String) -> InstallCheck {
    InstallCheck { level, message }
}

fn windows_path(path: &Path) -> String {
    path.components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("\\")
}

fn relative_path(base: &Path, dir: &Path) -> String {
    dir.strip_prefix(base)
        .unwrap_or(dir)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}
