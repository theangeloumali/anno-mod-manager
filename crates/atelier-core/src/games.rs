//! Per-game constants and install detection across Steam, Ubisoft Connect and Epic.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::activation::detect_scheme;
use crate::scan::ModsFolder;
use crate::{ActivationScheme, GameId, GameInfo, ModLocation, Store};

const MODS_DIR: &str = "mods";
const UBI_MODS_DIR: &str = ".ubi";

impl GameId {
    pub fn display_name(self) -> &'static str {
        match self {
            GameId::Anno117 => "Anno 117: Pax Romana",
            GameId::Anno1800 => "Anno 1800",
        }
    }

    pub fn executable_name(self) -> &'static str {
        match self {
            GameId::Anno117 => "Anno117.exe",
            GameId::Anno1800 => "Anno1800.exe",
        }
    }

    /// Relative to the install root.
    pub fn executable(self) -> PathBuf {
        Path::new("Bin").join("Win64").join(self.executable_name())
    }

    pub fn steam_app_id(self) -> u32 {
        match self {
            GameId::Anno117 => 3_274_580,
            GameId::Anno1800 => 916_440,
        }
    }

    /// Ubisoft Connect `Installs\<id>` registry key.
    pub fn ubisoft_install_id(self) -> u32 {
        match self {
            GameId::Anno117 => 921,
            GameId::Anno1800 => 4553,
        }
    }

    /// Folder under the user's Documents that holds `mods/` and `log/`.
    pub fn documents_folder(self) -> &'static str {
        match self {
            GameId::Anno117 => "Anno 117 - Pax Romana",
            GameId::Anno1800 => "Anno 1800",
        }
    }

    fn epic_title_prefix(self) -> &'static str {
        match self {
            GameId::Anno117 => "Anno 117",
            GameId::Anno1800 => "Anno 1800",
        }
    }
}

/// Where to look; `system` fills it from the registry and default paths.
#[derive(Debug, Clone, Default)]
pub struct Probe {
    pub steam_libraries: Vec<PathBuf>,
    pub epic_manifests: Option<PathBuf>,
    pub ubisoft_install: Option<PathBuf>,
}

impl Probe {
    pub fn system(game: GameId) -> Self {
        let steam_root = platform::steam_root();
        let mut steam_libraries: Vec<PathBuf> = steam_root
            .as_ref()
            .and_then(|root| {
                fs::read_to_string(root.join("steamapps").join("libraryfolders.vdf")).ok()
            })
            .map(|vdf| steam_library_paths(&vdf))
            .unwrap_or_default();
        if let Some(root) = steam_root
            && !steam_libraries.contains(&root)
        {
            steam_libraries.insert(0, root);
        }
        Self {
            steam_libraries,
            epic_manifests: platform::epic_manifests(),
            ubisoft_install: platform::ubisoft_install(game),
        }
    }
}

/// First store whose install actually contains the game executable.
pub fn detect_install(game: GameId, probe: &Probe) -> Option<(PathBuf, Store)> {
    let has_exe = |path: &PathBuf| path.join(game.executable()).is_file();
    let steam = probe
        .steam_libraries
        .iter()
        .filter_map(|library| steam_install(library, game))
        .find(has_exe)
        .map(|path| (path, Store::Steam));
    steam
        .or_else(|| {
            probe
                .ubisoft_install
                .clone()
                .filter(has_exe)
                .map(|path| (path, Store::Ubisoft))
        })
        .or_else(|| {
            probe
                .epic_manifests
                .as_deref()
                .and_then(|dir| epic_install(dir, game))
                .filter(has_exe)
                .map(|path| (path, Store::Epic))
        })
}

/// Every `"path" "<dir>"` pair in `libraryfolders.vdf`, unescaped.
pub fn steam_library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf_values(vdf, "path")
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

pub fn documents_game_dir(game: GameId, documents: &Path) -> PathBuf {
    documents.join(game.documents_folder())
}

/// Scan order matters: later folders' copies are the ones the player edits.
pub fn mod_folders(game: GameId, install: Option<&Path>, documents: &Path) -> Vec<ModsFolder> {
    let mut folders = Vec::new();
    if let Some(install) = install {
        let game_mods = install.join(MODS_DIR);
        if game == GameId::Anno117 {
            folders.push(ModsFolder {
                path: game_mods.join(UBI_MODS_DIR),
                location: ModLocation::Ubi,
            });
        }
        folders.push(ModsFolder {
            path: game_mods,
            location: ModLocation::Game,
        });
    }
    folders.push(ModsFolder {
        path: documents_game_dir(game, documents).join(MODS_DIR),
        location: ModLocation::Documents,
    });
    folders
}

