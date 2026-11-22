mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use atelier_app::ErrorCode;
use atelier_core::GameId;
use atelier_install::InstallItemStatus;
use atelier_modio::TokenStore;
use atelier_modio::{DownloadJob, DownloadStatus};
use common::*;
use md5::{Digest, Md5};
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const HUMANKIND: GameId = GameId::Humankind;
const GUID: &str = "402b91b9e5724da3851e2b6b727b100a";
/// mod.io's game id for Humankind, and the page and file in `fixture("mod.json")`.
const MODIO_PAGE: &str = "/v1/games/2599/mods/4801";
const MODFILE_FOLDER: &str = "4801_77001";

fn long_name(name: &str, version: &str) -> String {
    format!("{name}.{GUID}.{version}")
}

fn community(harness: &Harness) -> PathBuf {
    harness
        .root
        .path()
        .join("My Documents")
        .join("Humankind")
        .join("Community")
}

/// A module's three files under `parent`, as zip entries: module file, manifest, empty bundle.
fn module_entries(parent: &str, long: &str) -> Vec<(String, Vec<u8>)> {
    vec![
        (format!("{parent}/{long}"), b"module".to_vec()),
        (format!("{parent}/{long}.manifest"), b"m".to_vec()),
        (
            format!("{parent}/{}.assetbundle", long.to_lowercase()),
            Vec::new(),
        ),
    ]
}

fn write_module(dir: &Path, long: &str) {
    fs::create_dir_all(dir).unwrap();
    for (name, bytes) in module_entries("", long) {
        fs::write(dir.join(name.trim_start_matches('/')), bytes).unwrap();
    }
}

fn zip_entries(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let files: Vec<(&str, &[u8])> = entries
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    zip_bytes(&files)
}

fn drop_archive(harness: &Harness, name: &str, entries: &[(String, Vec<u8>)]) -> String {
    let path = harness.drop_path(name);
    fs::write(&path, zip_entries(entries)).unwrap();
    path.to_string_lossy().into_owned()
}

fn install_drop(harness: &Harness, archive: &str) {
    let preview = harness
        .app
        .preview_install(HUMANKIND, &[archive.to_owned()])
        .unwrap();
    harness.app.confirm_install(&preview.id, false).unwrap();
}

fn folder_of(harness: &Harness, id: &str) -> Option<String> {
    harness
        .app
        .list_mods(HUMANKIND)
        .unwrap()
        .into_iter()
        .find(|item| item.id == id)
        .map(|item| item.folder)
}

/// Serves page 4801 on Humankind's mod.io game with `archive` as its file.
async fn serve_page(server: &MockServer, archive: Vec<u8>) {
    let mut detail = fixture("mod.json");
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}/v1/files/77001/download", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
    Mock::given(method("GET"))
        .and(path(MODIO_PAGE))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
        .mount(server)
        .await;
}

