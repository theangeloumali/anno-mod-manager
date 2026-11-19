//! Live mod.io smoke test. Run with MODIO_API_KEY set: `cargo test -p atelier-modio -- --ignored`.

use atelier_core::GameId;
use atelier_modio::{ClientConfig, ModioClient, ModioQuery, ModioSort, modio_game_id};

#[tokio::test]
#[ignore = "hits the live mod.io API; needs MODIO_API_KEY"]
async fn live_games_and_browse() {
    let api_key = std::env::var("MODIO_API_KEY").expect("MODIO_API_KEY is set");
    let http = reqwest::Client::new();
    let client = ModioClient::new(ClientConfig {
        api_key: Some(api_key.clone()),
        base_override: None,
    })
    .expect("client builds")
    .with_token_store(std::sync::Arc::new(
        atelier_modio::MemoryTokenStore::default(),
    ));

    for game in [GameId::Anno117, GameId::Anno1800] {
        let id = modio_game_id(game);
        let status = http
            .get(format!("https://g-{id}.modapi.io/v1/games/{id}"))
            .query(&[("api_key", api_key.as_str())])
            .send()
            .await
            .map(|response| response.status().as_u16())
            .map_err(|err| err.without_url());
        println!("GET /games/{id}: {status:?}");
        assert_eq!(status.ok(), Some(200));

        let page = client
            .browse(&ModioQuery {
                game,
                search: String::new(),
                tags: Vec::new(),
                sort: ModioSort::Downloads,
                subscribed_only: false,
                offset: 0,
            })
            .await
            .expect("live browse");
        println!(
            "game {id}: {} free mods on page, {} total",
            page.items.len(),
            page.total
        );
    }
}
