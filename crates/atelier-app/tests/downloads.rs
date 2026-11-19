mod common;

use std::time::Duration;

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
    let info = modinfo(mod_id, version, GameId::Anno117);
    let archive = zip_bytes(&[("Packed/modinfo.json", info.as_bytes())]);
    let download_path = format!("/v1/files/{modio_id}/download");
    let mut detail = fixture("mod.json");
    detail["id"] = Value::from(modio_id);
    detail["modfile"]["download"]["binary_url"] =
        Value::String(format!("{}{download_path}", server.uri()));
    detail["modfile"]["filehash"]["md5"] = Value::String(format!("{:x}", Md5::digest(&archive)));
    detail["modfile"]["filesize"] = Value::from(archive.len());
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
