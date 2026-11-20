mod common;

use atelier_core::GameId;
use atelier_modio::{ModioQuery, ModioSort, ReportKind, modio_game_id};
use common::*;
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn query(game: GameId) -> ModioQuery {
    ModioQuery {
        game,
        search: String::new(),
        tags: Vec::new(),
        sort: ModioSort::Trending,
        subscribed_only: false,
        offset: 0,
    }
}

#[test]
fn game_ids_match_modio() {
    assert_eq!(modio_game_id(GameId::Anno117), 11358);
    assert_eq!(modio_game_id(GameId::Anno1800), 4169);
}

#[tokio::test]
async fn browse_maps_mods_and_hides_paid_ones() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .and(query_param("api_key", API_KEY))
        .and(query_param("_sort", "-downloads_today"))
        .and(query_param("_q", "roads"))
        .and(query_param("tags", "Gameplay,Infrastructure"))
        .and(header("X-Modio-Platform", "windows"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mods_page.json")))
        .expect(1)
        .mount(&server)
        .await;

    let mut request = query(GameId::Anno117);
    request.search = "roads".into();
    request.tags = vec!["Gameplay".into(), "Infrastructure".into()];
    let page = anonymous_client(&server).browse(&request).await.unwrap();

    let ids: Vec<u64> = page.items.iter().map(|item| item.id).collect();
    assert_eq!(ids, vec![4801, 4803], "the paid mod 4802 is skipped");
    assert_eq!(page.total, 3);
    assert_eq!(page.offset, 0);
    let first = &page.items[0];
    assert_eq!(first.author, "Aurelia");
    assert_eq!(
        (first.date_added, first.date_updated),
        (1762000000, 1763400000)
    );
    assert_eq!(first.downloads, 15230);
    assert_eq!(first.size_bytes, 11);
    assert_eq!(first.tags, vec!["Gameplay", "Infrastructure"]);
    assert_eq!(
        first.logo_url.as_deref(),
        Some("https://thumb.modcdn.io/mods/4801/crop_320x180/logo.png")
    );
    assert_eq!(page.items[1].size_bytes, 0, "zero modfiles means no size");

    let json = serde_json::to_value(first).unwrap();
    assert!(json.get("nameId").is_some() && json.get("installedModId").is_some());
}

#[tokio::test]
async fn browse_past_the_end_returns_an_empty_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/4169/mods"))
        .and(query_param("_offset", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mods_page_end.json")))
        .mount(&server)
        .await;

    let mut request = query(GameId::Anno1800);
    request.offset = 100;
    let page = anonymous_client(&server).browse(&request).await.unwrap();
    assert!(page.items.is_empty());
    assert_eq!((page.offset, page.total), (100, 3));
}

#[tokio::test]
async fn subscribed_only_browse_uses_me_subscribed_with_bearer() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/me/subscribed"))
        .and(query_param("game_id", "11358"))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("subscriptions.json")))
        .mount(&server)
        .await;

    let mut request = query(GameId::Anno117);
    request.subscribed_only = true;
    let page = signed_in_client(&server).browse(&request).await.unwrap();
    assert_eq!(page.items.len(), 1);
    assert!(page.items[0].subscribed);
    let received = server.received_requests().await.unwrap();
    assert!(
        received[0]
            .url
            .query_pairs()
            .all(|(key, _)| key != "api_key"),
        "the bearer wins, so the api key is not sent"
    );
}

#[tokio::test]
async fn zero_subscriptions_is_an_empty_list() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/me/subscribed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("subscriptions_empty.json")))
        .mount(&server)
        .await;

    let subs = signed_in_client(&server)
        .subscriptions(GameId::Anno117)
        .await
        .unwrap();
    assert!(subs.is_empty());
}

#[tokio::test]
async fn subscriptions_without_a_token_fail_before_any_request() {
    let server = MockServer::start().await;
    let error = anonymous_client(&server)
        .subscriptions(GameId::Anno117)
        .await
        .unwrap_err();
    assert!(matches!(error, atelier_modio::Error::NotSignedIn));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn tags_are_flattened_and_deduplicated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("tags.json")))
        .mount(&server)
        .await;

    let tags = anonymous_client(&server)
        .tags(GameId::Anno117)
        .await
        .unwrap();
    assert_eq!(tags, vec!["Gameplay", "Cosmetic", "Latium", "Albion"]);
}

