//! Builds the one [`App`] every command shares, wired to the real machine.

use std::sync::Arc;

use atelier_app::{App, AppConfig};
use atelier_core::{games, process};
use atelier_modio::KeyringTokenStore;
use tauri::Manager;

pub fn manage_app(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let state = App::new(AppConfig {
        data_dir: app.path().app_data_dir()?,
        documents_dir: None,
        locate: Arc::new(|game| games::detect_install(game, &games::Probe::system(game))),
        is_running: Arc::new(process::is_game_running),
        api_key_store: Arc::new(KeyringTokenStore::api_key()),
        token_store: Arc::new(KeyringTokenStore::default()),
        dev_api_key: dev_api_key(),
        modio_base: None,
        launch: Arc::new(process::launch),
    })?;
    app.manage(state);
    Ok(())
}

/// Debug builds only: `MODIO_API_KEY` from the environment, else the repo's gitignored `.env.local`.
#[cfg(debug_assertions)]
fn dev_api_key() -> Option<String> {
    const NAME: &str = "MODIO_API_KEY";
    let env_file = concat!(env!("CARGO_MANIFEST_DIR"), "/../.env.local");
    std::env::var(NAME)
        .ok()
        .or_else(|| {
            std::fs::read_to_string(env_file)
                .ok()?
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once('=')?;
                    (key.trim() == NAME).then(|| value.trim().trim_matches('"').to_owned())
                })
        })
        .filter(|key| !key.is_empty())
}

#[cfg(not(debug_assertions))]
fn dev_api_key() -> Option<String> {
    None
}
