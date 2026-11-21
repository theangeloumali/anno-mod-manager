mod common;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use atelier_core::GameId;
use atelier_install::{Error, KeptVersion, Limits};
use common::*;

const ANNO: GameId = GameId::Anno117;
const HUMANKIND: GameId = GameId::Humankind;
const GUID: &str = "402b91b9e5724da3851e2b6b727b100a";

/// Installs `id` at `version` in `folder`, then moves it into the backups as an uninstall would.
fn back_up(sandbox: &Sandbox, folder: &str, id: &str, version: &str) {
    let dir = sandbox.mods().join(folder);
    write_tree(
        &dir,
        &[
            ("modinfo.json", &modinfo(id, version, Some(8))),
            (&format!("v{version}.txt"), "x"),
        ],
    );
    sandbox.engine.remove_to_backup(ANNO, &dir).unwrap();
}

/// A Humankind module folder: no modinfo, just `<Name>.<guid>.<major>.<minor>` files.
fn module_dir(parent: &Path, name: &str, version: &str) -> PathBuf {
    let long = format!("{name}.{GUID}.{version}");
    let dir = parent.join(&long);
    write_tree(
        &dir,
        &[(&long, "module"), (&format!("{long}.manifest"), "m")],
    );
    dir
}

fn versions(sandbox: &Sandbox, game: GameId, id: &str) -> Vec<String> {
    sandbox
        .engine
        .backup_versions(game, id)
        .unwrap()
        .into_iter()
        .map(|backup| backup.version)
        .collect()
}

fn kept(game: GameId, id: &str, version: &str) -> KeptVersion {
    KeptVersion {
        game,
        mod_id: id.into(),
        version: version.into(),
    }
}

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[test]
fn backed_up_versions_are_listed_newest_first() {
    let sandbox = Sandbox::new();
    for version in ["1.0", "2.0", "3.0"] {
        back_up(&sandbox, "Roll", "roll", version);
    }

    assert_eq!(versions(&sandbox, ANNO, "roll"), ["3.0", "2.0", "1.0"]);
    let listed = sandbox.engine.backup_versions(ANNO, "roll").unwrap();
    assert!(listed[0].backed_up_at > listed[1].backed_up_at);
}

#[test]
fn a_mod_without_backups_lists_none() {
    let sandbox = Sandbox::new();
    assert!(versions(&sandbox, ANNO, "never").is_empty());
    assert!(versions(&sandbox, ANNO, "../escape").is_empty());
}

#[test]
fn a_chosen_version_is_restored_and_the_installed_copy_is_kept() {
    let sandbox = Sandbox::new();
    for version in ["1.0", "2.0", "3.0"] {
        back_up(&sandbox, "Roll", "roll", version);
    }
    write_tree(
        &sandbox.mods().join("Roll"),
        &[("modinfo.json", &modinfo("roll", "4.0", Some(8)))],
    );

    sandbox
        .engine
        .restore_version(ANNO, "roll", "2.0", &sandbox.mods(), None)
        .unwrap();

    let dir = sandbox.mods().join("Roll");
    assert_eq!(read_version(&dir), "2.0");
    assert!(dir.join("v2.0.txt").is_file());
    assert_eq!(versions(&sandbox, ANNO, "roll"), ["4.0", "3.0", "1.0"]);
}

#[test]
fn of_two_backups_of_one_version_the_newer_is_restored() {
    let sandbox = Sandbox::new();
    back_up(&sandbox, "Twin", "twin", "1.0");
    write_tree(
        &sandbox.mods().join("Twin"),
        &[
            ("modinfo.json", &modinfo("twin", "1.0", Some(8))),
            ("newer.txt", "x"),
        ],
    );
    sandbox
        .engine
        .remove_to_backup(ANNO, &sandbox.mods().join("Twin"))
        .unwrap();

    sandbox
        .engine
        .restore_version(ANNO, "twin", "1.0", &sandbox.mods(), None)
        .unwrap();

    assert!(sandbox.mods().join("twin/newer.txt").is_file());
}

#[test]
fn a_version_the_vault_lacks_is_an_error_and_changes_nothing() {
    let sandbox = Sandbox::new();
    back_up(&sandbox, "Roll", "roll", "1.0");

    let error = sandbox
        .engine
        .restore_version(ANNO, "roll", "9.9", &sandbox.mods(), None)
        .unwrap_err();

    assert!(matches!(error, Error::NoBackup(_)), "{error}");
    assert_eq!(versions(&sandbox, ANNO, "roll"), ["1.0"]);
    assert!(!sandbox.mods().join("roll").exists());
}

