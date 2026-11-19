mod common;

use std::time::Duration;

use atelier_app::ErrorCode;
use atelier_core::GameId;
use atelier_modio::{DownloadJob, DownloadStatus};
use common::*;
use md5::{Digest, Md5};
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn subscribing_as_a_guest_downloads_and_installs_into_documents() {
    let server = MockServer::start().await;
    let info = modinfo("roman-roads", "1.2.0", GameId::Anno117);
    let archive = zip_bytes(&[("Roman Roads/modinfo.json", info.as_bytes())]);
    let digest = format!("{:x}", Md5::digest(&archive));
    let mut detail = fixture("mod.json");
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}/v1/files/77001/download", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(digest);
    detail["modfile"]["filesize"] = Value::from(archive.len());
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));

    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    let job = wait_for_job(&harness).await;
    assert_eq!(job.status, DownloadStatus::Done, "{:?}", job.error);
    assert_eq!(job.name, "Roman Roads Overhaul");

    let mods = harness.app.list_mods(GameId::Anno117).unwrap();
    assert_eq!(mods[0].id, "roman-roads");
    assert_eq!(mods[0].modio_id, Some(4801));
}

#[tokio::test]
async fn a_failed_download_is_a_failed_job_with_a_reason() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mod_no_modfile.json")))
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    let job = wait_for_job(&harness).await;
    assert_eq!(job.status, DownloadStatus::Failed);
    assert!(job.error.is_some());
}

#[tokio::test]
async fn subscribing_twice_while_downloading_queues_one_job() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(fixture("mod_no_modfile.json"))
                .set_delay(Duration::from_millis(300)),
        )
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));

    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();

    assert_eq!(harness.app.list_downloads().len(), 1);
}

#[tokio::test]
async fn a_retry_refused_while_the_game_runs_leaves_the_job_failed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mod_no_modfile.json")))
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    let failed = wait_for_job(&harness).await;

    harness.set_running(true);
    let refused = harness.app.retry_download(&failed.id).unwrap_err();
    assert_eq!(refused.code, ErrorCode::GameRunning);
    assert_eq!(
        harness.app.list_downloads()[0].status,
        DownloadStatus::Failed
    );

    // Once the game closes the same page can be queued again.
    harness.set_running(false);
    harness.app.retry_download(&failed.id).unwrap();
    assert_ne!(
        harness.app.list_downloads()[0].status,
        DownloadStatus::Failed
    );
}

#[tokio::test]
async fn cancelling_a_download_removes_its_partial_files() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(fixture("mod_no_modfile.json"))
                .set_delay(Duration::from_millis(300)),
        )
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    let job = harness.app.list_downloads().remove(0);
    let partial = harness.root.path().join("data/downloads").join(&job.id);
    std::fs::create_dir_all(&partial).unwrap();
    std::fs::write(partial.join("mod.zip.part"), b"half").unwrap();

    harness.app.cancel_download(&job.id).unwrap();

    assert!(!partial.exists());
}

