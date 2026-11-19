use std::collections::HashMap;

use atelier_core::GameId;

use crate::api::{ApiMod, ApiPage, STATUS_ACCEPTED, STATUS_DELETED};
use crate::client::{Auth, MAX_LIMIT, ModioClient, modio_game_id};
use crate::error::{Error, Result};
use crate::types::UpdateCheck;

impl ModioClient {
    /// `installed` is `(mod_id, installed_modfile_id)`. Up-to-date mods and mods
    /// without a modfile produce no entry.
    pub async fn check_updates(
        &self,
        game: GameId,
        installed: &[(u64, u64)],
    ) -> Result<Vec<UpdateCheck>> {
        let path = format!("games/{}/mods", modio_game_id(game));
        let mut checks = Vec::new();
        for chunk in installed.chunks(MAX_LIMIT) {
            let ids: Vec<String> = chunk.iter().map(|(mod_id, _)| mod_id.to_string()).collect();
            let query = [("id-in", ids.join(",")), ("_limit", MAX_LIMIT.to_string())];
            let page: ApiPage<ApiMod> = self.get_json(game, &path, Auth::Any, &query).await?;
            let mut found: HashMap<u64, ApiMod> =
                page.data.into_iter().map(|item| (item.id, item)).collect();

            for &(mod_id, installed_modfile_id) in chunk {
                // Listings omit removed and hidden mods; ask for those directly.
                let current = match found.remove(&mod_id) {
                    Some(item) => Some(item),
                    None => self.fetch_mod(game, mod_id).await?,
                };
                checks.extend(classify(mod_id, installed_modfile_id, current));
            }
        }
        Ok(checks)
    }

    async fn fetch_mod(&self, game: GameId, mod_id: u64) -> Result<Option<ApiMod>> {
        let path = format!("games/{}/mods/{mod_id}", modio_game_id(game));
        match self.get_json(game, &path, Auth::Any, &[]).await {
            Ok(item) => Ok(Some(item)),
            Err(Error::NotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }
}

fn classify(
    mod_id: u64,
    installed_modfile_id: u64,
    current: Option<ApiMod>,
) -> Option<UpdateCheck> {
    let Some(item) = current else {
        return Some(UpdateCheck::Removed { mod_id });
    };
    if item.status == STATUS_DELETED {
        return Some(UpdateCheck::Removed { mod_id });
    }
    if item.visible == 0 || item.status != STATUS_ACCEPTED {
        return Some(UpdateCheck::Hidden { mod_id });
    }
    item.modfile
        .filter(|modfile| modfile.id != installed_modfile_id)
        .map(|modfile| UpdateCheck::UpdateAvailable {
            mod_id,
            modfile: modfile.into(),
        })
}
