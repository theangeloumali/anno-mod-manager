mod common;

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use atelier_app::{ErrorCode, LoadedPlugin, ReleaseSource};
use atelier_core::GameId;
use common::{Harness, Options, zip_bytes};
use serde_json::json;
use sha2::{Digest, Sha256};
use socket2::{Domain, Socket, Type};
use url::Url;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const HUMANKIND: GameId = GameId::Humankind;
const RELEASE_PATH: &str = "/repos/BepInEx/BepInEx/releases/tags/v5.4.23.6";
const ASSET_PATH: &str = "/download/BepInEx_win_x64_5.4.23.6.zip";
const LIST: &str = "BepInEx/.atelier-installed-files";
const RELEASE_FILES: [&str; 4] = [
    ".doorstop_version",
    "BepInEx/core/BepInEx.dll",
    "doorstop_config.ini",
    "winhttp.dll",
];

fn release_zip() -> Vec<u8> {
    let files: Vec<(&str, &[u8])> = RELEASE_FILES
        .iter()
        .map(|name| (*name, name.as_bytes()))
        .collect();
    zip_bytes(&files)
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn humankind() -> Harness {
    Harness::with(Options {
        install: Some(HUMANKIND),
        ..Options::default()
    })
}

fn source(server: &MockServer, sha: &str) -> ReleaseSource {
    ReleaseSource {
        release_url: Url::parse(&format!("{}{RELEASE_PATH}", server.uri())).unwrap(),
        sha256: sha.to_string(),
    }
}

async fn mount_release(server: &MockServer, digest: &str, asset: Vec<u8>, downloads: u64) {
    Mock::given(method("GET"))
        .and(path(RELEASE_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tag_name": "v5.4.23.6",
            "assets": [
                { "name": "BepInEx_unix_5.4.23.6.zip", "digest": "sha256:00",
                  "browser_download_url": format!("{}/download/BepInEx_unix_5.4.23.6.zip", server.uri()) },
                { "name": "BepInEx_win_x64_5.4.23.6.zip", "digest": format!("sha256:{digest}"),
                  "browser_download_url": format!("{}{ASSET_PATH}", server.uri()) }
            ]
        })))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(ASSET_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(asset))
        .expect(downloads)
        .mount(server)
        .await;
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn github_source_is_pinned_to_the_scripts_release() {
    let github = ReleaseSource::github();
    assert_eq!(
        github.release_url.as_str(),
        "https://api.github.com/repos/BepInEx/BepInEx/releases/tags/v5.4.23.6"
    );
    assert_eq!(
        github.sha256,
        "6c57a714a892efb4e9e1fef63d608a3391f30ec984031c465404e4ee381a0755"
    );
}

#[tokio::test]
async fn installs_the_verified_release_and_records_every_file() {
    let server = MockServer::start().await;
    let asset = release_zip();
    let sha = sha256(&asset);
    mount_release(&server, &sha, asset, 1).await;
    let harness = humankind();
    let install = harness.install_dir();

    let status = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap();

    assert!(status.present && status.installed_by_atelier);
    assert_eq!(status.version.as_deref(), Some("5.4.23.6"));
    for name in RELEASE_FILES {
        assert_eq!(fs::read_to_string(install.join(name)).unwrap(), name);
    }
    let listed = fs::read_to_string(install.join(LIST)).unwrap();
    assert_eq!(listed.lines().collect::<Vec<_>>(), RELEASE_FILES);

    // Already installed: answered from disk, no second download.
    let again = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap();
    assert!(again.installed_by_atelier);
}

#[tokio::test]
async fn a_download_that_does_not_match_the_pin_writes_nothing() {
    let server = MockServer::start().await;
    let sha = sha256(&release_zip());
    mount_release(&server, &sha, zip_bytes(&[("winhttp.dll", b"tampered")]), 1).await;
    let harness = humankind();

    let error = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::Install);
    assert!(error.message.contains("SHA-256"), "{}", error.message);
    assert!(!harness.install_dir().join("winhttp.dll").exists());
    assert!(!harness.install_dir().join("BepInEx").exists());
}

#[tokio::test]
async fn a_release_digest_that_disagrees_with_the_pin_is_refused_before_download() {
    let server = MockServer::start().await;
    let asset = release_zip();
    mount_release(&server, &"0".repeat(64), asset.clone(), 0).await;
    let harness = humankind();

    let error = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha256(&asset)))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::Install);
    assert!(!harness.install_dir().join("winhttp.dll").exists());
}

#[tokio::test]
async fn a_used_up_github_rate_limit_is_reported_as_such() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(RELEASE_PATH))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-ratelimit-remaining", "0")
                .set_body_json(json!({ "message": "API rate limit exceeded" })),
        )
        .mount(&server)
        .await;
    let harness = humankind();

    let error = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &"0".repeat(64)))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::RateLimited);
}

