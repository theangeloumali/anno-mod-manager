mod common;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use atelier_core::GameId;
use atelier_install::{Error, InstallPreview, Limits};
use common::*;

const GAME: GameId = GameId::Anno117;

fn preview_zip(sandbox: &Sandbox, name: &str, entries: &[Entry]) -> InstallPreview {
    let archive = sandbox.drop_path(name);
    write_zip(&archive, entries);
    let installed = sandbox.installed(GAME);
    sandbox
        .engine
        .preview(GAME, &[archive], &installed)
        .unwrap()
}

fn install(sandbox: &Sandbox, folder: &str, id: &str, version: &str) {
    let info = modinfo(id, version, Some(8));
    let path = format!("{folder}/modinfo.json");
    let marker = format!("{folder}/v{version}.txt");
    let preview = preview_zip(
        sandbox,
        &format!("{id}-{version}.zip"),
        &[
            Entry::File(&path, info.as_bytes()),
            Entry::File(&marker, b"x"),
        ],
    );
    sandbox
        .engine
        .confirm(&preview.id, &sandbox.mods(), &sandbox.installed(GAME), true)
        .unwrap();
}

/// Puts a mod straight into the mods folder, as if the user had copied it there.
fn seed(sandbox: &Sandbox, folder: &str, id: &str, version: &str) {
    let info = modinfo(id, version, Some(8));
    write_tree(
        &sandbox.mods().join(folder),
        &[("modinfo.json", &info), (&format!("v{version}.txt"), "x")],
    );
}

fn backup_versions(sandbox: &Sandbox, id: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(sandbox.backups().join("anno117").join(id))
        .map(|entries| {
            entries
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn find_dir(root: &Path, name: &str) -> Option<PathBuf> {
    for entry in fs::read_dir(root).ok()? {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().unwrap() == name {
                return Some(path);
            }
            if let Some(found) = find_dir(&path, name) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn a_new_mod_lands_in_a_mod_id_folder_and_creates_the_mods_dir() {
    let sandbox = Sandbox::new();
    let info = modinfo("Fresh_Mod", "1.0", Some(8));
    let preview = preview_zip(
        &sandbox,
        "Fresh.zip",
        &[Entry::File("Whatever Name/modinfo.json", info.as_bytes())],
    );
    assert!(!sandbox.mods().exists());

    let result = sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(GAME),
            false,
        )
        .unwrap();

    assert_eq!(result.installed, ["fresh-mod"]);
    assert!(result.skipped.is_empty());
    assert_eq!(read_version(&sandbox.mods().join("fresh-mod")), "1.0");
    assert!(!sandbox.staging().join(&preview.id).exists());
}

#[test]
fn a_preview_made_stale_by_a_later_install_is_refused() {
    let sandbox = Sandbox::new();
    let preview = preview_zip(
        &sandbox,
        "keeper-2.0.zip",
        &[Entry::File(
            "Keeper/modinfo.json",
            modinfo("keeper", "2.0", Some(8)).as_bytes(),
        )],
    );
    // A mod.io download lands a newer copy between the preview and the click.
    seed(&sandbox, "Keeper", "keeper", "3.0");

    let error = sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(GAME),
            false,
        )
        .unwrap_err();

    assert!(
        matches!(error, Error::PreviewStale(ref id) if id == "keeper"),
        "{error}"
    );
    assert_eq!(read_version(&sandbox.mods().join("Keeper")), "3.0");
}

#[test]
fn a_same_version_copy_swapped_in_after_the_preview_is_refused() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "Keeper", "keeper", "1.0");
    let preview = preview_zip(
        &sandbox,
        "keeper-2.0.zip",
        &[Entry::File(
            "Keeper/modinfo.json",
            modinfo("keeper", "2.0", Some(8)).as_bytes(),
        )],
    );
    // Another copy with the same version lands in between, with different contents.
    write_tree(
        &sandbox.mods().join("Keeper"),
        &[("textures/extra.dds", "a different build")],
    );

    let error = sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(GAME),
            false,
        )
        .unwrap_err();

    assert!(
        matches!(error, Error::PreviewStale(ref id) if id == "keeper"),
        "{error}"
    );
}

