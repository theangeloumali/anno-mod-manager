#![allow(dead_code)] // each test binary uses a different subset

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use atelier_app::{App, AppConfig};
use atelier_core::{GameId, Store};
use atelier_modio::MemoryTokenStore;
use serde_json::Value;
use tempfile::TempDir;
use url::Url;
use wiremock::MockServer;
use zip::write::SimpleFileOptions;

pub const API_KEY: &str = "0123456789abcdef0123456789abcdef";

/// One app over a throwaway Documents, data and (optional) game install folder.
pub struct Harness {
    pub root: TempDir,
    pub app: App,
    pub running: Arc<AtomicBool>,
    pub api_keys: Arc<MemoryTokenStore>,
    pub tokens: Arc<MemoryTokenStore>,
}

pub struct Options {
    pub install: Option<GameId>,
    pub api_key: Option<&'static str>,
    pub dev_api_key: Option<&'static str>,
    pub modio_base: Option<Url>,
    /// Stands in for the running-game check, e.g. to pause a job at its install step.
    pub is_running: Option<IsRunning>,
}

pub type IsRunning = Arc<dyn Fn(GameId) -> bool + Send + Sync>;

impl Default for Options {
    fn default() -> Self {
        Self {
            install: None,
            api_key: Some(API_KEY),
            dev_api_key: None,
            modio_base: None,
            is_running: None,
        }
    }
}

impl Harness {
    pub fn new() -> Self {
        Self::with(Options::default())
    }

    pub fn with(options: Options) -> Self {
        let root = tempfile::tempdir().unwrap();
        let documents = root.path().join("My Documents");
        fs::create_dir_all(&documents).unwrap();
        let install_root = root.path().join("Program Files").join("Anno");
        let installed = options.install.map(|game| {
            let exe = install_root.join(game.executable());
            fs::create_dir_all(exe.parent().unwrap()).unwrap();
            File::create(exe).unwrap();
            (game, install_root.clone())
        });
        let running = Arc::new(AtomicBool::new(false));
        let running_flag = Arc::clone(&running);
        let api_keys = Arc::new(match options.api_key {
            Some(key) => MemoryTokenStore::with_token(key),
            None => MemoryTokenStore::default(),
        });
        let tokens = Arc::new(MemoryTokenStore::default());
        let config = AppConfig {
            data_dir: root.path().join("data"),
            documents_dir: Some(documents),
            locate: Arc::new(move |game| {
                installed
                    .as_ref()
                    .filter(|(installed_game, _)| *installed_game == game)
                    .map(|(_, path)| (path.clone(), Store::Steam))
            }),
            is_running: options
                .is_running
                .unwrap_or_else(|| Arc::new(move |_| running_flag.load(Ordering::SeqCst))),
            api_key_store: api_keys.clone(),
            token_store: tokens.clone(),
            dev_api_key: options.dev_api_key.map(str::to_owned),
            modio_base: options.modio_base,
        };
        let app = App::new(config).unwrap();
        Self {
            root,
            app,
            running,
            api_keys,
            tokens,
        }
    }

    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }

    pub fn install_dir(&self) -> PathBuf {
        self.root.path().join("Program Files").join("Anno")
    }

    pub fn documents_mods(&self, game: GameId) -> PathBuf {
        self.root
            .path()
            .join("My Documents")
            .join(game.documents_folder())
            .join("mods")
    }

    /// Writes a mod folder with a modinfo.json into the Documents mods folder.
    pub fn add_mod(&self, game: GameId, folder: &str, id: &str, version: &str) -> PathBuf {
        let dir = self.documents_mods(game).join(folder);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("modinfo.json"), modinfo(id, version, game)).unwrap();
        dir
    }

    pub fn drop_path(&self, name: &str) -> PathBuf {
        let dir = self.root.path().join("Drops With Spaces");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }
}

pub fn modinfo(id: &str, version: &str, game: GameId) -> String {
    let anno = match game {
        GameId::Anno117 => "\"Anno\": 8,",
        GameId::Anno1800 => "",
    };
    format!(
        "{{ {anno} \"ModID\": \"{id}\", \"Version\": \"{version}\", \
         \"ModName\": {{ \"English\": \"{id} name\" }}, \"Category\": \"Test\" }}"
    )
}

pub fn write_zip(path: &Path, files: &[(&str, &[u8])]) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    for (name, bytes) in files {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap();
}

pub fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        for (name, bytes) in files {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    cursor.into_inner()
}

pub fn fixture(name: &str) -> Value {
    let path = format!(
        "{}/../../tests/fixtures/modio/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

pub fn base(server: &MockServer) -> Url {
    Url::parse(&format!("{}/v1/", server.uri())).unwrap()
}

pub fn harness_for(server: &MockServer, api_key: Option<&'static str>) -> Harness {
    Harness::with(Options {
        api_key,
        modio_base: Some(base(server)),
        ..Options::default()
    })
}
