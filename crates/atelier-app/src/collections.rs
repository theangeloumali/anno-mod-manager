//! "Install all + create profile": a collection lands in its own profile, so the user's
//! previous profile stays one click away in the profile bar.

use std::collections::HashSet;

use atelier_core::{ActivationScheme, GameId, activation};

use crate::error::{AppError, ErrorCode, Result};
use crate::{App, lock};

const PROFILE_PREFIX: &str = "Collection - ";

/// Where a collection's not-yet-downloaded mods go once they land: on in each collection
/// profile waiting for them, off in every other profile that existed before.
#[derive(Clone)]
pub(crate) struct HeldCollection {
    on: Vec<String>,
    off: Vec<String>,
}

impl HeldCollection {
    /// Two collections can wait on one mod; it ends up on in both of their profiles.
    fn merge(&mut self, other: &HeldCollection) {
        for name in &other.on {
            if !self.on.contains(name) {
                self.on.push(name.clone());
            }
        }
        for name in &other.off {
            if !self.off.contains(name) {
                self.off.push(name.clone());
            }
        }
        let on = self.on.clone();
        self.off.retain(|name| !on.contains(name));
    }
}
/// atelier-core refuses profile names over 64 bytes; this leaves room for a " 99" suffix.
const MAX_BASE_NAME: usize = 60;

impl App {
    /// Returns the new profile's name, or `None` where the game has no profiles (1800, legacy 117).
    pub async fn install_collection(
        &self,
        game: GameId,
        collection_id: u64,
    ) -> Result<Option<String>> {
        self.guard_writes(game)?;
        let collection = self
            .list_collections(game)
            .await?
            .into_iter()
            .find(|collection| collection.id == collection_id)
            .ok_or_else(|| {
                AppError::new(ErrorCode::NotFound, "That collection no longer exists.")
            })?;
        let modio_ids: HashSet<u64> = self
            .client()
            .collection_mods(game, collection_id)
            .await?
            .iter()
            .map(|item| item.id)
            .collect();
        // The game may have started while mod.io answered; check again before the first write.
        let _writes = self.write_guard(game)?;
        let profile = if activation::detect_scheme(game, &self.documents_mods(game))
            == ActivationScheme::Profile
        {
            let name = self.save_collection_profile(game, &collection.name, &modio_ids)?;
            self.hold_off_elsewhere(game, &name, &modio_ids)?;
            Some(name)
        } else {
            None
        };
        self.start_missing(game, modio_ids.into_iter())?;
        Ok(profile)
    }

    /// Mods not installed yet are set as they land, so switching back never turns them on.
    /// Their ModIDs only exist once the archive is read.
    fn hold_off_elsewhere(
        &self,
        game: GameId,
        collection_profile: &str,
        modio_ids: &HashSet<u64>,
    ) -> Result<()> {
        let installed: HashSet<u64> = self
            .list_mods(game)?
            .into_iter()
            .filter_map(|item| item.modio_id)
            .collect();
        let others: Vec<String> = self
            .list_profiles(game)?
            .into_iter()
            .map(|profile| profile.name)
            .filter(|name| name != collection_profile)
            .collect();
        let pending = HeldCollection {
            on: vec![collection_profile.to_owned()],
            off: others,
        };
        let mut held = lock(&self.0.held_off);
        for &modio_id in modio_ids.difference(&installed) {
            held.entry((game, modio_id))
                .and_modify(|existing| existing.merge(&pending))
                .or_insert_with(|| pending.clone());
        }
        Ok(())
    }

    /// A held page's dependencies are new to the user too, so they stay off in the same profiles.
    pub(crate) fn hold_dependencies(&self, game: GameId, modio_id: u64, dependencies: &[u64]) {
        let mut held = lock(&self.0.held_off);
        if let Some(pending) = held.get(&(game, modio_id)).cloned() {
            for &dependency in dependencies {
                held.entry((game, dependency))
                    .and_modify(|existing| existing.merge(&pending))
                    .or_insert_with(|| pending.clone());
            }
        }
    }

    /// Runs once a held page installs; a profile deleted since is skipped, never recreated.
    pub(crate) fn apply_held_off(
        &self,
        game: GameId,
        modio_id: u64,
        mod_ids: &[String],
    ) -> Result<()> {
        let Some(pending) = lock(&self.0.held_off).get(&(game, modio_id)).cloned() else {
            return Ok(());
        };
        let _writes = self.write_guard(game)?;
        let existing: Vec<String> = self
            .list_profiles(game)?
            .into_iter()
            .map(|profile| profile.name)
            .collect();
        let writable = self.writable(game);
        let snapshots = &self.0.snapshots;
        for name in pending.off.iter().filter(|name| existing.contains(name)) {
            activation::set_in_profile(&writable, name, mod_ids, false, snapshots)?;
        }
        // On in each waiting collection's profile, whatever the new-mods setting did on install.
        for name in pending.on.iter().filter(|name| existing.contains(name)) {
            activation::set_in_profile(&writable, name, mod_ids, true, snapshots)?;
        }
        lock(&self.0.held_off).remove(&(game, modio_id));
        Ok(())
    }

    /// Keeps today's on/off state, turns on the collection's installed mods, then switches.
    /// Mods still downloading start on because saved profiles enable new mods.
    fn save_collection_profile(
        &self,
        game: GameId,
        collection_name: &str,
        modio_ids: &HashSet<u64>,
    ) -> Result<String> {
        let taken: Vec<String> = self
            .list_profiles(game)?
            .into_iter()
            .map(|profile| profile.name)
            .collect();
        let name = unique_name(&profile_base_name(collection_name), &taken);
        let mut mods = self.list_mods(game)?;
        for item in &mut mods {
            if item.modio_id.is_some_and(|id| modio_ids.contains(&id)) {
                item.active = true;
            }
        }
        // The caller holds the write lock, so the core is called directly, not switch_profile.
        let writable = self.writable(game);
        activation::save_profile(&writable, &name, &mods, &self.0.snapshots)?;
        activation::switch_profile(&writable, &name, &self.0.snapshots)?;
        Ok(name)
    }
}

/// Profile names become file names, so anything but letters, digits, spaces, `-` and `_` goes.
fn profile_base_name(collection_name: &str) -> String {
    let cleaned: String = collection_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut name = format!(
        "{PROFILE_PREFIX}{}",
        if words.is_empty() { "Mods" } else { &words }
    );
    while name.len() > MAX_BASE_NAME {
        name.pop();
    }
    name.trim_end().to_owned()
}

/// `name`, then `name 2`, `name 3`, …; matched as NTFS matches file names, case-blind.
fn unique_name(base: &str, taken: &[String]) -> String {
    let is_taken = |candidate: &str| {
        taken
            .iter()
            .any(|name| activation::same_profile_name(name, candidate))
    };
    let mut candidate = base.to_owned();
    let mut number = 2;
    while is_taken(&candidate) {
        candidate = format!("{base} {number}");
        number += 1;
    }
    candidate
}
