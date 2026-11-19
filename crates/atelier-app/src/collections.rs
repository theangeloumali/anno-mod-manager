//! "Install all + create profile": a collection lands in its own profile, so the user's
//! previous profile stays one click away in the profile bar.

use std::collections::HashSet;

use atelier_core::{ActivationScheme, GameId, activation};

use crate::App;
use crate::error::{AppError, ErrorCode, Result};

const PROFILE_PREFIX: &str = "Collection - ";
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
        let profile = if activation::detect_scheme(game, &self.documents_mods(game))
            == ActivationScheme::Profile
        {
            Some(self.save_collection_profile(game, &collection.name, &modio_ids)?)
        } else {
            None
        };
        self.start_missing(game, modio_ids.into_iter())?;
        Ok(profile)
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
        activation::save_profile(&self.writable(game), &name, &mods, &self.0.snapshots)?;
        self.switch_profile(game, &name)?;
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

/// `name`, then `name 2`, `name 3`, …; the game matches profile files case-insensitively.
fn unique_name(base: &str, taken: &[String]) -> String {
    let is_taken = |candidate: &str| {
        taken
            .iter()
            .any(|name| name.eq_ignore_ascii_case(candidate))
    };
    let mut candidate = base.to_owned();
    let mut number = 2;
    while is_taken(&candidate) {
        candidate = format!("{base} {number}");
        number += 1;
    }
    candidate
}
