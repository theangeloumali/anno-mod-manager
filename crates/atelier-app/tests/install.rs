mod common;

use std::fs;

use atelier_app::ErrorCode;
use atelier_core::GameId;
use common::*;

#[test]
fn a_zip_dropped_from_a_path_with_spaces_installs_into_documents() {
    let harness = Harness::new();
    let archive = harness.drop_path("Roman Roads v1.zip");
    let info = modinfo("roman-roads", "1.0.0", GameId::Anno117);
    write_zip(&archive, &[("Roman Roads/modinfo.json", info.as_bytes())]);

    let preview = harness
        .app
        .preview_install(GameId::Anno117, &[archive.to_string_lossy().into_owned()])
        .unwrap();
    assert_eq!(preview.items.len(), 1);
    let result = harness.app.confirm_install(&preview.id, false).unwrap();
    assert_eq!(result.installed, ["roman-roads"]);
    assert!(
        harness
            .documents_mods(GameId::Anno117)
            .join("roman-roads")
            .join("modinfo.json")
            .is_file()
    );
}

#[test]
fn a_nested_zip_inside_a_zip_is_found() {
    let harness = Harness::new();
    let info = modinfo("inner", "1.0.0", GameId::Anno117);
    let inner = zip_bytes(&[("Inner/modinfo.json", info.as_bytes())]);
    let archive = harness.drop_path("bundle.zip");
    write_zip(&archive, &[("inner.zip", &inner)]);
    let preview = harness
        .app
        .preview_install(GameId::Anno117, &[archive.to_string_lossy().into_owned()])
        .unwrap();
    assert_eq!(preview.items[0].mod_id, "inner");
}

#[test]
fn a_rar_drop_is_a_typed_unsupported_error() {
    let harness = Harness::new();
    let archive = harness.drop_path("mod.rar");
    fs::write(&archive, b"Rar!\x1a\x07\x00").unwrap();
    let error = harness
        .app
        .preview_install(GameId::Anno117, &[archive.to_string_lossy().into_owned()])
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotSupported);
    assert_eq!(
        error.message,
        "RAR archives aren't supported yet: extract it and drop the folder instead"
    );
}

#[test]
fn an_empty_archive_is_a_typed_install_error() {
    let harness = Harness::new();
    let archive = harness.drop_path("empty.zip");
    write_zip(&archive, &[]);
    let error = harness
        .app
        .preview_install(GameId::Anno117, &[archive.to_string_lossy().into_owned()])
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Install);
}

#[test]
fn confirming_while_the_game_runs_is_refused_and_the_preview_survives() {
    let harness = Harness::new();
    let archive = harness.drop_path("roads.zip");
    let info = modinfo("roads", "1.0.0", GameId::Anno117);
    write_zip(&archive, &[("Roads/modinfo.json", info.as_bytes())]);
    let preview = harness
        .app
        .preview_install(GameId::Anno117, &[archive.to_string_lossy().into_owned()])
        .unwrap();
    harness.set_running(true);
    let error = harness.app.confirm_install(&preview.id, false).unwrap_err();
    assert_eq!(error.code, ErrorCode::GameRunning);
    harness.set_running(false);
    harness.app.confirm_install(&preview.id, false).unwrap();
}

#[test]
fn cancel_forgets_the_preview() {
    let harness = Harness::new();
    let archive = harness.drop_path("roads.zip");
    let info = modinfo("roads", "1.0.0", GameId::Anno117);
    write_zip(&archive, &[("Roads/modinfo.json", info.as_bytes())]);
    let preview = harness
        .app
        .preview_install(GameId::Anno117, &[archive.to_string_lossy().into_owned()])
        .unwrap();
    harness.app.cancel_install(&preview.id).unwrap();
    let error = harness.app.confirm_install(&preview.id, false).unwrap_err();
    assert_eq!(error.code, ErrorCode::Install);
}