async fn wait_for_job(harness: &Harness) -> DownloadJob {
    for _ in 0..200 {
        let jobs = harness.app.list_downloads();
        if let Some(job) = jobs
            .into_iter()
            .find(|job| matches!(job.status, DownloadStatus::Done | DownloadStatus::Failed))
        {
            return job;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("download job never finished");
}

/// Serves mod.io page `modio_id`, whose archive holds one mod `mod_id` at `version`.
async fn serve_modio_mod(server: &MockServer, modio_id: u64, mod_id: &str, version: &str) {
    server.reset().await;
    mount_modio_mod(server, modio_id, mod_id, version, false).await;
}

/// Adds mod.io page `modio_id` to what `server` already serves.
async fn mount_modio_mod(
    server: &MockServer,
    modio_id: u64,
    mod_id: &str,
    version: &str,
    has_dependencies: bool,
) {
    let info = modinfo(mod_id, version, GameId::Anno117);
    let archive = zip_bytes(&[("Packed/modinfo.json", info.as_bytes())]);
    let download_path = format!("/v1/files/{modio_id}/download");
    let mut detail = fixture("mod.json");
    detail["id"] = Value::from(modio_id);
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}{download_path}", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
    detail["dependencies"] = Value::from(has_dependencies);
    Mock::given(method("GET"))
        .and(path(format!("/v1/games/11358/mods/{modio_id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(download_path))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
        .mount(server)
        .await;
}

/// Runs one download to the end and clears it, so the next call waits on its own job.
async fn download(harness: &Harness, modio_id: u64) -> DownloadJob {
    harness
        .app
        .subscribe(GameId::Anno117, modio_id)
        .await
        .unwrap();
    let job = wait_for_job(harness).await;
    harness.app.cancel_download(&job.id).unwrap();
    job
}

fn installed(harness: &Harness, mod_id: &str) -> atelier_core::Mod {
    harness
        .app
        .list_mods(GameId::Anno117)
        .unwrap()
        .into_iter()
        .find(|item| item.id == mod_id)
        .unwrap()
}

#[tokio::test]
async fn a_modio_download_never_downgrades_its_own_mod() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "2.0.0").await;
    assert_eq!(download(&harness, 4801).await.status, DownloadStatus::Done);

    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;
    assert_eq!(installed(&harness, "roman-roads").version, "2.0.0");
}

#[tokio::test]
async fn a_modio_download_refuses_to_replace_a_hand_installed_mod() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");
    serve_modio_mod(&server, 4801, "roman-roads", "1.2.0").await;

    let job = download(&harness, 4801).await;
    assert_eq!(job.status, DownloadStatus::Failed);
    let reason = job.error.unwrap_or_default();
    assert!(reason.contains("roman-roads"), "{reason}");
    assert!(
        reason.contains("didn't come from this mod.io page"),
        "{reason}"
    );
    let kept = installed(&harness, "roman-roads");
    assert_eq!((kept.version.as_str(), kept.modio_id), ("1.0.0", None));
}

#[tokio::test]
async fn a_modio_download_refuses_to_replace_another_pages_mod() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 5000, "roman-roads", "1.0.0").await;
    download(&harness, 5000).await;

    serve_modio_mod(&server, 4801, "roman-roads", "1.2.0").await;
    assert_eq!(
        download(&harness, 4801).await.status,
        DownloadStatus::Failed
    );
    let kept = installed(&harness, "roman-roads");
    assert_eq!(
        (kept.version.as_str(), kept.modio_id),
        ("1.0.0", Some(5000))
    );
}

#[tokio::test]
async fn an_update_of_the_same_linked_mod_still_goes_through() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;

    serve_modio_mod(&server, 4801, "roman-roads", "1.2.0").await;
    assert_eq!(download(&harness, 4801).await.status, DownloadStatus::Done);
    let updated = installed(&harness, "roman-roads");
    assert_eq!(
        (updated.version.as_str(), updated.modio_id),
        ("1.2.0", Some(4801))
    );
}

#[tokio::test]
async fn a_new_modio_file_installs_even_when_its_version_is_unchanged() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;

    // The author uploads a fixed file without bumping the manifest version.
    server.reset().await;
    let info = modinfo("roman-roads", "1.0.0", GameId::Anno117);
    let archive = zip_bytes(&[
        ("Packed/modinfo.json", info.as_bytes()),
        ("Packed/hotfix.txt", b"fixed"),
    ]);
    let mut detail = fixture("mod.json");
    let next_file = detail["modfile"]["id"].as_u64().unwrap() + 1;
    detail["modfile"]["id"] = Value::from(next_file);
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}/v1/files/hotfix/download", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/files/hotfix/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
        .mount(&server)
        .await;

    harness
        .app
        .update_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    assert_eq!(wait_for_job(&harness).await.status, DownloadStatus::Done);

    let folder = installed(&harness, "roman-roads").folder;
    let dir = harness.documents_mods(GameId::Anno117).join(folder);
    assert!(dir.join("hotfix.txt").is_file(), "the new file was skipped");
}

