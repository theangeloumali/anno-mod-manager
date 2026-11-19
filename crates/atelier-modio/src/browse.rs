//! Read endpoints: browse, tags, detail and dependencies.

use atelier_core::GameId;

use crate::api::{ApiMod, ApiPage, ApiTagOption};
use crate::client::{Auth, ModioClient, modio_game_id};
use crate::error::Result;
use crate::types::{ModDetail, ModioMod, ModioPage, ModioQuery};

const PAGE_SIZE: u64 = 50;

/// Paid mods are out of scope for this app, so every listing drops them.
pub(crate) fn free_mods(mods: Vec<ApiMod>, subscribed: bool) -> Vec<ModioMod> {
    mods.into_iter()
        .filter(|item| !item.is_paid())
        .map(|item| item.into_mod(subscribed))
        .collect()
}

impl ModioClient {
    pub async fn browse(&self, query: &ModioQuery) -> Result<ModioPage> {
        let game_id = modio_game_id(query.game);
        let mut params = vec![
            ("_offset", query.offset.to_string()),
            ("_limit", PAGE_SIZE.to_string()),
            ("_sort", query.sort.api_value().to_owned()),
        ];
        let search = query.search.trim();
        if !search.is_empty() {
            params.push(("_q", search.to_owned()));
        }
        if !query.tags.is_empty() {
            params.push(("tags", query.tags.join(",")));
        }
        let (path, auth) = if query.subscribed_only {
            params.push(("game_id", game_id.to_string()));
            ("me/subscribed".to_owned(), Auth::Bearer)
        } else {
            (format!("games/{game_id}/mods"), Auth::Any)
        };

        let page: ApiPage<ApiMod> = self.get_json(query.game, &path, auth, &params).await?;
        Ok(ModioPage {
            items: free_mods(page.data, query.subscribed_only),
            total: page.result_total,
            offset: page.result_offset,
        })
    }

    /// Every tag the game offers, in mod.io's order, without duplicates.
    pub async fn tags(&self, game: GameId) -> Result<Vec<String>> {
        let path = format!("games/{}/tags", modio_game_id(game));
        let options: Vec<ApiTagOption> = self.get_all(game, &path, Auth::Any, &[]).await?;
        let mut tags: Vec<String> = Vec::new();
        for tag in options
            .into_iter()
            .filter(|option| !option.hidden)
            .flat_map(|option| option.tags)
        {
            if !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        Ok(tags)
    }

    pub async fn mod_detail(&self, game: GameId, mod_id: u64) -> Result<ModDetail> {
        let path = format!("games/{}/mods/{mod_id}", modio_game_id(game));
        let item: ApiMod = self.get_json(game, &path, Auth::Any, &[]).await?;
        Ok(item.into_detail())
    }

    /// The full dependency tree, minus mods that are deleted or hidden.
    pub async fn dependencies(&self, game: GameId, mod_id: u64) -> Result<Vec<ModioMod>> {
        let path = format!("games/{}/mods/{mod_id}/dependencies", modio_game_id(game));
        let query = [("recursive", "true".to_owned())];
        let mods: Vec<ApiMod> = self.get_all(game, &path, Auth::Any, &query).await?;
        Ok(mods
            .into_iter()
            .filter(ApiMod::is_live)
            .map(|item| item.into_mod(false))
            .collect())
    }
}
