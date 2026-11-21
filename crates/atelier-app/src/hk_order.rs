//! Humankind load order, its profiles and the ModListGuard history behind them.

use std::path::PathBuf;

use atelier_core::humankind::history::{self, HistoryEntry};
use atelier_core::humankind::{detect, load_order, profiles, registry};
use atelier_core::{GameId, Profile, games};

use crate::error::{AppError, ErrorCode, Result};
use crate::{App, lock};

const HUMANKIND: GameId = GameId::Humankind;

impl App {
    /// Returns the order as written: short names, the base game first.
    pub fn set_load_order(&self, game: GameId, order: Vec<String>) -> Result<Vec<String>> {
        if game != HUMANKIND {
            return Err(AppError::new(
                ErrorCode::NotSupported,
                format!("{} has no editable load order.", game.display_name()),
            ));
        }
        let _writes = self.write_guard(game)?;
        self.hk_write_order(&order)
    }

    /// Same shape as the Anno profiles; active means it matches the registry's list.
    pub fn humankind_profiles(&self) -> Result<Vec<Profile>> {
        let current = self.hk_current_order()?;
        Ok(profiles::list(&self.hk_profiles_file())?
            .into_iter()
            .map(|profile| Profile {
                active: load_order::normalize(&profile.modules) == current,
                name: profile.name,
                built_in: false,
            })
            .collect())
    }

    /// Profiles live in Atelier's data folder, so saving one is allowed while the game runs.
    pub fn save_humankind_profile(&self, name: &str) -> Result<()> {
        let _writes = lock(&self.0.writes);
        profiles::save(
            &self.hk_profiles_file(),
            name,
            &self.hk_current_order()?,
            &self.0.snapshots,
        )?;
        Ok(())
    }

    pub fn apply_humankind_profile(&self, name: &str) -> Result<()> {
        let _writes = self.write_guard(HUMANKIND)?;
        let profile = profiles::find(&self.hk_profiles_file(), name)?;
        self.hk_write_order(&profile.modules)?;
        Ok(())
    }

    pub fn delete_humankind_profile(&self, name: &str) -> Result<()> {
        let _writes = lock(&self.0.writes);
        Ok(profiles::delete(
            &self.hk_profiles_file(),
            name,
            &self.0.snapshots,
        )?)
    }

    /// Newest first; `restore_history_entry` takes an index into this list.
    pub fn mod_list_history(&self) -> Result<Vec<HistoryEntry>> {
        Ok(history::read_history(&history::history_path(
            &self.hk_game_dir(),
        ))?)
    }

    /// Saved as a profile, not written, so the player decides when to switch to it.
    pub fn restore_history_entry(&self, index: usize) -> Result<String> {
        let entry = self
            .mod_list_history()?
            .into_iter()
            .nth(index)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::NotFound,
                    "That mod list history entry no longer exists.",
                )
            })?;
        let _writes = lock(&self.0.writes);
        let saved = profiles::save(
            &self.hk_profiles_file(),
            &restored_name(&entry.time),
            &entry.old,
            &self.0.snapshots,
        )?;
        Ok(saved.name)
    }

    /// Listed modules that aren't installed; Humankind resets the list to vanilla over any.
    pub fn launch_blockers(&self, game: GameId) -> Result<Vec<String>> {
        if game != HUMANKIND {
            return Ok(Vec::new());
        }
        Ok(load_order::missing(
            &self.hk_current_order()?,
            &self.scan(game)?,
        ))
    }

    /// The caller holds `write_guard`, so the scan and the write see the same mods.
    fn hk_write_order(&self, order: &[String]) -> Result<Vec<String>> {
        let installed = self.scan(HUMANKIND)?;
        let order = load_order::normalize(order);
        load_order::validate(&order, &load_order::module_types(&installed))
            .map_err(atelier_core::Error::from)?;
        let missing = load_order::missing(&order, &installed);
        if !missing.is_empty() {
            return Err(atelier_core::Error::MissingModules(missing).into());
        }
        let game_dir = self.hk_game_dir();
        let path = detect::registry_path(&game_dir)?
            .ok_or(atelier_core::Error::RegistryNotFound(game_dir))?;
        registry::write_modules(&path, &order, &self.0.snapshots)?;
        Ok(order)
    }

    /// The base game alone until the game has written a registry.
    fn hk_current_order(&self) -> Result<Vec<String>> {
        let modules = match detect::registry_path(&self.hk_game_dir())? {
            Some(path) => registry::read_modules(&path)?,
            None => Vec::new(),
        };
        Ok(load_order::normalize(&modules))
    }

    fn hk_game_dir(&self) -> PathBuf {
        games::documents_game_dir(HUMANKIND, &self.0.documents)
    }

    fn hk_profiles_file(&self) -> PathBuf {
        profiles::profiles_path(&self.0.data_dir)
    }
}

/// `2026-11-21T03:04:05.0000000Z` → `Restored 2026-11-21 03-04-05`; names refuse `:` and `.`.
fn restored_name(time: &str) -> String {
    let stamp = time.split('.').next().unwrap_or(time).replacen('T', " ", 1);
    let safe: String = stamp
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect();
    format!("Restored {safe}")
}
