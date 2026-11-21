//! BepInEx for Humankind: a pinned, verified install that only ever removes what it added.

use std::cmp::Reverse;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

use atelier_core::GameId;
use atelier_core::health::humankind::bepinex_installed;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;

use crate::App;
use crate::error::{AppError, ErrorCode, Result};

const BEPINEX_VERSION: &str = "5.4.23.6";
const ASSET: &str = "BepInEx_win_x64_5.4.23.6.zip";
const RELEASE_URL: &str = "https://api.github.com/repos/BepInEx/BepInEx/releases/tags/v5.4.23.6";
/// The digest scripts/install-bepinex.sh pins; GitHub's listing and the download must match it.
const PINNED_SHA256: &str = "6c57a714a892efb4e9e1fef63d608a3391f30ec984031c465404e4ee381a0755";
const INSTALLED_LIST: &str = "BepInEx/.atelier-installed-files";
const LOG: &str = "BepInEx/LogOutput.log";
const MAX_RELEASE_JSON: usize = 1024 * 1024;
const MAX_ASSET_BYTES: usize = 16 * 1024 * 1024;
const MAX_UNPACKED_BYTES: u64 = 64 * 1024 * 1024;
/// The version and plugin lines come first in a session's log; the rest is never needed.
const MAX_LOG_BYTES: u64 = 8 * 1024 * 1024;

/// Where the release listing comes from, and the SHA-256 its asset must have.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseSource {
    pub release_url: Url,
    pub sha256: String,
}

impl ReleaseSource {
    pub fn github() -> Self {
        Self {
            release_url: Url::parse(RELEASE_URL).expect("the release URL is a valid constant"),
            sha256: PINNED_SHA256.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BepInExStatus {
    pub present: bool,
    pub version: Option<String>,
    /// Only then can Atelier remove it.
    pub installed_by_atelier: bool,
    /// From the last game session's log, in load order.
    pub plugins: Vec<LoadedPlugin>,
    pub log_found: bool,
    /// A plugin changed after the log was written, so the list may be out of date.
    pub log_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPlugin {
    pub name: String,
    pub version: String,
}

#[derive(Deserialize)]
struct Release {
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    digest: Option<String>,
    browser_download_url: String,
}

impl App {
    pub async fn install_bepinex(&self, game: GameId) -> Result<BepInExStatus> {
        self.install_bepinex_from(game, &ReleaseSource::github())
            .await
    }

    pub async fn install_bepinex_from(
        &self,
        game: GameId,
        source: &ReleaseSource,
    ) -> Result<BepInExStatus> {
        let install = self.bepinex_install(game)?;
        self.guard_writes(game)?;
        if install.join(INSTALLED_LIST).is_file() {
            return Ok(status(&install));
        }
        if bepinex_installed(&install) {
            return Err(AppError::new(
                ErrorCode::InvalidInput,
                "BepInEx is already installed in the game folder. Atelier leaves a copy it didn't install alone.",
            ));
        }
        let files = download(source).await?;
        // Taken only after the download: a std guard can't be held across an await.
        let _guard = self.write_guard(game)?;
        write_files(&install, &files)?;
        Ok(status(&install))
    }

    /// Files Atelier didn't list, such as plugins and BepInEx's own config, stay.
    pub fn uninstall_bepinex(&self, game: GameId) -> Result<BepInExStatus> {
        let install = self.bepinex_install(game)?;
        let _guard = self.write_guard(game)?;
        let list = install.join(INSTALLED_LIST);
        let text = fs::read_to_string(&list).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                AppError::new(
                    ErrorCode::NotFound,
                    "Atelier didn't install this BepInEx, so it won't remove it.",
                )
            } else {
                AppError::io(&error, &list)
            }
        })?;
        let files: Vec<PathBuf> = text
            .lines()
            .filter_map(enclosed)
            .map(|relative| install.join(relative))
            .collect();
        for file in files.iter().chain([&list]) {
            match fs::remove_file(file) {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(AppError::io(&error, file));
                }
                _ => {}
            }
        }
        let mut dirs: Vec<&Path> = files
            .iter()
            .chain([&list])
            .flat_map(|file| {
                file.ancestors()
                    .skip(1)
                    .take_while(|dir| *dir != install.as_path())
            })
            .collect();
        dirs.sort_by_key(|dir| (Reverse(dir.components().count()), *dir));
        dirs.dedup();
        for dir in dirs {
            // Fails on a folder that still holds the player's files, which is the point.
            let _ = fs::remove_dir(dir);
        }
        Ok(status(&install))
    }

