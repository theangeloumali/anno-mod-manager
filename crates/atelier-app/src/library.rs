//! Local mods: games, scanning, activation, profiles, options, health and launch.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::MutexGuard;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use atelier_core::health::humankind::plugins_dir;
use atelier_core::health::{self, Environment};
use atelier_core::scan::{self, ModsFolder};
use atelier_core::{
    GameId, GameInfo, HealthCheck, Issue, LoadOrder, LogEntry, Mod, ModLocation, ModOption,
    OptionValue, Profile, Store, activation, games, issues, load_order, modinfo, options,
};

use crate::error::{AppError, ErrorCode, Result};
use crate::{App, lock};

/// No mod.io file has id 0, so a link set to it always sees the latest file as an update.
const UNKNOWN_MODFILE: u64 = 0;
/// How long after a launch writes stay refused while the game's process starts up.
const LAUNCH_GRACE: Duration = Duration::from_secs(30);

impl App {
    pub fn list_games(&self) -> Result<Vec<GameInfo>> {
        Ok(GameId::ALL
            .into_iter()
            .map(|game| {
                games::game_info(
                    game,
                    self.install(game),
                    &self.0.documents,
                    (self.0.is_running)(game),
                )
            })
            .collect())
    }

    /// `None` goes back to auto-detection.
    pub fn set_game_path(&self, game: GameId, path: Option<&str>) -> Result<()> {
        let chosen = match path.map(PathBuf::from) {
            Some(dir) if !dir.join(game.executable()).is_file() => {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    format!(
                        "{} isn't in that folder. Pick the folder {} is installed in.",
                        game.executable().display(),
                        game.display_name()
                    ),
                ));
            }
            Some(dir) if !dir.is_absolute() || self.mods_can_write(&dir) => {
                return Err(AppError::new(
                    ErrorCode::InvalidInput,
                    "That folder is where mods or Atelier keep files, so it can't be the game. \
                     Pick the folder the game is installed in.",
                ));
            }
            other => other,
        };
        self.update_persisted(|state| match chosen {
            Some(dir) => {
                state.game_paths.insert(game, dir);
            }
            None => {
                state.game_paths.remove(&game);
            }
        })
    }

    pub fn list_mods(&self, game: GameId) -> Result<Vec<Mod>> {
        let mut mods = self.scan(game)?;
        let links = self.persisted().modio_links.get(&game).cloned();
        let updates = lock(&self.0.updates).get(&game).cloned();
        // mod.io installs only ever land in Documents, so a shipped copy is never its page's.
        for item in mods
            .iter_mut()
            .filter(|item| item.location == ModLocation::Documents)
        {
            if let Some(link) = links.as_ref().and_then(|links| links.get(&item.id)) {
                item.modio_id.get_or_insert(link.modio_id);
            }
            if let Some(update) = updates.as_ref().and_then(|updates| updates.get(&item.id)) {
                item.update = Some(update.clone());
            }
        }
        Ok(mods)
    }

    pub fn list_issues(&self, game: GameId) -> Result<Vec<Issue>> {
        let mods = self.scan(game)?;
        Ok(issues::check(&mods, &load_order::compute(&mods)))
    }

    pub fn load_order(&self, game: GameId) -> Result<LoadOrder> {
        match game {
            GameId::Humankind => self.humankind_load_order(),
            GameId::Anno117 | GameId::Anno1800 => Ok(load_order::compute(&self.scan(game)?)),
        }
    }

    pub fn set_mod_active(&self, game: GameId, mod_id: &str, active: bool) -> Result<()> {
        let _writes = self.write_guard(game)?;
        let target = self.find_mod(game, mod_id)?;
        activation::set_active(
            game,
            &self.writable(game),
            &target,
            active,
            &self.scan(game)?,
            &self.0.snapshots,
        )?;
        Ok(())
    }

    /// Only the Documents copy is removed; game and .ubi folders belong to the game. A
    /// subscribed mod uninstalled here is left out of later syncs until subscribed again.
    pub fn uninstall_mod(&self, game: GameId, mod_id: &str) -> Result<()> {
        let page = self
            .persisted()
            .modio_links
            .get(&game)
            .and_then(|links| links.get(mod_id))
            .map(|link| link.modio_id);
        self.uninstall_to_backup(game, mod_id)?;
        let subscribed = page.filter(|page| {
            self.persisted()
                .subscribed
                .get(&game)
                .is_some_and(|subscribed| subscribed.contains(page))
        });
        if let Some(page) = subscribed {
            self.update_persisted(|state| {
                state.declined.entry(game).or_default().insert(page);
            })?;
        }
        Ok(())
    }

    /// Moves the Documents copy into the backups; a sync's removal, which declines nothing.
    pub(crate) fn uninstall_to_backup(&self, game: GameId, mod_id: &str) -> Result<()> {
        let _writes = self.write_guard(game)?;
        let mods = self.scan(game)?;
        // A sub-mod lives inside another mod's folder, so only top-level mods are removable.
        let matches: Vec<&Mod> = mods
            .iter()
            .filter(|item| item.id == mod_id && item.parent_id.is_none())
            .collect();
        let Some(target) = matches
            .iter()
            .find(|item| item.location == ModLocation::Documents)
        else {
            return Err(match matches.first() {
                Some(shipped) => read_only_error(shipped),
                None => not_found(mod_id),
            });
        };
        let dir = scan::mod_dir(&self.folders(game), target).ok_or_else(|| not_found(mod_id))?;
        // Humankind resets a list naming a missing module to vanilla, so the module leaves it
        // first, unless a Workshop or game-folder copy still provides it.
        if game == GameId::Humankind && matches.len() == 1 {
            let writable = self.writable(game);
            activation::set_active(game, &writable, target, false, &mods, &self.0.snapshots)?;
        }
        // Kept in the backups for the retention period, so restore_mod can undo this.
        self.0.engine.remove_to_backup(game, &dir)?;
        // A mod.io install sits one level down (`<modId>_<fileId>/<module>`); drop that folder
        // once empty. remove_dir refuses a non-empty folder, so nothing else can go.
        if let Some(parent) = dir.parent()
            && parent != self.documents_mods(game)
            && parent.starts_with(self.documents_mods(game))
        {
            let _ = fs::remove_dir(parent);
        }
        let has_link = self
            .persisted()
            .modio_links
            .get(&game)
            .is_some_and(|links| links.contains_key(mod_id));
        if has_link {
            // Set aside, not dropped: restore_mod puts it back.
            self.update_persisted(|state| {
                if let Some(link) = state
                    .modio_links
                    .get_mut(&game)
                    .and_then(|links| links.remove(mod_id))
                {
                    let aside = state.uninstalled_links.entry(game).or_default();
                    aside.insert(mod_id.to_owned(), link);
                }
            })?;
        }
        Ok(())
    }

    /// Puts back the newest backup of `mod_id`: an uninstall or a replaced version.
    pub fn restore_mod(&self, game: GameId, mod_id: &str) -> Result<()> {
        let _writes = self.write_guard(game)?;
        // Humankind loads every copy of a module, so a backup never lands beside another one.
        if game == GameId::Humankind
            && let Some(other) = self.scan(game)?.into_iter().find(|copy| {
                copy.id == mod_id
                    && !(copy.location == ModLocation::Documents
                        && copy.folder.starts_with("Atelier/"))
            })
        {
            return Err(AppError::new(
                ErrorCode::ReadOnly,
                format!(
                    "{} is already installed in the {}; remove that copy first so Humankind doesn't load it twice.",
                    other.name,
                    other.location.place()
                ),
            ));
        }
        self.0
            .engine
            .rollback(game, mod_id, &self.documents_mods(game))?;
        self.relink_after_restore(game, mod_id)
    }

    /// A restored backup is either an undone uninstall or an older file than the link names.
    pub(crate) fn relink_after_restore(&self, game: GameId, mod_id: &str) -> Result<()> {
        self.update_persisted(|state| {
            let aside = state
                .uninstalled_links
                .get_mut(&game)
                .and_then(|aside| aside.remove(mod_id));
            let links = state.modio_links.entry(game).or_default();
            match (aside, links.get_mut(mod_id)) {
                // Undoing an uninstall: the set-aside link describes exactly this copy.
                (Some(link), None) => {
                    links.insert(mod_id.to_owned(), link);
                }
                // An older version replaced the linked file, so its file is no longer known and
                // the next update check offers the current one again.
                (_, Some(link)) => link.modfile_id = UNKNOWN_MODFILE,
                (None, None) => {}
            }
        })
    }

    pub fn list_profiles(&self, game: GameId) -> Result<Vec<Profile>> {
        match game {
            GameId::Humankind => self.humankind_profiles(),
            GameId::Anno117 | GameId::Anno1800 => {
                Ok(activation::list_profiles(&self.documents_mods(game))?)
            }
        }
    }

    pub fn switch_profile(&self, game: GameId, name: &str) -> Result<()> {
        if game == GameId::Humankind {
            return self.apply_humankind_profile(name);
        }
        let _writes = self.write_guard(game)?;
        Ok(activation::switch_profile(
            &self.writable(game),
            name,
            &self.0.snapshots,
        )?)
    }

    pub fn save_profile(&self, game: GameId, name: &str) -> Result<()> {
        if game == GameId::Humankind {
            return self.save_humankind_profile(name);
        }
        let _writes = self.write_guard(game)?;
        let mods = self.scan(game)?;
        Ok(activation::save_profile(
            &self.writable(game),
            name,
            &mods,
            &self.0.snapshots,
        )?)
    }

    pub fn delete_profile(&self, game: GameId, name: &str) -> Result<()> {
        if game == GameId::Humankind {
            return self.delete_humankind_profile(name);
        }
        let _writes = self.write_guard(game)?;
        Ok(activation::delete_profile(
            &self.writable(game),
            name,
            &self.0.snapshots,
        )?)
    }

    pub fn export_profile(&self, game: GameId, name: &str) -> Result<String> {
        if game == GameId::Humankind {
            return self.export_humankind_profile(name);
        }
        Ok(activation::export_profile(
            &self.documents_mods(game),
            name,
        )?)
    }

    pub fn list_options(&self, game: GameId, mod_id: &str) -> Result<Vec<ModOption>> {
        let defaults = self.option_defaults(game, mod_id)?;
        Ok(options::load(
            &self.documents_mods(game),
            mod_id,
            &defaults,
        )?)
    }

    /// `None` puts the option back to the mod's default.
    pub fn set_option(
        &self,
        game: GameId,
        mod_id: &str,
        key: &str,
        value: Option<OptionValue>,
    ) -> Result<()> {
        let _writes = self.write_guard(game)?;
        let defaults = self.option_defaults(game, mod_id)?;
        let option = defaults
            .iter()
            .find(|option| option.key == key)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::NotFound,
                    format!("{mod_id} has no option called {key}."),
                )
            })?;
        let value = value.unwrap_or_else(|| option.default_value.clone());
        Ok(options::save(
            &self.writable(game),
            mod_id,
            &[(key.to_owned(), value)],
            &self.0.snapshots,
        )?)
    }

    /// The checks that need no network.
    pub fn local_health_checks(&self, game: GameId) -> Result<Vec<HealthCheck>> {
        let mods = self.scan(game)?;
        let install = self.install(game).map(|(path, _)| path);
        Ok(health::check_environment(&Environment {
            game,
            install_path: install.as_deref(),
            mods_dir: &self.documents_mods(game),
            mods: &mods,
        }))
    }

    pub fn loader_log(&self, game: GameId) -> Result<Vec<LogEntry>> {
        match game {
            // Humankind writes HTML diagnostics, not an Anno mod-loader log; reading them is H10.
            GameId::Humankind => return Ok(Vec::new()),
            GameId::Anno117 | GameId::Anno1800 => {}
        }
        let path = games::documents_game_dir(game, &self.0.documents)
            .join("log")
            .join("mod-loader.log");
        match fs::read(&path) {
            Ok(bytes) => Ok(health::log::parse(&String::from_utf8_lossy(&bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(AppError::io(&error, &path)),
        }
    }

    pub fn launch_game(&self, game: GameId) -> Result<()> {
        let (install, store) = self.install(game).ok_or_else(|| game_not_found(game))?;
        if (self.0.is_running)(game) {
            return Err(AppError::new(
                ErrorCode::GameRunning,
                format!("{} is already running.", game.executable_name()),
            ));
        }
        // Humankind resets the whole mod list to vanilla when any listed module is missing.
        let missing = self.launch_blockers(game)?;
        if !missing.is_empty() {
            return Err(atelier_core::Error::MissingModules(missing).into());
        }
        // A picked folder is the Steam copy only when Steam's own detection finds it there.
        let store = match store {
            Store::Manual
                if (self.0.locate)(game).is_some_and(|(steam_copy, detected)| {
                    detected == Store::Steam && games::same_path(&steam_copy, &install)
                }) =>
            {
                Store::Steam
            }
            other => other,
        };
        let target = games::launch_target(game, &install, store);
        // A write in progress finishes first, so the game never reads a half-written mods folder.
        let _writes = lock(&self.0.writes);
        (self.0.launch)(&target).map_err(|error| AppError::io(&error, &install))?;
        lock(&self.0.launched).insert(game, Instant::now());
        Ok(())
    }

    /// Takes the old community 1800 loader out and puts the game's own python35.dll back,
    /// returning the loader's backup (none when it was already deleted by hand).
    pub fn remove_legacy_loader(&self, game: GameId) -> Result<Option<PathBuf>> {
        if game != GameId::Anno1800 {
            return Err(AppError::new(
                ErrorCode::NotSupported,
                "Only Anno 1800 had the old mod loader.",
            ));
        }
        let (install, _) = self.install(game).ok_or_else(|| game_not_found(game))?;
        let _writes = self.write_guard(game)?;
        if !health::legacy_loader_removable(&install) {
            return Err(AppError::new(
                ErrorCode::NotFound,
                "No old mod loader is installed.",
            ));
        }
        let dir = health::loader_dir(&install);
        let loader = dir.join(health::LEGACY_LOADER_DLL);
        let original = dir.join(health::LEGACY_LOADER_ORIGINAL);
        let backup = if loader.is_file() {
            Some(self.back_up_loader(&loader)?)
        } else {
            None
        };
        // A rename over the loader replaces it in one step, so python35.dll is never missing;
        // if it fails, the loader is still in place.
        fs::rename(&original, &loader).map_err(|error| AppError::io(&error, &original))?;
        Ok(backup)
    }

    /// A copy, not a move: the game and the backups can be on different drives.
    fn back_up_loader(&self, loader: &Path) -> Result<PathBuf> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let dir = self
            .0
            .data_dir
            .join("backups")
            .join("old-mod-loader")
            .join(stamp.to_string());
        fs::create_dir_all(&dir).map_err(|error| AppError::io(&error, &dir))?;
        let backup = dir.join(health::LEGACY_LOADER_DLL);
        fs::copy(loader, &backup).map_err(|error| AppError::io(&error, loader))?;
        Ok(backup)
    }

    /// A folder the user picked wins over detection, unless it has since become untrusted.
    pub(crate) fn install(&self, game: GameId) -> Option<(PathBuf, Store)> {
        let chosen = self.persisted().game_paths.get(&game).cloned();
        chosen
            .filter(|path| !self.mods_can_write(path))
            .map(|path| (path, Store::Manual))
            .or_else(|| (self.0.locate)(game))
    }

    /// Launch runs the executable it finds, and a mod can ship its own Anno117.exe, so no
    /// folder that mods or the app write into may stand in for the game.
    fn mods_can_write(&self, dir: &Path) -> bool {
        let mut roots: Vec<PathBuf> = GameId::ALL
            .iter()
            .map(|&game| games::documents_game_dir(game, &self.0.documents))
            .collect();
        roots.push(self.0.data_dir.clone());
        for game in GameId::ALL {
            let install = (self.0.locate)(game).map(|(install, _)| install);
            if let (GameId::Humankind, Some(install)) = (game, &install) {
                roots.push(plugins_dir(install));
            }
            roots.extend(
                games::mod_folders(game, install.as_deref(), &self.0.documents)
                    .into_iter()
                    .map(|folder| folder.path),
            );
        }
        let dir = canonical(dir);
        roots.iter().any(|root| dir.starts_with(canonical(root)))
    }

    pub(crate) fn scan(&self, game: GameId) -> Result<Vec<Mod>> {
        let mut mods = scan::scan(game, &self.folders(game))?.mods;
        let state = activation::read(game, &self.documents_mods(game))?;
        activation::apply(&state, &mut mods);
        Ok(mods)
    }

    pub(crate) fn documents_mods(&self, game: GameId) -> PathBuf {
        games::documents_mods_dir(game, &self.0.documents)
    }

    pub(crate) fn writable(&self, game: GameId) -> ModsFolder {
        ModsFolder {
            path: self.documents_mods(game),
            location: ModLocation::Documents,
        }
    }

    /// Serialises every write to mod and profile files: their edits are read-modify-write, so two
    /// at once could lose one. Taken by public entry points only; helpers under it never re-lock.
    pub(crate) fn write_guard(&self, game: GameId) -> Result<MutexGuard<'_, ()>> {
        let guard = lock(&self.0.writes);
        self.guard_writes(game)?;
        Ok(guard)
    }

    pub(crate) fn guard_writes(&self, game: GameId) -> Result<()> {
        let just_launched = lock(&self.0.launched)
            .get(&game)
            .is_some_and(|at| at.elapsed() < LAUNCH_GRACE);
        if just_launched || (self.0.is_running)(game) {
            return Err(AppError::new(
                ErrorCode::GameRunning,
                format!(
                    "{} is running. Close the game to change mods.",
                    game.executable_name()
                ),
            ));
        }
        Ok(())
    }

    fn folders(&self, game: GameId) -> Vec<ModsFolder> {
        let install = self.install(game).map(|(path, _)| path);
        games::mod_folders(game, install.as_deref(), &self.0.documents)
    }

    fn find_mod(&self, game: GameId, mod_id: &str) -> Result<Mod> {
        self.scan(game)?
            .into_iter()
            .find(|item| item.id == mod_id)
            .ok_or_else(|| not_found(mod_id))
    }

    fn option_defaults(&self, game: GameId, mod_id: &str) -> Result<Vec<ModOption>> {
        let target = self.find_mod(game, mod_id)?;
        let dir = scan::mod_dir(&self.folders(game), &target).ok_or_else(|| not_found(mod_id))?;
        Ok(modinfo::read_dir(&dir)?
            .map(|info| info.options)
            .unwrap_or_default())
    }
}

/// Only reached for a copy outside Documents.
fn read_only_error(shipped: &Mod) -> AppError {
    AppError::new(
        ErrorCode::ReadOnly,
        format!(
            "{} lives in the {}. Atelier only removes mods in Documents.",
            shipped.name,
            shipped.location.place()
        ),
    )
}

pub(crate) fn not_found(mod_id: &str) -> AppError {
    AppError::new(
        ErrorCode::NotFound,
        format!("{mod_id} isn't installed any more."),
    )
}

pub(crate) fn game_not_found(game: GameId) -> AppError {
    AppError::new(
        ErrorCode::GameNotFound,
        format!(
            "Atelier can't find {}. Set its folder in Settings.",
            game.display_name()
        ),
    )
}

/// Resolves `..` and links so a path can't dodge a prefix check; a missing path stays as given.
fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}