#[tokio::test]
async fn a_new_install_never_takes_over_another_pages_link() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 5000, "roman-roads", "1.0.0").await;
    download(&harness, 5000).await;
    // Deleted by hand: the folder goes, but the link to page 5000 stays behind.
    let folder = installed(&harness, "roman-roads").folder;
    std::fs::remove_dir_all(harness.documents_mods(GameId::Anno117).join(folder)).unwrap();

    serve_modio_mod(&server, 4801, "roman-roads", "1.2.0").await;
    download(&harness, 4801).await;
    assert_eq!(installed(&harness, "roman-roads").modio_id, Some(5000));
}

const COLLECTION_PROFILE: &str = "Collection - Province Starter Pack";

/// Serves collection 310 (free page 4801, paid page 4802) for `modio_game`.
async fn serve_collection(server: &MockServer, modio_game: u32) {
    serve_collection_named(server, modio_game, "Province Starter Pack").await;
}

async fn serve_collection_named(server: &MockServer, modio_game: u32, name: &str) {
    let mut collections = fixture("collections.json");
    collections["data"][0]["name"] = Value::from(name);
    Mock::given(method("GET"))
        .and(path(format!("/v1/games/{modio_game}/collections")))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/games/{modio_game}/collections/310/mods")))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("collection_mods.json")))
        .mount(server)
        .await;
}

fn active_profile(harness: &Harness, game: GameId) -> String {
    harness
        .app
        .list_profiles(game)
        .unwrap()
        .into_iter()
        .find(|profile| profile.active)
        .unwrap()
        .name
}

#[tokio::test]
async fn installing_a_collection_saves_and_switches_to_a_profile_named_after_it() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    harness.add_mod(GameId::Anno117, "Off", "kept-off", "1.0.0");
    harness
        .app
        .set_mod_active(GameId::Anno117, "kept-off", false)
        .unwrap();
    serve_collection(&server, 11358).await;

    let profile = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();

    assert_eq!(profile.as_deref(), Some(COLLECTION_PROFILE));
    assert_eq!(
        active_profile(&harness, GameId::Anno117),
        COLLECTION_PROFILE
    );
    assert!(installed(&harness, "kept-on").active);
    assert!(!installed(&harness, "kept-off").active);
    let queued: Vec<u64> = harness
        .app
        .list_downloads()
        .iter()
        .map(|job| job.modio_id)
        .collect();
    // 4802 is a paid mod, which the client drops.
    assert_eq!(queued, [4801]);
    // The previous profile stays in the bar as the way back.
    harness
        .app
        .switch_profile(GameId::Anno117, "Default")
        .unwrap();
    assert!(!installed(&harness, "kept-off").active);
}

#[tokio::test]
async fn a_missing_profiles_folder_is_created_for_the_collection_profile() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    let profiles = harness.documents_mods(GameId::Anno117).join("profiles");
    assert!(!profiles.exists());
    serve_collection(&server, 11358).await;

    harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();

    assert!(profiles.join(format!("{COLLECTION_PROFILE}.txt")).is_file());
}

#[tokio::test]
async fn a_taken_collection_profile_name_gets_a_number() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    harness
        .app
        .save_profile(GameId::Anno117, COLLECTION_PROFILE)
        .unwrap();
    serve_collection(&server, 11358).await;

    let profile = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();

    let expected = format!("{COLLECTION_PROFILE} 2");
    assert_eq!(profile.as_deref(), Some(expected.as_str()));
    assert_eq!(active_profile(&harness, GameId::Anno117), expected);
}

#[tokio::test]
async fn a_game_started_while_a_collection_loads_stops_it_before_any_write() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("collections.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/collections/310/mods"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(fixture("collection_mods.json"))
                .set_delay(Duration::from_millis(400)),
        )
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    let running = harness.running.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        running.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    let error = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::GameRunning);
    assert!(
        !harness
            .documents_mods(GameId::Anno117)
            .join("profiles")
            .exists()
    );
    assert!(harness.app.list_downloads().is_empty());
}

