mod common;

use std::fs;

use atelier_core::GameId;
use atelier_install::{Error, Limits};
use common::*;

fn staging_is_empty(sandbox: &Sandbox) -> bool {
    fs::read_dir(sandbox.staging()).map_or(true, |mut entries| entries.next().is_none())
}

fn preview_error(sandbox: &Sandbox, entries: &[Entry]) -> Error {
    let archive = sandbox.drop_path("bad.zip");
    write_zip(&archive, entries);
    let error = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap_err();
    assert!(staging_is_empty(sandbox), "a failed preview leaves staging");
    error
}

#[test]
fn zip_slip_rejects_the_whole_archive() {
    let sandbox = Sandbox::new();
    let error = preview_error(
        &sandbox,
        &[
            Entry::File("Good/modinfo.json", b"{}"),
            Entry::File("../evil.txt", b"pwned"),
        ],
    );

    assert!(matches!(error, Error::UnsafePath { ref entry, .. } if entry == "../evil.txt"));
    assert!(!sandbox.root.path().join("evil.txt").exists());
}

#[test]
fn absolute_and_drive_letter_entries_are_rejected() {
    for name in [
        "/etc/evil.txt",
        "C:/evil.txt",
        "C:\\evil.txt",
        "a/../../evil.txt",
    ] {
        let sandbox = Sandbox::new();
        let error = preview_error(&sandbox, &[Entry::File(name, b"x")]);
        assert!(matches!(error, Error::UnsafePath { .. }), "{name}: {error}");
    }
}

#[test]
fn a_symlink_entry_is_rejected() {
    let sandbox = Sandbox::new();
    let error = preview_error(
        &sandbox,
        &[
            Entry::File("Mod/modinfo.json", b"{}"),
            Entry::Symlink("Mod/link", "/etc/passwd"),
        ],
    );
    assert!(matches!(error, Error::UnsafePath { .. }), "{error}");
}

#[test]
fn a_zip_bomb_over_the_size_cap_fails_before_extracting() {
    let sandbox = Sandbox::with(
        Some(30),
        Limits {
            max_total_bytes: 1024 * 1024,
            ..Limits::default()
        },
    );
    let zeros = vec![0u8; 2 * 1024 * 1024];
    let error = preview_error(&sandbox, &[Entry::File("Bomb/zeros.bin", &zeros)]);
    assert!(matches!(error, Error::TooLarge { .. }), "{error}");
}

#[test]
fn too_many_entries_fails() {
    let sandbox = Sandbox::with(
        Some(30),
        Limits {
            max_entries: 3,
            ..Limits::default()
        },
    );
    let names: Vec<String> = (0..5).map(|n| format!("Mod/f{n}.txt")).collect();
    let entries: Vec<Entry> = names.iter().map(|n| Entry::File(n, b"x")).collect();
    let error = preview_error(&sandbox, &entries);
    assert!(matches!(error, Error::TooManyEntries { .. }), "{error}");
}

#[test]
fn the_default_caps_are_4_gib_and_50k_entries() {
    let limits = Limits::default();
    assert_eq!(limits.max_total_bytes, 4 * 1024 * 1024 * 1024);
    assert_eq!(limits.max_entries, 50_000);
}

#[test]
fn an_empty_archive_is_an_error() {
    let sandbox = Sandbox::new();
    let error = preview_error(&sandbox, &[]);
    assert!(
        matches!(error, Error::EmptySource(ref name) if name == "bad.zip"),
        "{error}"
    );
}

#[test]
fn a_corrupt_zip_is_an_error() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("broken.zip");
    fs::write(&archive, b"PK\x03\x04 this is not really a zip").unwrap();

    let error = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap_err();

    assert!(matches!(error, Error::CorruptArchive { .. }), "{error}");
    assert!(staging_is_empty(&sandbox));
}

#[test]
fn a_rar_is_unsupported_with_a_user_facing_message() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Mod.RAR");
    fs::write(&archive, b"Rar!\x1a\x07\x00").unwrap();

    let error = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap_err();

    assert!(
        matches!(error, Error::Unsupported(ref kind) if kind == "rar"),
        "{error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("RAR") && message.contains("extract"),
        "{message}"
    );
}