#[test]
fn a_mod_id_that_is_a_windows_device_name_gets_a_safe_folder() {
    let sandbox = Sandbox::new();

    install(&sandbox, "Console Tweaks", "con", "1.0");

    assert!(sandbox.mods().join("con~").is_dir());
    assert!(!sandbox.mods().join("con").exists());
}

#[test]
fn a_device_name_mod_never_shares_a_backup_slot_with_a_lookalike_mod_id() {
    let sandbox = Sandbox::new();
    install(&sandbox, "Con Mod", "con-mod", "1.0");
    install(&sandbox, "Console", "con", "1.0");
    install(&sandbox, "Console", "con", "2.0");

    assert_eq!(backup_versions(&sandbox, "con-mod").len(), 0);
    assert_eq!(backup_versions(&sandbox, "con~").len(), 1);
}

#[test]
fn an_update_replaces_an_installed_mod_whose_mod_id_normalises_to_empty() {
    let sandbox = Sandbox::new();
    // The scan names this one by its folder, "cool-mod"; the update must find it the same way.
    seed(&sandbox, "Cool Mod", "!!!", "1.0");

    install(&sandbox, "Cool Mod", "cool-mod", "2.0");

    assert_eq!(read_version(&sandbox.mods().join("Cool Mod")), "2.0");
    assert!(!sandbox.mods().join("cool-mod").exists());
    assert_eq!(backup_versions(&sandbox, "cool-mod").len(), 1);
}

#[test]
fn an_update_backs_up_the_old_copy_and_keeps_its_folder_name() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "My Layout", "Keeper", "1.0");

    install(&sandbox, "Keeper Renamed", "Keeper", "2.0");

    let mod_dir = sandbox.mods().join("My Layout");
    assert_eq!(read_version(&mod_dir), "2.0");
    assert!(
        !mod_dir.join("v1.0.txt").exists(),
        "old files must not linger"
    );
    assert!(!sandbox.mods().join("keeper").exists());
    let backups = backup_versions(&sandbox, "keeper");
    assert_eq!(backups.len(), 1);
    assert!(backups[0].starts_with("1.0-"), "{backups:?}");
    let backup = sandbox.backups().join("anno117/keeper").join(&backups[0]);
    assert!(backup.join("v1.0.txt").is_file());
}

#[test]
fn downgrades_and_same_versions_are_skipped_unless_opted_in() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "Down", "Down", "2.0");
    seed(&sandbox, "Same", "Same", "1.0");
    let (down, same) = (
        modinfo("Down", "1.0", Some(8)),
        modinfo("Same", "1.0", Some(8)),
    );
    let entries = [
        Entry::File("Down/modinfo.json", down.as_bytes()),
        Entry::File("Same/modinfo.json", same.as_bytes()),
    ];

    let preview = preview_zip(&sandbox, "Mixed.zip", &entries);
    let result = sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(GAME),
            false,
        )
        .unwrap();
    assert!(result.installed.is_empty());
    assert_eq!(result.skipped, ["down", "same"]);
    assert_eq!(read_version(&sandbox.mods().join("Down")), "2.0");

    let preview = preview_zip(&sandbox, "Mixed2.zip", &entries);
    let result = sandbox
        .engine
        .confirm(&preview.id, &sandbox.mods(), &sandbox.installed(GAME), true)
        .unwrap();
    assert_eq!(result.installed, ["down"]);
    assert_eq!(read_version(&sandbox.mods().join("Down")), "1.0");
}