#[tokio::test]
async fn a_collection_never_takes_a_profile_name_that_differs_only_in_unicode_case() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    harness
        .app
        .save_profile(GameId::Anno117, "Collection - Été")
        .unwrap();
    serve_collection_named(&server, 11358, "été").await;

    let profile = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();

    assert_eq!(profile.as_deref(), Some("Collection - été 2"));
}

#[tokio::test]
async fn a_collection_mod_installed_but_off_is_on_in_the_new_profile() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;
    harness
        .app
        .set_mod_active(GameId::Anno117, "roman-roads", false)
        .unwrap();
    serve_collection(&server, 11358).await;

    harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();

    assert!(installed(&harness, "roman-roads").active);
}

#[tokio::test]
async fn anno_1800_installs_a_collection_without_a_profile() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_collection(&server, 4169).await;

    let profile = harness
        .app
        .install_collection(GameId::Anno1800, 310)
        .await
        .unwrap();

    assert_eq!(profile, None);
    assert_eq!(harness.app.list_downloads().len(), 1);
    assert!(
        !harness
            .documents_mods(GameId::Anno1800)
            .join("profiles")
            .exists()
    );
}

#[tokio::test]
async fn a_running_game_refuses_a_collection_install_and_writes_nothing() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    serve_collection(&server, 11358).await;
    harness.set_running(true);

    let error = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::GameRunning);
    assert!(harness.app.list_downloads().is_empty());
    let mods_dir = harness.documents_mods(GameId::Anno117);
    assert!(!mods_dir.join("profiles").exists());
    assert!(!mods_dir.join("profile.txt").exists());
}

/// Installs collection 310 on 117 and waits for its one free download (4801, `roman-roads`).
async fn install_collection_and_download(server: &MockServer, harness: &Harness) {
    serve_modio_mod(server, 4801, "roman-roads", "1.0.0").await;
    serve_collection(server, 11358).await;
    harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();
    let job = wait_for_job(harness).await;
    assert_eq!(job.status, DownloadStatus::Done, "{:?}", job.error);
}

#[tokio::test]
async fn a_downloaded_collection_mod_is_off_in_default() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));

    install_collection_and_download(&server, &harness).await;

    assert!(installed(&harness, "roman-roads").active);
    harness
        .app
        .switch_profile(GameId::Anno117, "Default")
        .unwrap();
    assert!(!installed(&harness, "roman-roads").active);
}

#[tokio::test]
async fn a_collection_mod_already_on_in_default_stays_on_there() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;
    serve_collection(&server, 11358).await;

    harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();
    harness
        .app
        .switch_profile(GameId::Anno117, "Default")
        .unwrap();

    assert!(installed(&harness, "roman-roads").active);
}

#[tokio::test]
async fn an_older_custom_profile_gets_downloaded_collection_mods_off() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.add_mod(GameId::Anno117, "Kept", "kept-on", "1.0.0");
    harness.app.save_profile(GameId::Anno117, "Older").unwrap();

    install_collection_and_download(&server, &harness).await;

    harness
        .app
        .switch_profile(GameId::Anno117, "Older")
        .unwrap();
    assert!(!installed(&harness, "roman-roads").active);
    assert!(installed(&harness, "kept-on").active);
}

#[tokio::test]
async fn a_collection_name_is_cleaned_into_a_valid_profile_name() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_collection_named(&server, 11358, "Rome: Roads & Villas / v2.1").await;

    let profile = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();

    assert_eq!(
        profile.as_deref(),
        Some("Collection - Rome Roads Villas v2 1")
    );
}

#[tokio::test]
async fn a_collection_mods_new_dependency_is_off_in_default_too() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    mount_modio_mod(&server, 4801, "roman-roads", "1.0.0", true).await;
    mount_modio_mod(&server, 4820, "aqueduct-lib", "1.0.0", false).await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801/dependencies"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("dependencies.json")))
        .mount(&server)
        .await;
    serve_collection(&server, 11358).await;

    harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap();
    wait_for_all_jobs(&harness).await;
    harness
        .app
        .switch_profile(GameId::Anno117, "Default")
        .unwrap();

    assert!(!installed(&harness, "aqueduct-lib").active);
}

