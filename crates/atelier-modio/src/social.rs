//! Signed-in actions: subscriptions, collections, ratings and reports.

use std::collections::HashSet;

use atelier_core::GameId;
use reqwest::Method;

use crate::api::{ApiCollection, ApiMod};
use crate::browse::free_mods;
use crate::client::{Auth, ModioClient, modio_game_id};
use crate::error::Result;
use crate::types::{ModioCollection, ModioMod, ReportKind};

impl ModioClient {
    pub async fn subscribe(&self, game: GameId, mod_id: u64) -> Result<()> {
        // Dependencies are resolved and subscribed explicitly by the caller.
        let form = [("include_dependencies", "false".to_owned())];
        self.write(
            game,
            Method::POST,
            &subscribe_path(game, mod_id),
            Auth::Bearer,
            &form,
        )
        .await
    }

    pub async fn unsubscribe(&self, game: GameId, mod_id: u64) -> Result<()> {
        self.write(
            game,
            Method::DELETE,
            &subscribe_path(game, mod_id),
            Auth::Bearer,
            &[],
        )
        .await
    }

    pub async fn subscriptions(&self, game: GameId) -> Result<Vec<ModioMod>> {
        let query = [("game_id", modio_game_id(game).to_string())];
        let mods: Vec<ApiMod> = self
            .get_all_complete(game, "me/subscribed", Auth::Bearer, &query)
            .await?;
        Ok(mods.into_iter().map(|item| item.into_mod(true)).collect())
    }

    pub async fn collections(&self, game: GameId) -> Result<Vec<ModioCollection>> {
        let game_id = modio_game_id(game);
        let path = format!("games/{game_id}/collections");
        let collections: Vec<ApiCollection> = self.get_all(game, &path, Auth::Any, &[]).await?;
        let followed = self.followed_collection_ids(game).await?;
        Ok(collections
            .into_iter()
            .map(|collection| {
                let following = followed.contains(&collection.id);
                collection.into_collection(following)
            })
            .collect())
    }

    async fn followed_collection_ids(&self, game: GameId) -> Result<HashSet<u64>> {
        if !self.signed_in()? {
            return Ok(HashSet::new());
        }
        let followed: Vec<ApiCollection> = self
            .get_all(game, "me/following/collections", Auth::Bearer, &[])
            .await?;
        // The endpoint spans every game, so keep this game's ids only.
        let game_id = modio_game_id(game);
        Ok(followed
            .into_iter()
            .filter(|collection| collection.game_id == game_id)
            .map(|collection| collection.id)
            .collect())
    }

    pub async fn collection_mods(&self, game: GameId, collection_id: u64) -> Result<Vec<ModioMod>> {
        let path = format!(
            "games/{}/collections/{collection_id}/mods",
            modio_game_id(game)
        );
        let mods: Vec<ApiMod> = self.get_all(game, &path, Auth::Any, &[]).await?;
        Ok(free_mods(mods, false))
    }

    /// `follow: false` unfollows.
    pub async fn follow(&self, game: GameId, collection_id: u64, follow: bool) -> Result<()> {
        let path = format!(
            "games/{}/collections/{collection_id}/followers",
            modio_game_id(game)
        );
        let method = if follow { Method::POST } else { Method::DELETE };
        self.write(game, method, &path, Auth::Bearer, &[]).await
    }

    pub async fn rate(&self, game: GameId, mod_id: u64, up: bool) -> Result<()> {
        let path = format!("games/{}/mods/{mod_id}/ratings", modio_game_id(game));
        let rating = if up { "1" } else { "-1" };
        self.write(
            game,
            Method::POST,
            &path,
            Auth::Bearer,
            &[("rating", rating.to_owned())],
        )
        .await
    }

    pub async fn report(
        &self,
        game: GameId,
        mod_id: u64,
        kind: ReportKind,
        summary: &str,
    ) -> Result<()> {
        let form = [
            ("resource", "MODS".to_owned()),
            ("id", mod_id.to_string()),
            ("type", kind.code().to_string()),
            ("summary", summary.to_owned()),
        ];
        self.write(game, Method::POST, "report", Auth::Any, &form)
            .await
    }
}

fn subscribe_path(game: GameId, mod_id: u64) -> String {
    format!("games/{}/mods/{mod_id}/subscribe", modio_game_id(game))
}