#[tokio::test]
async fn no_network_is_reported_as_offline() {
    // Bound but never listening: connections are refused, and no parallel test's mock server
    // can take the port, as it could once a dropped listener freed it.
    let reserved = Socket::new(Domain::IPV4, Type::STREAM, None).unwrap();
    reserved
        .bind(
            &"127.0.0.1:0"
                .parse::<std::net::SocketAddr>()
                .unwrap()
                .into(),
        )
        .unwrap();
    let port = reserved.local_addr().unwrap().as_socket().unwrap().port();
    let harness = humankind();
    let unreachable = ReleaseSource {
        release_url: Url::parse(&format!("http://127.0.0.1:{port}{RELEASE_PATH}")).unwrap(),
        sha256: "0".repeat(64),
    };

    let error = harness
        .app
        .install_bepinex_from(HUMANKIND, &unreachable)
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::Offline);
}

#[tokio::test]
async fn a_hand_installed_bepinex_is_never_touched() {
    let server = MockServer::start().await;
    let asset = release_zip();
    let sha = sha256(&asset);
    mount_release(&server, &sha, asset, 0).await;
    let harness = humankind();
    let install = harness.install_dir();
    write(&install.join("winhttp.dll"), "mine");
    write(&install.join("BepInEx/core/BepInEx.dll"), "mine");

    let refused = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap_err();
    assert_eq!(refused.code, ErrorCode::InvalidInput);
    let not_ours = harness.app.uninstall_bepinex(HUMANKIND).unwrap_err();
    assert_eq!(not_ours.code, ErrorCode::NotFound);

    assert_eq!(
        fs::read_to_string(install.join("winhttp.dll")).unwrap(),
        "mine"
    );
    let status = harness.app.bepinex_status(HUMANKIND).unwrap();
    assert!(status.present && !status.installed_by_atelier);
}

#[tokio::test]
async fn a_file_already_in_the_way_stops_the_install_and_is_kept() {
    let server = MockServer::start().await;
    let asset = release_zip();
    let sha = sha256(&asset);
    mount_release(&server, &sha, asset, 1).await;
    let harness = humankind();
    let install = harness.install_dir();
    write(&install.join("doorstop_config.ini"), "my settings");

    let error = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::Install);
    assert!(
        error.message.contains("doorstop_config.ini"),
        "{}",
        error.message
    );
    assert_eq!(
        fs::read_to_string(install.join("doorstop_config.ini")).unwrap(),
        "my settings"
    );
    assert!(!install.join("winhttp.dll").exists());
    assert!(!install.join("BepInEx").exists());
}

#[tokio::test]
async fn a_running_game_blocks_install_and_uninstall_but_not_status() {
    let server = MockServer::start().await;
    let harness = humankind();
    harness.set_running(true);

    let install = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &"0".repeat(64)))
        .await
        .unwrap_err();
    assert_eq!(install.code, ErrorCode::GameRunning);
    let uninstall = harness.app.uninstall_bepinex(HUMANKIND).unwrap_err();
    assert_eq!(uninstall.code, ErrorCode::GameRunning);
    assert!(!harness.app.bepinex_status(HUMANKIND).unwrap().present);
}

#[tokio::test]
async fn an_unknown_install_path_is_game_not_found() {
    let server = MockServer::start().await;
    let harness = Harness::new();

    let error = harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &"0".repeat(64)))
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::GameNotFound);
    assert_eq!(
        harness.app.bepinex_status(HUMANKIND).unwrap_err().code,
        ErrorCode::GameNotFound
    );
}

#[tokio::test]
async fn other_games_are_not_supported() {
    let server = MockServer::start().await;
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });

    let install = harness
        .app
        .install_bepinex_from(GameId::Anno117, &source(&server, &"0".repeat(64)))
        .await
        .unwrap_err();
    assert_eq!(install.code, ErrorCode::NotSupported);
    for error in [
        harness.app.uninstall_bepinex(GameId::Anno117).unwrap_err(),
        harness.app.bepinex_status(GameId::Anno117).unwrap_err(),
    ] {
        assert_eq!(error.code, ErrorCode::NotSupported);
    }
}

#[tokio::test]
async fn uninstall_removes_only_the_listed_files() {
    let server = MockServer::start().await;
    let asset = release_zip();
    let sha = sha256(&asset);
    mount_release(&server, &sha, asset, 1).await;
    let harness = humankind();
    let install = harness.install_dir();
    harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap();
    write(
        &install.join("BepInEx/plugins/ZKidz/ModListGuard.dll"),
        "plugin",
    );
    write(&install.join("BepInEx/config/BepInEx.cfg"), "config");

    let status = harness.app.uninstall_bepinex(HUMANKIND).unwrap();

    assert!(!status.present && !status.installed_by_atelier);
    for name in RELEASE_FILES {
        assert!(!install.join(name).exists(), "{name} is still there");
    }
    assert!(!install.join(LIST).exists());
    assert!(!install.join("BepInEx/core").exists());
    assert!(
        install
            .join("BepInEx/plugins/ZKidz/ModListGuard.dll")
            .is_file()
    );
    assert!(install.join("BepInEx/config/BepInEx.cfg").is_file());
    assert!(install.join("Humankind.exe").is_file());
}