#[tokio::test]
async fn mod_detail_exposes_the_live_modfile_or_none() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mod.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4803"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mod_no_modfile.json")))
        .mount(&server)
        .await;

    let client = anonymous_client(&server);
    let detail = client.mod_detail(GameId::Anno117, 4801).await.unwrap();
    assert_eq!(detail.info.name, "Roman Roads Overhaul");
    let modfile = detail.modfile.expect("live modfile");
    assert_eq!(modfile.id, 77001);
    assert_eq!(modfile.md5, "5eb63bbbe01eeed093cb22bb8f5acdc3");
    assert_eq!(modfile.version.as_deref(), Some("1.2.0"));

    let empty = client.mod_detail(GameId::Anno117, 4803).await.unwrap();
    assert!(empty.modfile.is_none());
}

#[tokio::test]
async fn dependencies_skip_removed_mods() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801/dependencies"))
        .and(query_param("recursive", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("dependencies.json")))
        .mount(&server)
        .await;

    let deps = anonymous_client(&server)
        .dependencies(GameId::Anno117, 4801)
        .await
        .unwrap();
    let ids: Vec<u64> = deps.iter().map(|item| item.id).collect();
    assert_eq!(ids, vec![4820]);
}

#[tokio::test]
async fn subscribe_and_unsubscribe_send_bearer_writes() {
    let server = MockServer::start().await;
    let auth = format!("Bearer {TOKEN}");
    Mock::given(method("POST"))
        .and(path("/v1/games/11358/mods/4801/subscribe"))
        .and(header("Authorization", auth.as_str()))
        .and(body_string_contains("include_dependencies=false"))
        .respond_with(ResponseTemplate::new(201).set_body_json(fixture("mod.json")))
        .expect(1)
        .mount(&server)
        .await;
    // mod.io answers 415 without a form Content-Type, even on a bodiless DELETE.
    Mock::given(method("DELETE"))
        .and(path("/v1/games/11358/mods/4801/subscribe"))
        .and(header("Authorization", auth.as_str()))
        .and(header("Content-Type", "application/x-www-form-urlencoded"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let client = signed_in_client(&server);
    client.subscribe(GameId::Anno117, 4801).await.unwrap();
    client.unsubscribe(GameId::Anno117, 4801).await.unwrap();
}

#[tokio::test]
async fn collections_mark_the_followed_ones() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("collections.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/me/following/collections"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(fixture("followed_collections.json")),
        )
        .mount(&server)
        .await;

    let collections = signed_in_client(&server)
        .collections(GameId::Anno117)
        .await
        .unwrap();
    assert_eq!(collections.len(), 2);
    assert_eq!(collections[0].author, "Aurelia");
    assert_eq!(collections[0].followers, 37);
    assert!(!collections[0].following);
    assert!(collections[1].following);
}

#[tokio::test]
async fn collection_mods_hide_paid_mods() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/collections/310/mods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("collection_mods.json")))
        .mount(&server)
        .await;

    let mods = anonymous_client(&server)
        .collection_mods(GameId::Anno117, 310)
        .await
        .unwrap();
    assert_eq!(mods.len(), 1);
}

#[tokio::test]
async fn follow_rate_and_report_hit_their_endpoints() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/games/11358/collections/311/followers"))
        .respond_with(
            ResponseTemplate::new(201)
                .set_body_json(fixture("collections.json")["data"][1].clone()),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/v1/games/11358/collections/311/followers"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/games/11358/mods/4801/ratings"))
        .and(body_string_contains("rating=-1"))
        .respond_with(ResponseTemplate::new(201).set_body_json(fixture("message_201.json")))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/report"))
        .and(body_string_contains("resource=MODS"))
        .and(body_string_contains("type=2"))
        .and(body_string_contains("id=4801"))
        .respond_with(ResponseTemplate::new(201).set_body_json(fixture("message_201.json")))
        .expect(1)
        .mount(&server)
        .await;

    let client = signed_in_client(&server);
    client.follow(GameId::Anno117, 311, true).await.unwrap();
    client.follow(GameId::Anno117, 311, false).await.unwrap();
    client.rate(GameId::Anno117, 4801, false).await.unwrap();
    client
        .report(
            GameId::Anno117,
            4801,
            ReportKind::NotWorking,
            "Crashes on load",
        )
        .await
        .unwrap();
}