#[test]
fn a_folder_named_like_the_new_mod_but_holding_another_mod_is_left_alone() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "cool-mod", "other-mod", "1.0");

    install(&sandbox, "Cool", "cool-mod", "2.0");

    let untouched = fs::read_to_string(sandbox.mods().join("cool-mod/modinfo.json")).unwrap();
    assert!(untouched.contains("other-mod"));
    assert!(sandbox.mods().join("cool-mod-2/modinfo.json").exists());
    assert!(backup_versions(&sandbox, "other-mod").is_empty());
}

#[test]
fn the_preview_names_the_free_folder_when_the_mod_id_folder_is_taken() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "cool-mod", "other-mod", "1.0");
    let info = modinfo("cool-mod", "2.0", Some(8));
    let preview = preview_zip(
        &sandbox,
        "Cool.zip",
        &[Entry::File("Cool/modinfo.json", info.as_bytes())],
    );

    let messages: Vec<_> = preview.items[0]
        .checks
        .iter()
        .map(|c| c.message.as_str())
        .collect();
    assert!(
        messages.contains(&"Installs to mods\\cool-mod-2"),
        "{messages:?}"
    );
}

#[test]
fn a_mod_id_that_normalises_to_empty_is_invalid_and_never_touches_the_mods_folder() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "keep-me", "keep-me", "1.0");
    let blank = modinfo("!!!", "1", Some(8));
    let preview = preview_zip(
        &sandbox,
        "Blank.zip",
        &[Entry::File("Blank/modinfo.json", blank.as_bytes())],
    );
    assert_eq!(
        preview.items[0].status,
        atelier_install::InstallItemStatus::Invalid
    );

    let result = sandbox
        .engine
        .confirm(&preview.id, &sandbox.mods(), &sandbox.installed(GAME), true)
        .unwrap();

    assert!(result.installed.is_empty());
    assert!(sandbox.mods().join("keep-me/modinfo.json").exists());
}

#[test]
fn wrong_game_and_invalid_items_are_skipped() {
    let sandbox = Sandbox::new();
    let other = modinfo("OtherGame", "1", Some(7));
    let preview = preview_zip(
        &sandbox,
        "Skip.zip",
        &[
            Entry::File("OtherGame/modinfo.json", other.as_bytes()),
            Entry::File("Broken/modinfo.json", b"{ not json"),
        ],
    );

    let result = sandbox
        .engine
        .confirm(&preview.id, &sandbox.mods(), &sandbox.installed(GAME), true)
        .unwrap();

    assert!(result.installed.is_empty());
    assert_eq!(result.skipped.len(), 2);
    assert!(files_under(&sandbox.mods()).is_empty());
}

#[test]
fn confirm_after_cancel_is_an_error() {
    let sandbox = Sandbox::new();
    let info = modinfo("Gone", "1", Some(8));
    let preview = preview_zip(
        &sandbox,
        "Gone.zip",
        &[Entry::File("Gone/modinfo.json", info.as_bytes())],
    );
    sandbox.engine.cancel(&preview.id).unwrap();

    let error = sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(GAME),
            false,
        )
        .unwrap_err();

    assert!(
        matches!(error, Error::PreviewNotFound(ref id) if *id == preview.id),
        "{error}"
    );
    assert!(!sandbox.mods().join("gone").exists());
}

#[test]
fn a_failure_mid_confirm_restores_everything_already_moved() {
    let sandbox = Sandbox::new();
    seed(&sandbox, "AlphaMod", "AlphaMod", "1.0");
    let (alpha, beta) = (
        modinfo("AlphaMod", "2.0", Some(8)),
        modinfo("BetaMod", "1.0", Some(8)),
    );
    let preview = preview_zip(
        &sandbox,
        "Two.zip",
        &[
            Entry::File("AlphaMod/modinfo.json", alpha.as_bytes()),
            Entry::File("BetaMod/modinfo.json", beta.as_bytes()),
        ],
    );
    // Removing BetaMod's staged copy makes the second move fail after AlphaMod is swapped.
    let staged_beta = find_dir(&sandbox.staging().join(&preview.id), "BetaMod").unwrap();
    fs::remove_dir_all(staged_beta).unwrap();

    let error = sandbox.engine.confirm(
        &preview.id,
        &sandbox.mods(),
        &sandbox.installed(GAME),
        false,
    );

    assert!(error.is_err());
    assert_eq!(read_version(&sandbox.mods().join("AlphaMod")), "1.0");
    assert!(sandbox.mods().join("AlphaMod/v1.0.txt").is_file());
    assert!(!sandbox.mods().join("betamod").exists());
    assert!(backup_versions(&sandbox, "alphamod").is_empty());
    let staged_alpha = find_dir(&sandbox.staging().join(&preview.id), "AlphaMod").unwrap();
    assert_eq!(read_version(&staged_alpha), "2.0");
}

