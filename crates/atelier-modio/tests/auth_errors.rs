mod common;

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Instant;

use atelier_core::GameId;
use atelier_modio::{ClientConfig, Error, MemoryTokenStore, ModioClient, TokenStore};
use common::*;
use url::Url;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn terms_are_fetched_with_the_api_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/authenticate/terms"))
        .and(query_param("api_key", API_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("terms.json")))
        .mount(&server)
        .await;

    let terms = anonymous_client(&server).terms().await.unwrap();
    assert_eq!(terms.agree_label, "I Agree");
    assert_eq!(terms.disagree_label, "No, Thanks");
    let privacy = terms
        .links
        .iter()
        .find(|link| link.kind == "privacy")
        .expect("privacy link");
    assert_eq!(privacy.url, "https://mod.io/privacy");
    assert!(privacy.required);
}

#[tokio::test]
async fn sign_in_validates_the_token_then_stores_it() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/me"))
        .and(header("Authorization", "Bearer fresh-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("me.json")))
        .mount(&server)
        .await;

    let store = Arc::new(MemoryTokenStore::default());
    let client = client_with_store(&server, store.clone());
    let account = client.sign_in_with_token("fresh-token").await.unwrap();
    assert!(account.signed_in && account.terms_accepted);
    assert_eq!(account.username.as_deref(), Some("Aurelia"));
    assert_eq!(store.load().unwrap().as_deref(), Some("fresh-token"));

    let again = client.account().await.unwrap();
    assert_eq!(again.username.as_deref(), Some("Aurelia"));

    client.sign_out().unwrap();
    assert_eq!(store.load().unwrap(), None);
    assert!(!client.account().await.unwrap().signed_in);
}

#[tokio::test]
async fn a_rejected_token_is_not_stored() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/me"))
        .respond_with(ResponseTemplate::new(401).set_body_json(fixture("error_401.json")))
        .mount(&server)
        .await;

    let store = Arc::new(MemoryTokenStore::default());
    let client = client_with_store(&server, store.clone());
    let error = client.sign_in_with_token("bad").await.unwrap_err();
    assert!(matches!(error, Error::Unauthorized));
    assert_eq!(store.load().unwrap(), None);
}

#[tokio::test]
async fn an_expired_token_surfaces_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/me"))
        .respond_with(ResponseTemplate::new(401).set_body_json(fixture("error_401.json")))
        .mount(&server)
        .await;

    let error = signed_in_client(&server).account().await.unwrap_err();
    assert!(matches!(error, Error::Unauthorized));
}

#[tokio::test]
async fn terms_required_and_limited_key_are_typed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/games/11358/mods/4801/subscribe"))
        .respond_with(ResponseTemplate::new(403).set_body_json(fixture("error_403_terms.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(fixture("error_403_limited_key.json")),
        )
        .mount(&server)
        .await;

    let client = signed_in_client(&server);
    let terms = client.subscribe(GameId::Anno117, 4801).await.unwrap_err();
    assert!(matches!(terms, Error::TermsRequired));
    let limited = client.tags(GameId::Anno117).await.unwrap_err();
    assert!(matches!(limited, Error::LimitedKeyAbilities));
}

#[tokio::test]
async fn a_429_backs_off_once_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(fixture("error_429.json")),
        )
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("tags.json")))
        .with_priority(2)
        .mount(&server)
        .await;

    let started = Instant::now();
    let tags = anonymous_client(&server)
        .tags(GameId::Anno117)
        .await
        .unwrap();
    assert!(!tags.is_empty());
    assert!(started.elapsed().as_millis() >= 1000, "honours retry-after");
}

#[tokio::test]
async fn a_retry_after_longer_than_a_minute_is_surfaced_not_slept() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "3600")
                .set_body_json(fixture("error_429.json")),
        )
        .mount(&server)
        .await;

    let started = Instant::now();
    let error = anonymous_client(&server)
        .tags(GameId::Anno117)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::RateLimited { retry_after: 3600 }));
    assert!(started.elapsed().as_secs() < 5, "never sleeps an hour");
}

#[tokio::test]
async fn an_oversized_json_response_is_refused() {
    let server = MockServer::start().await;
    let padding = "x".repeat(17 * 1024 * 1024);
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(format!("{{\"pad\":\"{padding}\"}}"), "application/json"),
        )
        .mount(&server)
        .await;

    let error = anonymous_client(&server)
        .tags(GameId::Anno117)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::ResponseTooLarge), "{error}");
}

#[tokio::test]
async fn a_list_that_never_ends_stops_after_the_page_cap() {
    let server = MockServer::start().await;
    let mut page = fixture("tags.json");
    let first = page["data"][0].clone();
    page["data"] = serde_json::Value::Array(vec![first]);
    page["result_count"] = 1.into();
    page["result_total"] = 1_000_000.into();
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page))
        .expect(50)
        .mount(&server)
        .await;

    anonymous_client(&server)
        .tags(GameId::Anno117)
        .await
        .unwrap();
}

#[tokio::test]
async fn a_second_429_surfaces_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/tags"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(fixture("error_429.json")),
        )
        .expect(2)
        .mount(&server)
        .await;

    let error = anonymous_client(&server)
        .tags(GameId::Anno117)
        .await
        .unwrap_err();
    assert!(matches!(error, Error::RateLimited { retry_after: 1 }));
}

#[tokio::test]
async fn a_refused_connection_is_offline_and_hides_the_key() {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let config = ClientConfig {
        api_key: Some(API_KEY.to_owned()),
        base_override: Some(Url::parse(&format!("http://127.0.0.1:{port}/v1/")).unwrap()),
    };
    let client = ModioClient::new(config)
        .unwrap()
        .with_token_store(Arc::new(MemoryTokenStore::default()));
    let error = client.tags(GameId::Anno117).await.unwrap_err();
    assert!(matches!(error, Error::Offline));
    assert!(!format!("{error} {error:?}").contains(API_KEY));
}

#[test]
fn config_debug_redacts_the_api_key() {
    let config = ClientConfig {
        api_key: Some(API_KEY.to_owned()),
        base_override: None,
    };
    assert!(!format!("{config:?}").contains(API_KEY));
}

#[tokio::test]
async fn no_key_and_no_token_fails_without_a_request() {
    let server = MockServer::start().await;
    let config = ClientConfig {
        api_key: None,
        base_override: Some(base_url(&server)),
    };
    let client = ModioClient::new(config)
        .unwrap()
        .with_token_store(Arc::new(MemoryTokenStore::default()));
    let error = client.tags(GameId::Anno117).await.unwrap_err();
    assert!(matches!(error, Error::MissingCredentials));
    assert!(server.received_requests().await.unwrap().is_empty());
}