    /// Read-only, so it also answers while the game runs.
    pub fn bepinex_status(&self, game: GameId) -> Result<BepInExStatus> {
        Ok(status(&self.bepinex_install(game)?))
    }

    fn bepinex_install(&self, game: GameId) -> Result<PathBuf> {
        if game != GameId::Humankind {
            return Err(AppError::new(
                ErrorCode::NotSupported,
                format!(
                    "Atelier manages BepInEx for Humankind only, not {}.",
                    game.display_name()
                ),
            ));
        }
        self.install(game).map(|(path, _)| path).ok_or_else(|| {
            AppError::new(
                ErrorCode::GameNotFound,
                "Atelier can't find the Humankind install folder.",
            )
        })
    }
}

/// A listed path is used only if it stays inside the game folder.
fn enclosed(line: &str) -> Option<&Path> {
    let path = Path::new(line.trim());
    let inside = path.components().next().is_some()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    inside.then_some(path)
}

async fn download(source: &ReleaseSource) -> Result<Vec<(String, Vec<u8>)>> {
    // GitHub refuses API calls without a User-Agent.
    let client = reqwest::Client::builder()
        .user_agent(concat!("atelier/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(transport)?;
    let listing = fetch(&client, source.release_url.clone(), MAX_RELEASE_JSON).await?;
    let release: Release = serde_json::from_slice(&listing).map_err(|error| {
        install_error(format!(
            "GitHub's BepInEx release listing couldn't be read: {error}"
        ))
    })?;
    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name == ASSET)
        .ok_or_else(|| install_error(format!("The BepInEx release has no {ASSET}.")))?;
    if let Some(digest) = &asset.digest
        && !digest
            .strip_prefix("sha256:")
            .is_some_and(|listed| listed.eq_ignore_ascii_case(&source.sha256))
    {
        return Err(install_error(format!(
            "GitHub lists {ASSET} as {digest}, not the pinned sha256:{}. Nothing was installed.",
            source.sha256
        )));
    }
    let url = Url::parse(&asset.browser_download_url).map_err(|error| {
        install_error(format!(
            "GitHub lists a broken download link for {ASSET}: {error}"
        ))
    })?;
    let bytes = fetch(&client, url, MAX_ASSET_BYTES).await?;
    let actual: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if !actual.eq_ignore_ascii_case(&source.sha256) {
        return Err(install_error(format!(
            "{ASSET} failed its SHA-256 check (expected {}, got {actual}). Nothing was installed.",
            source.sha256
        )));
    }
    unpack(&bytes)
}

async fn fetch(client: &reqwest::Client, url: Url, max_bytes: usize) -> Result<Vec<u8>> {
    let mut response = client.get(url).send().await.map_err(transport)?;
    let status = response.status();
    let quota_used_up = response
        .headers()
        .get("x-ratelimit-remaining")
        .is_some_and(|remaining| remaining == "0");
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS
        || (status == reqwest::StatusCode::FORBIDDEN && quota_used_up)
    {
        return Err(AppError::new(
            ErrorCode::RateLimited,
            "GitHub's download limit for this connection is used up. Try again in an hour.",
        ));
    }
    if !status.is_success() {
        return Err(install_error(format!(
            "GitHub answered {status} for {}.",
            response.url()
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(transport)? {
        if body.len() + chunk.len() > max_bytes {
            return Err(install_error(format!(
                "GitHub sent more than {max_bytes} bytes; nothing was installed."
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn transport(error: reqwest::Error) -> AppError {
    if error.is_connect() || error.is_timeout() {
        AppError::new(
            ErrorCode::Offline,
            "GitHub is unreachable. Check your connection.",
        )
    } else {
        install_error(format!("The BepInEx download failed: {error}"))
    }
}

fn install_error(message: String) -> AppError {
    AppError::new(ErrorCode::Install, message)
}

/// Each file as its `/`-separated path inside the game folder, sorted.
fn unpack(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    let unreadable =
        |error: zip::result::ZipError| install_error(format!("{ASSET} couldn't be read: {error}"));
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(unreadable)?;
    let mut files = Vec::new();
    let mut unpacked = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(unreadable)?;
        if entry.is_dir() {
            continue;
        }
        let relative = entry
            .enclosed_name()
            .filter(|_| !entry.is_symlink())
            .ok_or_else(|| {
                install_error(format!(
                    "{ASSET} holds an unsafe path: {}",
                    String::from_utf8_lossy(entry.name_raw())
                ))
            })?;
        let mut data = Vec::new();
        (&mut entry)
            .take(MAX_UNPACKED_BYTES - unpacked + 1)
            .read_to_end(&mut data)
            .map_err(|error| install_error(format!("{ASSET} couldn't be unpacked: {error}")))?;
        unpacked += data.len() as u64;
        if unpacked > MAX_UNPACKED_BYTES {
            return Err(install_error(format!(
                "{ASSET} unpacks to more than {MAX_UNPACKED_BYTES} bytes."
            )));
        }
        let parts: Vec<String> = relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        files.push((parts.join("/"), data));
    }
    files.sort();
    Ok(files)
}

/// All files or none: anything already there stops it, and a failed write undoes the rest.
fn write_files(install: &Path, files: &[(String, Vec<u8>)]) -> Result<()> {
    if let Some((taken, _)) = files
        .iter()
        .find(|(relative, _)| install.join(relative).symlink_metadata().is_ok())
    {
        return Err(install_error(format!(
            "{taken} is already in the game folder. Atelier won't overwrite a file it didn't add, so nothing was installed."
        )));
    }
    let list: String = files
        .iter()
        .map(|(relative, _)| format!("{relative}\n"))
        .collect();
    let mut journal = Journal::default();
    let written = files
        .iter()
        .try_for_each(|(relative, data)| journal.write(&install.join(relative), data))
        .and_then(|()| journal.write(&install.join(INSTALLED_LIST), list.as_bytes()));
    if written.is_err() {
        journal.undo();
    }
    written
}

#[derive(Default)]
struct Journal {
    files: Vec<PathBuf>,
    dirs: Vec<PathBuf>,
}

impl Journal {
    fn write(&mut self, path: &Path, data: &[u8]) -> Result<()> {
        let missing: Vec<&Path> = path
            .ancestors()
            .skip(1)
            .take_while(|dir| !dir.exists())
            .collect();
        for dir in missing.into_iter().rev() {
            fs::create_dir(dir).map_err(|error| AppError::io(&error, dir))?;
            self.dirs.push(dir.to_path_buf());
        }
        // create_new: a file that appeared since the check is still never overwritten.
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| AppError::io(&error, path))?;
        self.files.push(path.to_path_buf());
        file.write_all(data)
            .map_err(|error| AppError::io(&error, path))
    }

    fn undo(self) {
        for file in self.files.iter().rev() {
            let _ = fs::remove_file(file);
        }
        for dir in self.dirs.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
    }
}

fn status(install: &Path) -> BepInExStatus {
    let present = bepinex_installed(install);
    let installed_by_atelier = install.join(INSTALLED_LIST).is_file();
    let log_path = install.join(LOG);
    let log = fs::File::open(&log_path).ok().and_then(|file| {
        let mut bytes = Vec::new();
        file.take(MAX_LOG_BYTES).read_to_end(&mut bytes).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    });
    let (logged_version, plugins) = log.as_deref().map(parse_log).unwrap_or_default();
    // An Atelier list names the version for sure; a log may be left from an earlier copy.
    let version = if installed_by_atelier {
        Some(BEPINEX_VERSION.to_string())
    } else {
        logged_version
    };
    let log_stale = log.is_some()
        && modified(&log_path).is_some_and(|written| {
            newest_change(&install.join("BepInEx").join("plugins"))
                .is_some_and(|changed| changed > written)
        });
    BepInExStatus {
        present,
        version: version.filter(|_| present),
        installed_by_atelier,
        plugins,
        log_found: log.is_some(),
        log_stale,
    }
}

/// The first line names the version (`] BepInEx 5.4.23.6 - Humankind`); each plugin logs
/// `] Loading [Name 1.2.3]`.
fn parse_log(text: &str) -> (Option<String>, Vec<LoadedPlugin>) {
    let version = text.lines().find_map(|line| {
        let (_, rest) = line.split_once("] BepInEx ")?;
        let version = rest.split(" - ").next()?.trim();
        version
            .starts_with(|first: char| first.is_ascii_digit())
            .then(|| version.to_string())
    });
    let plugins = text
        .lines()
        .filter_map(|line| {
            let (_, rest) = line.split_once("] Loading [")?;
            let (name, version) = rest.trim_end().strip_suffix(']')?.rsplit_once(' ')?;
            Some(LoadedPlugin {
                name: name.to_string(),
                version: version.to_string(),
            })
        })
        .collect();
    (version, plugins)
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn newest_change(dir: &Path) -> Option<SystemTime> {
    fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let kind = entry.file_type().ok()?;
            if kind.is_dir() {
                newest_change(&entry.path())
            } else {
                modified(&entry.path())
            }
        })
        .max()
}