async fn wait_for_job(harness: &Harness) -> DownloadJob {
    for _ in 0..200 {
        let finished = harness
            .app
            .list_downloads()
            .into_iter()
            .find(|job| matches!(job.status, DownloadStatus::Done | DownloadStatus::Failed));
        if let Some(job) = finished {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("download job never finished");
}

#[test]
fn a_dropped_module_installs_under_atelier_in_community() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    let archive = drop_archive(&harness, "Sigma.zip", &module_entries("Sigma", &long));

    install_drop(&harness, &archive);

    assert!(
        community(&harness)
            .join("Atelier")
            .join(&long)
            .join(&long)
            .is_file()
    );
    assert_eq!(
        folder_of(&harness, "sigma pack"),
        Some(format!("Atelier/{long}"))
    );
}

#[tokio::test]
async fn a_modio_module_installs_under_its_modfile_folder() {
    let server = MockServer::start().await;
    let long = long_name("Sigma Pack", "2.5");
    serve_page(&server, zip_entries(&module_entries("Sigma", &long))).await;
    let harness = harness_for(&server, Some(API_KEY));

    harness.app.subscribe(HUMANKIND, 4801).await.unwrap();
    let job = wait_for_job(&harness).await;

    assert_eq!(job.status, DownloadStatus::Done, "{:?}", job.error);
    let folder = format!("Atelier/{MODFILE_FOLDER}/{long}");
    assert!(community(&harness).join(&folder).join(&long).is_file());
    assert!(!community(&harness).join("mod.io").exists());
    let mods = harness.app.list_mods(HUMANKIND).unwrap();
    assert_eq!(mods[0].folder, folder);
    assert_eq!(mods[0].modio_id, Some(4801));
}

#[tokio::test]
async fn a_modio_module_already_in_modios_folder_fails_and_installs_nothing() {
    let server = MockServer::start().await;
    let long = long_name("Sigma Pack", "2.5");
    serve_page(&server, zip_entries(&module_entries("Sigma", &long))).await;
    let harness = harness_for(&server, Some(API_KEY));
    write_module(
        &community(&harness)
            .join("mod.io")
            .join(MODFILE_FOLDER)
            .join(&long),
        &long,
    );

    harness.app.subscribe(HUMANKIND, 4801).await.unwrap();
    let job = wait_for_job(&harness).await;

    assert_eq!(job.status, DownloadStatus::Failed);
    let error = job.error.unwrap_or_default();
    assert!(error.contains("Community\\mod.io\\4801_77001"), "{error}");
    assert!(!community(&harness).join("Atelier").exists());
    assert_eq!(harness.app.list_mods(HUMANKIND).unwrap().len(), 1);
}

#[test]
fn a_plugin_waits_for_bepinex_then_lands_in_its_plugins_folder() {
    let harness = Harness::with(Options {
        install: Some(HUMANKIND),
        ..Options::default()
    });
    let entries = vec![("Cool.dll".to_string(), b"MZ".to_vec())];
    let archive = drop_archive(&harness, "CoolPlugin.zip", &entries);

    let refused = harness
        .app
        .preview_install(HUMANKIND, std::slice::from_ref(&archive))
        .unwrap();
    assert_eq!(refused.items[0].status, InstallItemStatus::Invalid);
    assert!(
        refused.items[0].checks[0]
            .message
            .contains("Install BepInEx first")
    );
    harness.app.cancel_install(&refused.id).unwrap();

    let bepinex = harness.install_dir().join("BepInEx");
    fs::create_dir_all(bepinex.join("core")).unwrap();
    fs::write(bepinex.join("core/BepInEx.dll"), "").unwrap();
    install_drop(&harness, &archive);

    assert!(bepinex.join("plugins/CoolPlugin/Cool.dll").is_file());
}

#[test]
fn uninstalling_a_workshop_or_game_folder_module_is_refused() {
    let harness = Harness::with(Options {
        install: Some(HUMANKIND),
        ..Options::default()
    });
    let workshop = harness.root.path().join("workshop/content/1124300/3001");
    write_module(&workshop, &long_name("Alpha", "1.0"));
    let shipped = harness.install_dir().join("Public/Modification/Beta");
    write_module(&shipped, &long_name("Beta", "1.0"));

    for (id, place) in [("alpha", "Steam Workshop folder"), ("beta", "game folder")] {
        let refused = harness.app.uninstall_mod(HUMANKIND, id).unwrap_err();
        assert_eq!(refused.code, ErrorCode::ReadOnly);
        assert!(refused.message.contains(place), "{}", refused.message);
    }
    assert!(workshop.is_dir() && shipped.is_dir());
}

#[test]
fn uninstalling_a_modio_module_leaves_no_empty_folder() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    let page = community(&harness).join("Atelier").join("123_456");
    write_module(&page.join(&long), &long);

    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();

    assert!(!page.exists(), "{} was left behind", page.display());
}

fn registry_text(harness: &Harness) -> String {
    fs::read_to_string(harness.humankind_registry()).unwrap()
}

/// Humankind resets a list naming a module that isn't installed to vanilla.
#[test]
fn uninstalling_a_module_takes_it_out_of_the_load_order() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    install_drop(
        &harness,
        &drop_archive(&harness, "Sigma.zip", &module_entries("Sigma", &long)),
    );
    harness.write_humankind_registry("project mercury/sigma pack");

    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();

    assert!(
        registry_text(&harness)
            .contains("<RuntimeModuleConfiguration>project mercury</RuntimeModuleConfiguration>"),
        "{}",
        registry_text(&harness)
    );
}

