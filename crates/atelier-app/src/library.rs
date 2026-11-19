//! Local mods: games, scanning, activation, profiles, options, health and launch.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::MutexGuard;

use atelier_core::health::{self, Environment};
use atelier_core::scan::{self, ModsFolder};
use atelier_core::{
    GameId, GameInfo, HealthCheck, Issue, LoadOrder, LogEntry, Mod, ModLocation, ModOption,
    OptionValue, Profile, Store, activation, games, issues, load_order, modinfo, options,
};

use crate::error::{AppError, ErrorCode, Result};
use crate::{App, lock};

const GAMES: [GameId; 2] = [GameId::Anno117, GameId::Anno1800];

impl App {
    pub fn list_games(&self) -> Result<Vec<GameInfo>> {
        Ok(GAMES
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
        for item in &mut mods {
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
        Ok(load_order::compute(&self.scan(game)?))
    }

    pub fn set_mod_active(&self, game: GameId, mod_id: &str, active: bool) -> Result<()> {
        let _writes = self.write_guard(game)?;
        let target = self.find_mod(game, mod_id)?;
        activation::set_active(
            game,
            &self.writable(game),
            &target,
            active,
            &self.0.snapshots,
        )?;
        Ok(())
    }

    /// Only the Documents copy is removed; game and .ubi folders belong to the game.
    pub fn uninstall_mod(&self, game: GameId, mod_id: &str) -> Result<()> {
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
        // Kept in the backups for the retention period, so restore_mod can undo this.
        self.0.engine.remove_to_backup(game, &dir)?;
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
        self.0
            .engine
            .rollback(game, mod_id, &self.documents_mods(game))?;
        // A link made since (a fresh install from another page) wins over the old one.
        self.update_persisted(|state| {
            if let Some(link) = state
                .uninstalled_links
                .get_mut(&game)
                .and_then(|aside| aside.remove(mod_id))
            {
                let links = state.modio_links.entry(game).or_default();
                links.entry(mod_id.to_owned()).or_insert(link);
            }
        })
    }

    pub fn list_profiles(&self, game: GameId) -> Result<Vec<Profile>> {
        Ok(activation::list_profiles(&self.documents_mods(game))?)
    }

    pub fn switch_profile(&self, game: GameId, name: &str) -> Result<()> {
        let _writes = self.write_guard(game)?;
        Ok(activation::switch_profile(
            &self.writable(game),
            name,
            &self.0.snapshots,
        )?)
    }

    pub fn save_profile(&self, game: GameId, name: &str) -> Result<()> {
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
        let _writes = self.write_guard(game)?;
        Ok(activation::delete_profile(
            &self.writable(game),
            name,
            &self.0.snapshots,
        )?)
    }

    pub fn export_profile(&self, game: GameId, name: &str) -> Result<String> {
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
        let (install, _) = self.install(game).ok_or_else(|| game_not_found(game))?;
        if (self.0.is_running)(game) {
            return Err(AppError::new(
                ErrorCode::GameRunning,
                format!("{} is already running.", game.executable_name()),
            ));
        }
        let exe = install.join(game.executable());
        let work_dir = exe.parent().unwrap_or(&install);
        Command::new(&exe)
            .current_dir(work_dir)
            .spawn()
            .map_err(|error| AppError::io(&error, &exe))?;
        Ok(())
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
        let mut roots: Vec<PathBuf> = GAMES
            .iter()
            .map(|&game| games::documents_game_dir(game, &self.0.documents))
            .collect();
        roots.push(self.0.data_dir.clone());
        roots.extend(
            GAMES
                .iter()
                .filter_map(|&game| (self.0.locate)(game))
                .map(|(install, _)| install.join("mods")),
        );
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
        games::documents_game_dir(game, &self.0.documents).join("mods")
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
        if (self.0.is_running)(game) {
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

fn read_only_error(shipped: &Mod) -> AppError {
    let place = match shipped.location {
        ModLocation::Ubi => "Ubisoft-managed .ubi folder",
        ModLocation::Game | ModLocation::Documents => "game folder",
    };
    AppError::new(
        ErrorCode::ReadOnly,
        format!(
            "{} lives in the {place}. Atelier only removes mods in Documents.",
            shipped.name
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
