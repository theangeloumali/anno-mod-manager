#![allow(dead_code)] // each test binary uses a different subset of these helpers

use std::sync::Arc;

use atelier_modio::{ClientConfig, MemoryTokenStore, ModioClient};
use serde_json::Value;
use url::Url;
use wiremock::MockServer;

pub const API_KEY: &str = "test-api-key";
pub const TOKEN: &str = "test-bearer-token";

pub fn fixture_text(name: &str) -> String {
    let path = format!(
        "{}/../../tests/fixtures/modio/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("fixture {path}: {err}"))
}

pub fn fixture(name: &str) -> Value {
    serde_json::from_str(&fixture_text(name)).expect("fixture is valid JSON")
}

pub fn base_url(server: &MockServer) -> Url {
    Url::parse(&format!("{}/v1/", server.uri())).expect("mock server url")
}

pub fn client_with_store(server: &MockServer, store: Arc<MemoryTokenStore>) -> ModioClient {
    let config = ClientConfig {
        api_key: Some(API_KEY.to_owned()),
        base_override: Some(base_url(server)),
    };
    ModioClient::new(config)
        .expect("client builds")
        .with_token_store(store)
}

pub fn anonymous_client(server: &MockServer) -> ModioClient {
    client_with_store(server, Arc::new(MemoryTokenStore::default()))
}

pub fn signed_in_client(server: &MockServer) -> ModioClient {
    client_with_store(server, Arc::new(MemoryTokenStore::with_token(TOKEN)))
}

/// A mod object derived from the base fixture, for building large result sets.
pub fn mod_with(id: u64, modfile_id: u64) -> Value {
    let mut item = fixture("mod.json");
    item["id"] = id.into();
    item["modfile"]["id"] = modfile_id.into();
    item["modfile"]["mod_id"] = id.into();
    item
}

pub fn page_of(items: Vec<Value>, total: u64) -> Value {
    serde_json::json!({
        "data": items.clone(),
        "result_count": items.len(),
        "result_offset": 0,
        "result_limit": 100,
        "result_total": total,
    })
}
