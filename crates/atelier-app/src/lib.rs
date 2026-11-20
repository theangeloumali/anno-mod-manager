//! App state behind the Tauri commands. Every method returns UI-ready, camelCase types
//! and an [`AppError`] the UI can show as is.

mod collections;
mod downloads;
mod error;
mod install;
mod library;
mod modio;
mod state;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::Instant;

use atelier_core::activation::Snapshots;
use atelier_core::{GameId, ModUpdate, Store};
use atelier_install::{EngineConfig, InstallEngine};
use atelier_modio::{ModioClient, TokenStore};
use url::Url;

pub use error::{AppError, ErrorCode, Result};
pub use modio::{AppAccount, AuthMethod};
pub use state::{NewModsStart, Settings, UpdateChannel, UpdateCheck};

use crate::downloads::JobEntry;
use crate::state::Persisted;

const SNAPSHOTS_KEPT: usize = 20;

pub type Locate = Arc<dyn Fn(GameId) -> Option<(PathBuf, Store)> + Send + Sync>;
pub type IsRunning = Arc<dyn Fn(GameId) -> bool + Send + Sync>;

pub struct AppConfig {
    pub data_dir: PathBuf,
    /// None asks the OS for the user's Documents folder.
    pub documents_dir: Option<PathBuf>,
    pub locate: Locate,
    pub is_running: IsRunning,
    pub api_key_store: Arc<dyn TokenStore>,
    pub token_store: Arc<dyn TokenStore>,
    /// Debug builds only: stands in when the user has saved no key.
    pub dev_api_key: Option<String>,
    pub modio_base: Option<Url>,
}

#[derive(Clone)]
pub struct App(Arc<Inner>);

struct Inner {
    data_dir: PathBuf,
    documents: PathBuf,
    locate: Locate,
    is_running: IsRunning,
    api_key_store: Arc<dyn TokenStore>,
    token_store: Arc<dyn TokenStore>,
    dev_api_key: Option<String>,
    modio_base: Option<Url>,
    engine: InstallEngine,
    snapshots: Snapshots,
    persisted: Mutex<Persisted>,
    /// Preview id → game, since the engine's preview carries no game.
    previews: Mutex<HashMap<String, GameId>>,
    client: RwLock<Arc<ModioClient>>,
    jobs: Mutex<Vec<JobEntry>>,
    next_job: AtomicU64,
    /// When each page was last unsubscribed here, so a sync whose mod.io answer is older
    /// doesn't reinstall it.
    unsubscribed_at: Mutex<HashMap<(GameId, u64), Instant>>,
    /// One subscription sync at a time, so one can't overwrite another's pending removals.
    sync_lock: tokio::sync::Mutex<()>,
    updates: Mutex<HashMap<GameId, HashMap<String, ModUpdate>>>,
    /// (game, mod.io page) → profiles its mods go into off once installed, from a collection.
    held_off: Mutex<HashMap<(GameId, u64), collections::HeldCollection>>,
    /// Held for every write to mod and profile files; see `App::write_guard`.
    writes: Mutex<()>,
    /// When each game was last launched, since its process takes a moment to show up.
    launched: Mutex<HashMap<GameId, Instant>>,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        let documents = config
            .documents_dir
            .or_else(atelier_core::games::system_documents_dir)
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::GameNotFound,
                    "Atelier can't find your Documents folder.",
                )
            })?;
        let persisted = Persisted::load(&config.data_dir)?;
        let engine = InstallEngine::new(engine_config(
            &config.data_dir,
            persisted.settings.backup_retention_days,
        ));
        let api_key = current_key(&*config.api_key_store, config.dev_api_key.as_deref())?;
        let client = build_client(api_key, config.modio_base.clone(), &config.token_store)?;
        let app = Self(Arc::new(Inner {
            snapshots: Snapshots::new(config.data_dir.join("snapshots"), SNAPSHOTS_KEPT),
            data_dir: config.data_dir,
            documents,
            locate: config.locate,
            is_running: config.is_running,
            api_key_store: config.api_key_store,
            token_store: config.token_store,
            dev_api_key: config.dev_api_key,
            modio_base: config.modio_base,
            engine,
            persisted: Mutex::new(persisted),
            previews: Mutex::new(HashMap::new()),
            client: RwLock::new(Arc::new(client)),
            jobs: Mutex::new(Vec::new()),
            next_job: AtomicU64::new(1),
            updates: Mutex::new(HashMap::new()),
            held_off: Mutex::new(HashMap::new()),
            unsubscribed_at: Mutex::new(HashMap::new()),
            sync_lock: tokio::sync::Mutex::new(()),
            writes: Mutex::new(()),
            launched: Mutex::new(HashMap::new()),
        }));
        // Backups expire with time, not only when the setting changes; a failure loses nothing.
        let _ = app.prune_backups();
        Ok(app)
    }

    pub fn settings(&self) -> Settings {
        self.persisted().settings.clone()
    }

    /// Retention changes prune right away; a fresh engine is enough because pruning is stateless.
    pub fn save_settings(&self, settings: Settings) -> Result<()> {
        self.update_persisted(|state| state.settings = settings)?;
        self.prune_backups()
    }

    /// Uses the current retention setting; the long-lived engine holds the one from startup.
    pub(crate) fn prune_backups(&self) -> Result<()> {
        let retention = self.settings().backup_retention_days;
        InstallEngine::new(engine_config(&self.0.data_dir, retention))
            .prune_backups()
            .map_err(AppError::from)
    }

    fn persisted(&self) -> MutexGuard<'_, Persisted> {
        lock(&self.0.persisted)
    }

    fn update_persisted(&self, change: impl FnOnce(&mut Persisted)) -> Result<()> {
        let mut state = self.persisted();
        change(&mut state);
        state.save(&self.0.data_dir)
    }
}

fn engine_config(data_dir: &std::path::Path, retention_days: Option<u32>) -> EngineConfig {
    EngineConfig {
        staging_dir: data_dir.join("staging"),
        backup_dir: data_dir.join("backups"),
        retention_days,
    }
}

fn current_key(store: &dyn TokenStore, dev_key: Option<&str>) -> Result<Option<String>> {
    Ok(store.load()?.or_else(|| dev_key.map(str::to_owned)))
}

fn build_client(
    api_key: Option<String>,
    base_override: Option<Url>,
    tokens: &Arc<dyn TokenStore>,
) -> Result<ModioClient> {
    Ok(ModioClient::new(atelier_modio::ClientConfig {
        api_key,
        base_override,
    })?
    .with_token_store(Arc::clone(tokens)))
}

/// A poisoned lock only means another command panicked; the data is still usable.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