/// The collection page and its dependency 4820 (4821 is a deleted page, which the client drops).
async fn wait_for_all_jobs(harness: &Harness) {
    for _ in 0..200 {
        let jobs = harness.app.list_downloads();
        if jobs.len() == 2
            && jobs
                .iter()
                .all(|job| matches!(job.status, DownloadStatus::Done | DownloadStatus::Failed))
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "download jobs never finished: {:?}",
        harness.app.list_downloads()
    );
}

#[tokio::test]
async fn undoing_an_uninstall_keeps_the_mods_modio_link() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;

    harness
        .app
        .uninstall_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    harness
        .app
        .restore_mod(GameId::Anno117, "roman-roads")
        .unwrap();

    assert_eq!(installed(&harness, "roman-roads").modio_id, Some(4801));
    harness
        .app
        .update_mod(GameId::Anno117, "roman-roads")
        .unwrap();
}

#[tokio::test]
async fn a_download_that_installs_nothing_is_not_reported_done() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modio_mod(&server, 4801, "roman-roads", "1.0.0").await;
    download(&harness, 4801).await;

    // Same page, same file, same version: nothing to install.
    let again = download(&harness, 4801).await;
    assert_eq!(again.status, DownloadStatus::Failed);
    assert!(
        again
            .error
            .as_deref()
            .is_some_and(|error| error.contains("already installed")),
        "{:?}",
        again.error
    );
}

#[tokio::test]
async fn an_install_that_fails_after_its_preview_leaves_no_staging_behind() {
    let server = MockServer::start().await;
    let info = modinfo("roman-roads", "1.0.0", GameId::Anno117);
    let archive = zip_bytes(&[("Packed/modinfo.json", info.as_bytes())]);
    let mut detail = fixture("mod.json");
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}/v1/files/slow/download", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/files/slow/download"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(archive)
                .set_delay(Duration::from_millis(300)),
        )
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    // The game starts mid-download, so the confirm step refuses.
    tokio::time::sleep(Duration::from_millis(100)).await;
    harness.set_running(true);

    let job = wait_for_job(&harness).await;
    assert_eq!(job.status, DownloadStatus::Failed);
    let staging = harness.root.path().join("data/staging");
    let leftovers = std::fs::read_dir(&staging).map_or(0, |entries| entries.count());
    assert_eq!(leftovers, 0, "abandoned preview staging remains");
}

#[tokio::test]
async fn a_collection_mod_is_on_in_its_profile_even_when_new_mods_start_off() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    let mut settings = harness.app.settings();
    settings.new_mods_start = atelier_app::NewModsStart::Inactive;
    harness.app.save_settings(settings).unwrap();

    install_collection_and_download(&server, &harness).await;

    assert!(
        installed(&harness, "roman-roads").active,
        "the collection's own profile must run its mods"
    );
    harness
        .app
        .switch_profile(GameId::Anno117, "Default")
        .unwrap();
    assert!(!installed(&harness, "roman-roads").active);
}

#[tokio::test]
async fn retrying_a_failed_job_never_runs_beside_an_active_one_for_that_page() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(fixture("mod_no_modfile.json"))
                .set_delay(Duration::from_millis(300)),
        )
        .mount(&server)
        .await;
    let harness = harness_for(&server, Some(API_KEY));
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();
    let failed = wait_for_job(&harness).await;
    assert_eq!(failed.status, DownloadStatus::Failed);
    harness.app.subscribe(GameId::Anno117, 4801).await.unwrap();

    harness.app.retry_download(&failed.id).unwrap();

    let active = harness
        .app
        .list_downloads()
        .into_iter()
        .filter(|job| !matches!(job.status, DownloadStatus::Done | DownloadStatus::Failed))
        .count();
    assert_eq!(active, 1);
}