#[tokio::test]
async fn uninstall_leaves_no_empty_bepinex_folder_behind() {
    let server = MockServer::start().await;
    let asset = release_zip();
    let sha = sha256(&asset);
    mount_release(&server, &sha, asset, 1).await;
    let harness = humankind();
    harness
        .app
        .install_bepinex_from(HUMANKIND, &source(&server, &sha))
        .await
        .unwrap();

    harness.app.uninstall_bepinex(HUMANKIND).unwrap();

    assert!(!harness.install_dir().join("BepInEx").exists());
}

#[test]
fn a_tampered_list_cannot_reach_outside_the_install() {
    let harness = humankind();
    let install = harness.install_dir();
    let outside = install.parent().unwrap().join("outside.txt");
    write(&outside, "keep");
    write(&install.join(LIST), "../outside.txt\nwinhttp.dll\n");
    write(&install.join("winhttp.dll"), "listed");

    harness.app.uninstall_bepinex(HUMANKIND).unwrap();

    assert_eq!(fs::read_to_string(&outside).unwrap(), "keep");
    assert!(!install.join("winhttp.dll").exists());
}

#[test]
fn status_reads_the_version_and_loaded_plugins_from_the_log() {
    let harness = humankind();
    let install = harness.install_dir();
    write(&install.join("winhttp.dll"), "dll");
    write(&install.join("BepInEx/core/BepInEx.dll"), "dll");
    write(
        &install.join("BepInEx/plugins/ZKidz/ModListGuard.dll"),
        "plugin",
    );
    let log = install.join("BepInEx/LogOutput.log");
    write(
        &log,
        "[Message:   BepInEx] BepInEx 5.4.23.6 - Humankind (11/20/2026 3:55:01 PM)\n\
         [Info   :   BepInEx] Running under Unity v2019.4.40\n\
         [Info   :   BepInEx] 2 plugins to load\n\
         [Info   :   BepInEx] Loading [ModListGuard 0.1.0]\n\
         [Info   :   BepInEx] Loading [City Render Diagnostics 1.2.3]\n\
         [Message:   BepInEx] Chainloader startup complete\n",
    );
    let after_plugins = SystemTime::now() + Duration::from_secs(5);
    fs::File::options()
        .write(true)
        .open(&log)
        .unwrap()
        .set_modified(after_plugins)
        .unwrap();

    let status = harness.app.bepinex_status(HUMANKIND).unwrap();

    assert!(status.present && status.log_found && !status.log_stale);
    assert!(!status.installed_by_atelier);
    assert_eq!(status.version.as_deref(), Some("5.4.23.6"));
    assert_eq!(
        status.plugins,
        vec![
            LoadedPlugin {
                name: "ModListGuard".into(),
                version: "0.1.0".into()
            },
            LoadedPlugin {
                name: "City Render Diagnostics".into(),
                version: "1.2.3".into()
            },
        ]
    );
}

#[test]
fn a_log_older_than_the_plugins_is_stale() {
    let harness = humankind();
    let install = harness.install_dir();
    let log = install.join("BepInEx/LogOutput.log");
    write(&log, "[Info   :   BepInEx] Loading [ModListGuard 0.0.9]\n");
    fs::File::options()
        .write(true)
        .open(&log)
        .unwrap()
        .set_modified(SystemTime::now() - Duration::from_secs(3600))
        .unwrap();
    write(
        &install.join("BepInEx/plugins/ZKidz/ModListGuard.dll"),
        "newer plugin",
    );

    let status = harness.app.bepinex_status(HUMANKIND).unwrap();

    assert!(status.log_found && status.log_stale);
    assert_eq!(status.plugins.len(), 1);
}

#[test]
fn a_log_is_read_only_up_to_its_size_cap() {
    let harness = humankind();
    let mut log = " ".repeat(9 * 1024 * 1024);
    log.push_str("\n[Info   :   BepInEx] Loading [Past The Cap 1.0]\n");
    write(&harness.install_dir().join("BepInEx/LogOutput.log"), &log);

    let status = harness.app.bepinex_status(HUMANKIND).unwrap();

    assert!(status.log_found);
    assert!(status.plugins.is_empty());
}

#[test]
fn status_without_a_log_reports_no_plugins() {
    let harness = humankind();

    let status = harness.app.bepinex_status(HUMANKIND).unwrap();

    assert!(!status.present && !status.log_found && !status.log_stale);
    assert!(status.plugins.is_empty());
    assert_eq!(status.version, None);
}

/// Tauri runs async commands on a multi-threaded runtime, so the future must be Send.
#[test]
fn installing_bepinex_can_run_as_a_tauri_command() {
    fn assert_send<T: Send>(_: &T) {}
    let harness = Harness::new();
    let future = harness.app.install_bepinex(GameId::Humankind);
    assert_send(&future);
}

#[test]
fn a_folder_inside_bepinex_plugins_cannot_be_the_game() {
    let harness = humankind();
    let plugin = harness.install_dir().join("BepInEx/plugins/Fake");
    fs::create_dir_all(&plugin).unwrap();
    fs::write(plugin.join(HUMANKIND.executable()), b"").unwrap();

    let error = harness
        .app
        .set_game_path(HUMANKIND, Some(plugin.to_str().unwrap()))
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::InvalidInput);
}
