mod common;

use atelier_core::GameId;
use atelier_modio::{DownloadStatus, Error, Modfile, UpdateCheck};
use common::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn update_check_reports_new_removed_and_hidden_mods() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .and(query_param("id-in", "4801,4810,4811,4803"))
        .and(query_param("_limit", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mods_page.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4810"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mod_deleted.json")))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/4811"))
        .respond_with(ResponseTemplate::new(200).set_body_json(fixture("mod_hidden.json")))
        .mount(&server)
        .await;

    let installed = [(4801, 1), (4810, 2), (4811, 3), (4803, 4)];
    let mut checks = anonymous_client(&server)
        .check_updates(GameId::Anno117, &installed)
        .await
        .unwrap();
    checks.sort_by_key(UpdateCheck::mod_id);
    assert_eq!(
        checks.len(),
        3,
        "4803 has zero modfiles, so nothing to offer"
    );
    assert!(
        matches!(&checks[0], UpdateCheck::UpdateAvailable { mod_id: 4801, modfile } if modfile.id == 77001)
    );
    assert!(matches!(checks[1], UpdateCheck::Removed { mod_id: 4810 }));
    assert!(matches!(checks[2], UpdateCheck::Hidden { mod_id: 4811 }));
}

#[tokio::test]
async fn a_missing_mod_that_404s_is_removed_and_current_mods_are_quiet() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(page_of(vec![mod_with(4801, 77001)], 1)),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods/9999"))
        .respond_with(ResponseTemplate::new(404).set_body_json(fixture("error_404.json")))
        .mount(&server)
        .await;

    let checks = anonymous_client(&server)
        .check_updates(GameId::Anno117, &[(4801, 77001), (9999, 1)])
        .await
        .unwrap();
    assert_eq!(checks.len(), 1);
    assert!(matches!(checks[0], UpdateCheck::Removed { mod_id: 9999 }));
}

#[tokio::test]
async fn more_than_100_installed_mods_split_across_calls() {
    let server = MockServer::start().await;
    let first: Vec<_> = (1..=100).map(|id| mod_with(id, id * 10)).collect();
    let rest: Vec<_> = (101..=150).map(|id| mod_with(id, id * 10)).collect();
    let first_ids: Vec<String> = (1..=100).map(|id: u64| id.to_string()).collect();
    let rest_ids: Vec<String> = (101..=150).map(|id: u64| id.to_string()).collect();
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .and(query_param("id-in", first_ids.join(",")))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_of(first, 100)))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/games/11358/mods"))
        .and(query_param("id-in", rest_ids.join(",")))
        .respond_with(ResponseTemplate::new(200).set_body_json(page_of(rest, 50)))
        .expect(1)
        .mount(&server)
        .await;

    let installed: Vec<(u64, u64)> = (1..=150).map(|id| (id, id * 10)).collect();
    let checks = anonymous_client(&server)
        .check_updates(GameId::Anno117, &installed)
        .await
        .unwrap();
    assert!(checks.is_empty(), "every installed modfile is current");
}

fn modfile(server: &MockServer, md5: &str, virus_positive: u8) -> Modfile {
    Modfile {
        id: 77001,
        mod_id: 4801,
        filename: "../escape/roman-roads-overhaul.zip".into(),
        version: Some("1.2.0".into()),
        filesize: 11,
        md5: md5.into(),
        binary_url: format!("{}/v1/files/77001/download", server.uri()),
        virus_status: 1,
        virus_positive,
        filesize_uncompressed: 0,
    }
}

const HELLO_MD5: &str = "5eb63bbbe01eeed093cb22bb8f5acdc3";

#[tokio::test]
async fn download_streams_verifies_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .and(header("Authorization", format!("Bearer {TOKEN}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello world".to_vec()))
        .mount(&server)
        .await;

    let dest = tempfile::tempdir().unwrap();
    let mut statuses = Vec::new();
    let saved = signed_in_client(&server)
        .download(&modfile(&server, HELLO_MD5, 0), dest.path(), |progress| {
            statuses.push((progress.status, progress.downloaded))
        })
        .await
        .unwrap();

    assert_eq!(
        saved,
        dest.path().join("roman-roads-overhaul.zip"),
        "no path traversal"
    );
    assert_eq!(std::fs::read(&saved).unwrap(), b"hello world");
    assert_eq!(statuses.last(), Some(&(DownloadStatus::Done, 11)));
    assert!(statuses.contains(&(DownloadStatus::Verifying, 11)));
    let received = server.received_requests().await.unwrap();
    assert!(
        received[0].url.query().is_none(),
        "no api key on the binary url"
    );
}

#[tokio::test]
async fn an_md5_mismatch_deletes_the_file() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"tampered!!!".to_vec()))
        .mount(&server)
        .await;

    let dest = tempfile::tempdir().unwrap();
    let error = anonymous_client(&server)
        .download(&modfile(&server, HELLO_MD5, 0), dest.path(), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::ChecksumMismatch { ref expected, .. } if expected == HELLO_MD5));
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
}

