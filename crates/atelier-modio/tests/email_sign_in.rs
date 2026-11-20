mod common;

use std::sync::Arc;

use atelier_modio::{Error, MemoryTokenStore, TokenStore};
use common::*;
use serde_json::json;
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn api_error(status: u16, error_ref: u32) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(json!({
        "error": { "code": status, "error_ref": error_ref, "message": "mod.io says no" }
    }))
}

#[tokio::test]
async fn a_code_is_requested_for_the_email_with_the_users_key() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/emailrequest"))
        .and(query_param("api_key", API_KEY))
        .and(body_string_contains("email=player%40example.com"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("email_request.json")))
        .expect(1)
        .mount(&server)
        .await;

    anonymous_client(&server)
        .request_email_code("player@example.com")
        .await
        .unwrap();
}

#[tokio::test]
async fn a_badly_formed_email_is_its_own_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/emailrequest"))
        .respond_with(api_error(422, 13009))
        .mount(&server)
        .await;

    let error = anonymous_client(&server)
        .request_email_code("not-an-email")
        .await
        .unwrap_err();

    assert!(matches!(error, Error::InvalidEmail), "{error:?}");
}

#[tokio::test]
async fn an_emailed_code_signs_in_and_stores_the_token() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/oauth/emailexchange"))
        .and(query_param("api_key", API_KEY))
        .and(body_string_contains("security_code=AB12C"))
        .and(body_string_contains("terms_agreed=true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("access_token.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/me"))
        .and(header("Authorization", "Bearer emailed-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("me.json")))
        .mount(&server)
        .await;
    let store = Arc::new(MemoryTokenStore::default());

    let account = client_with_store(&server, store.clone())
        .sign_in_with_email_code(" ab12c ")
        .await
        .unwrap();

    assert_eq!(account.username.as_deref(), Some("Aurelia"));
    assert_eq!(store.load().unwrap().as_deref(), Some("emailed-token"));
}

#[tokio::test]
async fn a_wrong_or_expired_code_is_typed_and_stores_nothing() {
    for (error_ref, expired) in [(11014, false), (11013, false), (11012, true), (11011, true)] {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/oauth/emailexchange"))
            .respond_with(api_error(403, error_ref))
            .mount(&server)
            .await;
        let store = Arc::new(MemoryTokenStore::default());

        let error = client_with_store(&server, store.clone())
            .sign_in_with_email_code("AB12C")
            .await
            .unwrap_err();

        let typed = if expired {
            matches!(error, Error::SecurityCodeExpired)
        } else {
            matches!(error, Error::InvalidSecurityCode)
        };
        assert!(typed, "ref {error_ref}: {error:?}");
        assert_eq!(store.load().unwrap(), None);
    }
}
