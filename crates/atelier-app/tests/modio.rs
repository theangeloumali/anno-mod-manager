mod common;

use atelier_app::ErrorCode;
use atelier_core::GameId;
use atelier_modio::{ModioQuery, ModioSort, TokenStore};
use common::*;
use serde_json::Value;
use url::Url;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn query() -> ModioQuery {
    ModioQuery {
        game: GameId::Anno117,
        search: String::new(),
        tags: Vec::new(),
        sort: ModioSort::Trending,
        subscribed_only: false,
        offset: 0,
    }
}

async fn mount_terms(server: &MockServer, status: u16, body: Value) {
    Mock::given(method("GET"))
        .and(path("/v1/authenticate/terms"))
        .respond_with(ResponseTemplate::new(status).set_body_json(body))
        .mount(server)
        .await;
}

#[tokio::test]
async fn a_rejected_key_is_invalid_and_never_stored() {
    let server = MockServer::start().await;
    mount_terms(&server, 401, fixture("error_401.json")).await;
    let harness = harness_for(&server, None);
    let error = harness.app.set_api_key(API_KEY).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidApiKey);
    assert_eq!(
        error.message,
        "mod.io doesn't recognise that key. Copy it again from mod.io/me/access."
    );
    assert_eq!(harness.api_keys.load().unwrap(), None);
    assert!(!harness.app.account().await.unwrap().has_api_key);
}

#[tokio::test]
async fn an_accepted_key_is_stored_and_reported_without_echoing_it() {
    let server = MockServer::start().await;
    mount_terms(&server, 200, fixture("terms.json")).await;
    let harness = harness_for(&server, None);
    harness
        .app
        .set_api_key(&format!("  {API_KEY} "))
        .await
        .unwrap();
    assert_eq!(harness.api_keys.load().unwrap().as_deref(), Some(API_KEY));
    let account = harness.app.account().await.unwrap();
    assert!(account.has_api_key);
    assert!(!serde_json::to_string(&account).unwrap().contains(API_KEY));

    harness.app.clear_api_key().unwrap();
    assert!(!harness.app.account().await.unwrap().has_api_key);
}

#[tokio::test]
async fn a_malformed_key_is_rejected_before_any_request() {
    let harness = Harness::with(Options {
        api_key: None,
        ..Options::default()
    });
    let error = harness.app.set_api_key("not a key").await.unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidApiKey);
}

#[tokio::test]
async fn browsing_without_any_key_asks_for_one() {
    let harness = Harness::with(Options {
        api_key: None,
        ..Options::default()
    });
    let error = harness.app.browse(&query()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::NoApiKey);
    assert_eq!(
        error.message,
        "Add your mod.io API key in Settings to browse mod.io."
    );
}

#[tokio::test]
async fn a_debug_env_key_stands_in_when_none_is_saved() {
    let harness = Harness::with(Options {
        api_key: None,
        dev_api_key: Some(API_KEY),
        ..Options::default()
    });
    assert!(harness.app.account().await.unwrap().has_api_key);
}

#[tokio::test]
async fn an_unreachable_modio_is_offline() {
    let closed = Url::parse("http://127.0.0.1:9/v1/").unwrap();
    let harness = Harness::with(Options {
        modio_base: Some(closed),
        ..Options::default()
    });
    let error = harness.app.browse(&query()).await.unwrap_err();
    assert_eq!(error.code, ErrorCode::Offline);
    assert_eq!(
        error.message,
        "mod.io is unreachable. Check your connection."
    );
}

#[tokio::test]
async fn connectivity_follows_whether_modio_answers() {
    let closed = Url::parse("http://127.0.0.1:9/v1/").unwrap();
    let offline = Harness::with(Options {
        modio_base: Some(closed),
        ..Options::default()
    });
    assert!(!offline.app.modio_reachable().await);

    let server = MockServer::start().await;
    mount_terms(&server, 200, fixture("terms.json")).await;
    assert!(harness_for(&server, None).app.modio_reachable().await);
}

#[tokio::test]
async fn browse_reports_rating_as_a_whole_percentage() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mods_page.json")))
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    let page = harness.app.browse(&query()).await.unwrap();
    assert!(!page.items.is_empty());
    assert!(page.items.iter().all(|item| item.rating <= 100.0));
    assert!(page.items.iter().any(|item| item.rating > 1.0));
    assert!(page.items.iter().all(|item| item.rating.fract() == 0.0));
}

#[tokio::test]
async fn token_sign_in_is_the_only_supported_method() {
    let harness = Harness::new();
    for method in [
        atelier_app::AuthMethod::Browser,
        atelier_app::AuthMethod::Device,
    ] {
        let error = harness.app.sign_in(method, None).await.unwrap_err();
        assert_eq!(error.code, ErrorCode::NotSupported);
    }
}

#[tokio::test]
async fn syncing_subscriptions_needs_a_signed_in_account() {
    let harness = Harness::new();
    let error = harness
        .app
        .sync_subscriptions(GameId::Anno117)
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotSignedIn);
}

#[tokio::test]
async fn tags_come_back_flattened() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .and(query_param("api_key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("tags.json")))
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    let tags = harness.app.modio_tags(GameId::Anno117).await.unwrap();
    assert!(tags.contains(&"Gameplay".to_owned()));
}