pub fn game_info(
    game: GameId,
    install: Option<(PathBuf, Store)>,
    documents: &Path,
    running: bool,
) -> GameInfo {
    let game_dir = documents_game_dir(game, documents);
    let mods_path = game_dir.join(MODS_DIR);
    let activation_scheme = match game {
        GameId::Anno1800 => ActivationScheme::ActivationJson,
        GameId::Anno117 => detect_scheme(game, &mods_path),
    };
    let legacy_mod_loader = game == GameId::Anno1800
        && install
            .as_ref()
            .is_some_and(|(path, _)| crate::health::legacy_loader_removable(path));
    let (install_path, store) = match install {
        Some((path, store)) => (Some(path.to_string_lossy().into_owned()), Some(store)),
        None => (None, None),
    };
    GameInfo {
        id: game,
        display_name: game.display_name().to_string(),
        install_path,
        store,
        mods_path: mods_path.to_string_lossy().into_owned(),
        log_path: game_dir
            .join("log")
            .join("mod-loader.log")
            .to_string_lossy()
            .into_owned(),
        running,
        activation_scheme,
        legacy_mod_loader,
    }
}

pub fn system_documents_dir() -> Option<PathBuf> {
    platform::documents_dir()
}

fn steam_install(library: &Path, game: GameId) -> Option<PathBuf> {
    let steamapps = library.join("steamapps");
    let manifest = steamapps.join(format!("appmanifest_{}.acf", game.steam_app_id()));
    let text = fs::read_to_string(manifest).ok()?;
    let dir = vdf_values(&text, "installdir").into_iter().next()?;
    Some(steamapps.join("common").join(dir))
}

fn epic_install(manifests: &Path, game: GameId) -> Option<PathBuf> {
    let entries = fs::read_dir(manifests).ok()?;
    entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "item"))
        .filter_map(|path| fs::read_to_string(path).ok())
        .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
        .find(|item| {
            item["DisplayName"]
                .as_str()
                .is_some_and(|name| name.starts_with(game.epic_title_prefix()))
        })
        .and_then(|item| item["InstallLocation"].as_str().map(PathBuf::from))
}

/// Values of `"key" "value"` pairs in Valve KeyValues text; `\\` unescapes to `\`.
fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let tokens = vdf_strings(text);
    tokens
        .windows(2)
        .filter(|pair| pair[0].eq_ignore_ascii_case(key))
        .map(|pair| pair[1].clone())
        .collect()
}

fn vdf_strings(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut token = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => token.extend(chars.next()),
                other => token.push(other),
            }
        }
        tokens.push(token);
    }
    tokens
}

#[cfg(windows)]
mod platform {
    use std::path::PathBuf;

    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    use crate::GameId;

    fn read(root: winreg::HKEY, key: &str, value: &str) -> Option<String> {
        RegKey::predef(root)
            .open_subkey(key)
            .ok()?
            .get_value(value)
            .ok()
    }

    pub fn steam_root() -> Option<PathBuf> {
        read(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath").map(PathBuf::from)
    }

    pub fn ubisoft_install(game: GameId) -> Option<PathBuf> {
        let key = format!(
            r"SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\{}",
            game.ubisoft_install_id()
        );
        read(HKEY_LOCAL_MACHINE, &key, "InstallDir").map(PathBuf::from)
    }

    pub fn epic_manifests() -> Option<PathBuf> {
        let program_data = std::env::var_os("ProgramData")?;
        Some(PathBuf::from(program_data).join(r"Epic\EpicGamesLauncher\Data\Manifests"))
    }

    pub fn documents_dir() -> Option<PathBuf> {
        let shell = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders";
        read(HKEY_CURRENT_USER, shell, "Personal")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("Documents"))
            })
    }
}

#[cfg(not(windows))]
mod platform {
    use std::path::PathBuf;

    use crate::GameId;

    pub fn steam_root() -> Option<PathBuf> {
        None
    }

    pub fn ubisoft_install(_game: GameId) -> Option<PathBuf> {
        None
    }

    pub fn epic_manifests() -> Option<PathBuf> {
        None
    }

    pub fn documents_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Documents"))
    }
}

/// How Launch starts a game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchTarget {
    /// A store URL, opened by the OS.
    Url(String),
    Exe {
        exe: PathBuf,
        work_dir: PathBuf,
    },
}

/// A Steam copy run as a bare exe hands itself to Ubisoft Connect and never starts, so
/// Steam copies launch through Steam.
pub fn launch_target(game: GameId, install: &Path, store: Store) -> LaunchTarget {
    if store == Store::Steam {
        return LaunchTarget::Url(format!("steam://rungameid/{}", game.steam_app_id()));
    }
    let exe = install.join(game.executable());
    let work_dir = exe.parent().unwrap_or(install).to_path_buf();
    LaunchTarget::Exe { exe, work_dir }
}

/// Windows paths compare without case.
pub fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}