#[test]
fn rollback_restores_the_newest_backup() {
    let sandbox = Sandbox::new();
    install(&sandbox, "Roll", "Roll", "1.0");
    install(&sandbox, "Roll", "Roll", "2.0");
    install(&sandbox, "Roll", "Roll", "3.0");

    sandbox
        .engine
        .rollback(GAME, "roll", &sandbox.mods())
        .unwrap();

    let mod_dir = sandbox.mods().join("roll");
    assert_eq!(read_version(&mod_dir), "2.0");
    assert!(mod_dir.join("v2.0.txt").is_file());
    let backups = backup_versions(&sandbox, "roll");
    assert!(backups.iter().any(|b| b.starts_with("1.0-")), "{backups:?}");
    assert!(
        backups.iter().any(|b| b.starts_with("3.0-")),
        "the replaced copy is kept: {backups:?}"
    );
}

#[test]
fn rollback_without_a_backup_is_an_error() {
    let sandbox = Sandbox::new();
    let error = sandbox
        .engine
        .rollback(GAME, "nothing", &sandbox.mods())
        .unwrap_err();
    assert!(
        matches!(error, Error::NoBackup(ref id) if id == "nothing"),
        "{error}"
    );
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[test]
fn pruning_drops_expired_backups_but_keeps_each_mods_newest() {
    let sandbox = Sandbox::with(Some(30), Limits::default());
    let recent = now_millis();
    let backups = sandbox.backups().join("anno117");
    for dir in [
        "solo/1.0-1000".to_string(),
        "busy/1.0-1000".to_string(),
        "busy/1.1-2000".to_string(),
        format!("busy/1.2-{recent}"),
    ] {
        write_tree(&backups.join(dir), &[("modinfo.json", "{}")]);
    }

    sandbox
        .engine
        .prune_backups(GameId::Anno117, &HashSet::new())
        .unwrap();

    assert_eq!(backup_versions(&sandbox, "solo"), ["1.0-1000"]);
    assert_eq!(backup_versions(&sandbox, "busy"), [format!("1.2-{recent}")]);
}

#[test]
fn pruning_without_a_retention_keeps_everything() {
    let sandbox = Sandbox::with(None, Limits::default());
    for dir in ["old/1.0-1000", "old/1.1-2000"] {
        write_tree(
            &sandbox.backups().join("anno117").join(dir),
            &[("modinfo.json", "{}")],
        );
    }

    sandbox
        .engine
        .prune_backups(GameId::Anno117, &HashSet::new())
        .unwrap();

    assert_eq!(backup_versions(&sandbox, "old").len(), 2);
}

#[test]
fn installed_files_skip_mac_metadata() {
    let sandbox = Sandbox::new();
    let info = modinfo("Clean", "1", Some(8));
    let preview = preview_zip(
        &sandbox,
        "Clean.zip",
        &[
            Entry::File("Clean/modinfo.json", info.as_bytes()),
            Entry::File("Clean/.DS_Store", b"junk"),
            Entry::File("__MACOSX/Clean/._modinfo.json", b"junk"),
        ],
    );

    sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(GAME),
            false,
        )
        .unwrap();

    assert_eq!(files_under(&sandbox.mods()), ["clean/modinfo.json"]);
}