#[test]
fn uninstalling_works_while_the_load_order_names_a_module_that_is_gone() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    install_drop(
        &harness,
        &drop_archive(&harness, "Sigma.zip", &module_entries("Sigma", &long)),
    );
    harness.write_humankind_registry("project mercury/ghost/sigma pack");

    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();

    assert!(
        registry_text(&harness).contains(
            "<RuntimeModuleConfiguration>project mercury/ghost</RuntimeModuleConfiguration>"
        ),
        "{}",
        registry_text(&harness)
    );
    assert_eq!(folder_of(&harness, "sigma pack"), None);
}

#[test]
fn uninstalling_a_module_the_load_order_does_not_list_leaves_the_registry_alone() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    write_module(&community(&harness).join("Sigma").join(&long), &long);
    harness.write_humankind_registry("project mercury");
    let before = registry_text(&harness);

    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();

    assert_eq!(registry_text(&harness), before);
    assert_eq!(folder_of(&harness, "sigma pack"), None);
}

#[test]
fn a_module_still_in_the_workshop_stays_in_the_load_order_after_uninstall() {
    let harness = Harness::with(Options {
        install: Some(HUMANKIND),
        ..Options::default()
    });
    let long = long_name("Sigma Pack", "2.5");
    write_module(&community(&harness).join("Sigma").join(&long), &long);
    write_module(
        &harness.root.path().join("workshop/content/1124300/3001"),
        &long,
    );
    harness.write_humankind_registry("project mercury/sigma pack");

    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();

    assert!(registry_text(&harness).contains("project mercury/sigma pack"));
}

#[tokio::test]
async fn a_module_removed_by_unsubscribing_leaves_the_load_order() {
    let server = MockServer::start().await;
    let long = long_name("Sigma Pack", "2.5");
    serve_page(&server, zip_entries(&module_entries("Sigma", &long))).await;
    Mock::given(path(format!("{MODIO_PAGE}/subscribe")))
        .respond_with(ResponseTemplate::new(201).set_body_json(fixture("message_201.json")))
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.app.subscribe(HUMANKIND, 4801).await.unwrap();
    assert_eq!(wait_for_job(&harness).await.status, DownloadStatus::Done);
    harness.write_humankind_registry("project mercury/sigma pack");
    harness.tokens.save("signed-in-token").unwrap();

    harness.app.unsubscribe(HUMANKIND, 4801).await.unwrap();

    assert_eq!(folder_of(&harness, "sigma pack"), None);
    assert!(
        !registry_text(&harness).contains("sigma pack"),
        "{}",
        registry_text(&harness)
    );
}

#[test]
fn restoring_is_refused_while_another_copy_is_installed() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    install_drop(
        &harness,
        &drop_archive(&harness, "Sigma.zip", &module_entries("Sigma", &long)),
    );
    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();
    let game_copy = community(&harness).join("mod.io").join("3506667_4753257");
    write_module(&game_copy.join(&long), &long);

    let error = harness
        .app
        .restore_mod(HUMANKIND, "sigma pack")
        .unwrap_err();

    assert!(error.message.contains("mod.io"), "{}", error.message);
    let copies = fs::read_dir(community(&harness).join("Atelier"))
        .map(|dir| dir.count())
        .unwrap_or(0);
    assert_eq!(copies, 0, "the backup must not land beside the other copy");
}

#[test]
fn an_uninstalled_module_is_restored_under_atelier() {
    let harness = Harness::new();
    let long = long_name("Sigma Pack", "2.5");
    let archive = drop_archive(&harness, "Sigma.zip", &module_entries("Sigma", &long));
    install_drop(&harness, &archive);

    harness.app.uninstall_mod(HUMANKIND, "sigma pack").unwrap();
    assert_eq!(folder_of(&harness, "sigma pack"), None);
    harness.app.restore_mod(HUMANKIND, "sigma pack").unwrap();

    let folder = folder_of(&harness, "sigma pack").unwrap();
    assert!(folder.starts_with("Atelier/"), "{folder}");
}