/// mod.io leaves `filehash.md5` empty on some large files; the listed size stands in for it.
async fn download_without_md5(
    body: &[u8],
) -> (tempfile::TempDir, Result<std::path::PathBuf, Error>) {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.to_vec()))
        .mount(&server)
        .await;
    let dest = tempfile::tempdir().unwrap();
    let saved = anonymous_client(&server)
        .download(&modfile(&server, "", 0), dest.path(), |_| {})
        .await;
    (dest, saved)
}

#[tokio::test]
async fn a_modfile_listed_without_an_md5_downloads_when_its_size_matches() {
    let (_dest, saved) = download_without_md5(b"hello world").await;
    assert_eq!(std::fs::read(saved.unwrap()).unwrap(), b"hello world");
}

#[tokio::test]
async fn a_modfile_listed_without_an_md5_is_refused_when_it_arrives_short() {
    let (dest, saved) = download_without_md5(b"hello").await;
    assert!(matches!(
        saved.unwrap_err(),
        Error::SizeMismatch {
            expected: 11,
            actual: 5
        }
    ));
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn a_modfile_listed_with_neither_an_md5_nor_a_size_is_refused_before_downloading() {
    let server = MockServer::start().await;
    let dest = tempfile::tempdir().unwrap();
    let mut unverifiable = modfile(&server, "", 0);
    unverifiable.filesize = 0;
    let error = anonymous_client(&server)
        .download(&unverifiable, dest.path(), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::Unverifiable { modfile_id: 77001 }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_virus_positive_modfile_is_refused_before_downloading() {
    let server = MockServer::start().await;
    let dest = tempfile::tempdir().unwrap();
    let error = anonymous_client(&server)
        .download(&modfile(&server, HELLO_MD5, 1), dest.path(), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::VirusDetected { modfile_id: 77001 }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn an_unscanned_modfile_is_refused_before_downloading() {
    let server = MockServer::start().await;
    let dest = tempfile::tempdir().unwrap();
    let mut unscanned = modfile(&server, HELLO_MD5, 0);
    unscanned.virus_status = 0;
    let error = anonymous_client(&server)
        .download(&unscanned, dest.path(), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        Error::NotScanned {
            modfile_id: 77001,
            virus_status: 0
        }
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_download_larger_than_its_listed_size_is_stopped_and_deleted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 4096]))
        .mount(&server)
        .await;

    let dest = tempfile::tempdir().unwrap();
    let error = anonymous_client(&server)
        .download(&modfile(&server, HELLO_MD5, 0), dest.path(), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::TooLarge { listed: 11 }), "{error}");
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn a_modfile_listed_as_zero_bytes_accepts_no_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello world".to_vec()))
        .mount(&server)
        .await;
    let mut unlisted = modfile(&server, HELLO_MD5, 0);
    unlisted.filesize = 0;

    let dest = tempfile::tempdir().unwrap();
    let error = anonymous_client(&server)
        .download(&unlisted, dest.path(), |_| {})
        .await
        .unwrap_err();
    assert!(matches!(error, Error::TooLarge { listed: 0 }), "{error}");
    assert_eq!(std::fs::read_dir(dest.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn the_bearer_is_not_sent_to_another_port_on_the_same_host() {
    let api = MockServer::start().await;
    let elsewhere = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello world".to_vec()))
        .mount(&elsewhere)
        .await;

    let dest = tempfile::tempdir().unwrap();
    signed_in_client(&api)
        .download(&modfile(&elsewhere, HELLO_MD5, 0), dest.path(), |_| {})
        .await
        .unwrap();
    let received = elsewhere.received_requests().await.unwrap();
    assert!(!received[0].headers.contains_key("authorization"));
}

#[tokio::test]
async fn a_file_too_large_for_modios_virus_scan_still_downloads() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/files/77001/download"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello world".to_vec()))
        .mount(&server)
        .await;
    let dest = tempfile::tempdir().unwrap();
    let mut too_large = modfile(&server, HELLO_MD5, 0);
    too_large.virus_status = 3;

    let saved = anonymous_client(&server)
        .download(&too_large, dest.path(), |_| {})
        .await
        .unwrap();

    assert!(too_large.scan_skipped());
    assert_eq!(std::fs::read(saved).unwrap(), b"hello world");
}

#[tokio::test]
async fn a_file_still_being_scanned_is_refused_before_downloading() {
    let server = MockServer::start().await;
    let dest = tempfile::tempdir().unwrap();
    let mut scanning = modfile(&server, HELLO_MD5, 0);
    scanning.virus_status = 2;

    let error = anonymous_client(&server)
        .download(&scanning, dest.path(), |_| {})
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::NotScanned {
            virus_status: 2,
            ..
        }
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
}