#[test]
fn an_id_that_could_leave_the_backups_is_refused() {
    let sandbox = Sandbox::new();
    for id in ["../roll", "a/b", "..", ""] {
        let error = sandbox
            .engine
            .restore_version(ANNO, id, "1.0", &sandbox.mods(), None)
            .unwrap_err();
        assert!(matches!(error, Error::NoBackup(_)), "{id}: {error}");
    }
}

#[test]
fn an_installed_folder_outside_the_mods_folder_is_refused() {
    let sandbox = Sandbox::new();
    back_up(&sandbox, "Roll", "roll", "1.0");
    let outside = sandbox.root.path().join("elsewhere");

    let error = sandbox
        .engine
        .restore_version(ANNO, "roll", "1.0", &sandbox.mods(), Some(&outside))
        .unwrap_err();

    assert!(matches!(error, Error::UnsafeTarget(_)), "{error}");
    assert_eq!(versions(&sandbox, ANNO, "roll"), ["1.0"]);
}

#[test]
fn humankind_modules_are_backed_up_by_short_name_and_version() {
    let sandbox = Sandbox::new();
    let nested = sandbox.mods().join("mod.io/1_1/ws64");
    let dir = module_dir(&nested, "Sigma Pack", "2.5");

    sandbox.engine.remove_to_backup(HUMANKIND, &dir).unwrap();

    assert_eq!(versions(&sandbox, HUMANKIND, "sigma pack"), ["2.5"]);
    assert!(versions(&sandbox, ANNO, "sigma pack").is_empty());
}

#[test]
fn a_humankind_version_replaces_the_installed_copy_where_it_lives() {
    let sandbox = Sandbox::new();
    let nested = sandbox.mods().join("mod.io/1_1/ws64");
    let old = module_dir(&nested, "Eq's Inheritance", "1.1");
    sandbox.engine.remove_to_backup(HUMANKIND, &old).unwrap();
    let current = module_dir(&nested, "Eq's Inheritance", "2.0");

    sandbox
        .engine
        .restore_version(
            HUMANKIND,
            "eq's inheritance",
            "1.1",
            &sandbox.mods(),
            Some(&current),
        )
        .unwrap();

    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].id, "eq's inheritance");
    assert_eq!(installed[0].version, "1.1");
    assert_eq!(versions(&sandbox, HUMANKIND, "eq's inheritance"), ["2.0"]);
}

#[test]
fn a_humankind_version_with_no_installed_copy_gets_its_own_folder() {
    let sandbox = Sandbox::new();
    let dir = module_dir(&sandbox.mods(), "Sigma Pack", "2.5");
    sandbox.engine.remove_to_backup(HUMANKIND, &dir).unwrap();

    sandbox
        .engine
        .restore_version(HUMANKIND, "sigma pack", "2.5", &sandbox.mods(), None)
        .unwrap();

    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].version, "2.5");
}

#[test]
fn pruning_never_deletes_a_kept_version() {
    let sandbox = Sandbox::with(Some(30), Limits::default());
    let recent = now_millis();
    let backups = sandbox.backups().join("anno117");
    for dir in [
        "busy/1.0-1000".to_string(),
        "busy/1.1-2000".to_string(),
        format!("busy/1.2-{recent}"),
    ] {
        write_tree(&backups.join(dir), &[("modinfo.json", "{}")]);
    }
    let keep = HashSet::from([
        kept(ANNO, "busy", "1.0"),
        kept(GameId::Anno1800, "busy", "1.1"),
    ]);

    sandbox.engine.prune_backups(ANNO, &keep).unwrap();

    assert_eq!(versions(&sandbox, ANNO, "busy"), ["1.2", "1.0"]);
}

#[test]
fn pruning_keeps_a_humankind_version_a_save_needs() {
    let sandbox = Sandbox::with(Some(30), Limits::default());
    let backups = sandbox.backups().join("humankind");
    for dir in [
        "eventful/0.7-1000",
        "eventful/0.8-2000",
        "eventful/0.9-3000",
    ] {
        fs::create_dir_all(backups.join(dir)).unwrap();
    }
    let keep = HashSet::from([kept(HUMANKIND, "eventful", "0.7")]);

    sandbox.engine.prune_backups(HUMANKIND, &keep).unwrap();

    assert_eq!(versions(&sandbox, HUMANKIND, "eventful"), ["0.9", "0.7"]);
}