#[tokio::test]
async fn a_mod_shared_by_two_pending_collections_is_on_in_both_their_profiles() {
    let server = MockServer::start().await;
    let info = modinfo("roman-roads", "1.0.0", GameId::Anno117);
    let archive = zip_bytes(&[("Packed/modinfo.json", info.as_bytes())]);
    let mut detail = fixture("mod.json");
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}/v1/files/shared/download", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(detail)
                .set_delay(Duration::from_millis(400)),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/files/shared/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
        .mount(&server)
        .await;
    // Two collections, 310 and 311, both holding mod 4801.
    let mut collections = fixture("collections.json");
    let mut second = collections["data"][0].clone();
    second["id"] = Value::from(311);
    second["name"] = Value::from("Second Pack");
    collections["data"].as_array_mut().unwrap().push(second);
    collections["result_count"] = Value::from(2);
    collections["result_total"] = Value::from(2);
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/collections"))
        .respond_with(ResponseTemplate::new(200).set_body_json(collections))
        .mount(&server)
        .await;
    for id in [310, 311] {
        Mock::given(method("GET"))
            .and(path(format!("/v1/games/11358/collections/{id}/mods")))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixture("collection_mods.json")))
            .mount(&server)
            .await;
    }
    let harness = harness_for(&server, Some(API_KEY));

    let first = harness
        .app
        .install_collection(GameId::Anno117, 310)
        .await
        .unwrap()
        .unwrap();
    let second = harness
        .app
        .install_collection(GameId::Anno117, 311)
        .await
        .unwrap()
        .unwrap();
    // One download serves both collections.
    let job = wait_for_job(&harness).await;
    assert_eq!(job.status, DownloadStatus::Done, "{:?}", job.error);

    for profile in [&first, &second] {
        harness
            .app
            .switch_profile(GameId::Anno117, profile)
            .unwrap();
        assert!(
            installed(&harness, "roman-roads").active,
            "off in {profile}"
        );
    }
    harness
        .app
        .switch_profile(GameId::Anno117, "Default")
        .unwrap();
    assert!(!installed(&harness, "roman-roads").active);
}

/// Serves page 4801 at `version` from modfile `modfile_id`, for detail, download and update checks.
async fn serve_modfile(server: &MockServer, version: &str, modfile_id: u64) {
    server.reset().await;
    let info = modinfo("roman-roads", version, GameId::Anno117);
    let archive = zip_bytes(&[("Packed/modinfo.json", info.as_bytes())]);
    let mut detail = fixture("mod.json");
    detail["modfile"]["id"] = Value::from(modfile_id);
    detail["modfile"]["version"] = Value::from(version);
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}/v1/files/{modfile_id}/download", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
    let page = serde_json::json!({
        "data": [detail.clone()], "result_count": 1, "result_offset": 0,
        "result_limit": 100, "result_total": 1
    });
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4801"))
        .respond_with(ResponseTemplate::new(200).set_body_json(detail))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .respond_with(ResponseTemplate::new(200).set_body_json(page))
        .mount(server)
        .await;
    Mock::given(method("GET"))
        .and(path(format!("/v1/files/{modfile_id}/download")))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(archive))
        .mount(server)
        .await;
}

#[tokio::test]
async fn restoring_an_older_version_offers_the_update_again() {
    let server = MockServer::start().await;
    let harness = harness_for(&server, Some(API_KEY));
    serve_modfile(&server, "1.0.0", 9001).await;
    download(&harness, 4801).await;
    serve_modfile(&server, "1.2.0", 9002).await;
    harness
        .app
        .update_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    assert_eq!(wait_for_job(&harness).await.status, DownloadStatus::Done);

    harness
        .app
        .restore_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    harness.app.check_updates(GameId::Anno117).await.unwrap();

    let restored = installed(&harness, "roman-roads");
    assert_eq!(restored.version, "1.0.0");
    assert!(restored.update.is_some(), "1.2.0 must be offered again");
}
