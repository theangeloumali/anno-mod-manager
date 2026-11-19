mod common;

use std::fs;

use atelier_app::{ErrorCode, Settings};
use atelier_core::{GameId, ModLocation, Store};
use common::*;

#[test]
fn no_game_detected_still_lists_both_games_without_an_install() {
    let harness = Harness::new();
    let games = harness.app.list_games().unwrap();
    assert_eq!(games.len(), 2);
    assert!(games.iter().all(|game| game.install_path.is_none()));
    assert!(games[0].mods_path.contains("Anno 117 - Pax Romana"));
}

#[test]
fn anno_1800_without_a_documents_mods_folder_lists_no_mods() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno1800),
        ..Options::default()
    });
    assert!(!harness.documents_mods(GameId::Anno1800).exists());
    assert!(harness.app.list_mods(GameId::Anno1800).unwrap().is_empty());
    assert!(
        harness
            .app
            .list_issues(GameId::Anno1800)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn absent_profiles_folder_means_every_mod_is_active() {
    let harness = Harness::new();
    harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");
    harness.add_mod(GameId::Anno117, "Bigger Docks", "bigger-docks", "2.0.0");
    let mods = harness.app.list_mods(GameId::Anno117).unwrap();
    assert_eq!(mods.len(), 2);
    assert!(mods.iter().all(|item| item.active));
    assert!(
        mods.iter()
            .all(|item| item.location == ModLocation::Documents)
    );
}

#[test]
fn a_running_game_refuses_every_write() {
    let harness = Harness::new();
    harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");
    harness.set_running(true);
    let error = harness
        .app
        .set_mod_active(GameId::Anno117, "roman-roads", false)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::GameRunning);
    assert_eq!(
        error.message,
        "Anno117.exe is running. Close the game to change mods."
    );
    let uninstall = harness
        .app
        .uninstall_mod(GameId::Anno117, "roman-roads")
        .unwrap_err();
    assert_eq!(uninstall.code, ErrorCode::GameRunning);
    assert!(
        harness
            .documents_mods(GameId::Anno117)
            .join("Roman Roads")
            .exists()
    );
}

#[test]
fn toggling_a_mod_off_persists_across_a_fresh_scan() {
    let harness = Harness::new();
    harness.add_mod(GameId::Anno1800, "Ship Speed", "ship-speed", "1.0.0");
    harness
        .app
        .set_mod_active(GameId::Anno1800, "ship-speed", false)
        .unwrap();
    let mods = harness.app.list_mods(GameId::Anno1800).unwrap();
    assert!(!mods[0].active);
}

#[test]
fn uninstall_never_deletes_a_sub_mod_that_shares_the_mod_id() {
    let harness = Harness::new();
    harness.add_mod(GameId::Anno117, "A Pack", "a-pack", "1.0.0");
    let sub_mod = harness.add_mod(GameId::Anno117, "A Pack/x [SubMod]", "x", "1.0.0");
    let real = harness.add_mod(GameId::Anno117, "Z Real", "x", "1.0.0");

    harness.app.uninstall_mod(GameId::Anno117, "x").unwrap();

    assert!(sub_mod.exists(), "the pack's sub-mod must survive");
    assert!(!real.exists());
}

#[test]
fn uninstall_removes_only_the_documents_copy() {
    let harness = Harness::new();
    let dir = harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");
    harness
        .app
        .uninstall_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    assert!(!dir.exists());
    let missing = harness
        .app
        .uninstall_mod(GameId::Anno117, "roman-roads")
        .unwrap_err();
    assert_eq!(missing.code, ErrorCode::NotFound);
}

#[test]
fn a_game_folder_mod_cannot_be_uninstalled() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });
    let dir = harness.install_dir().join("mods").join("Shipped Mod");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("modinfo.json"),
        modinfo("shipped", "1.0.0", GameId::Anno117),
    )
    .unwrap();
    let error = harness
        .app
        .uninstall_mod(GameId::Anno117, "shipped")
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ReadOnly);
    assert_eq!(
        error.message,
        "shipped name lives in the game folder. Atelier only removes mods in Documents."
    );
    assert!(dir.exists());
}

#[test]
fn set_game_path_validates_and_persists() {
    let harness = Harness::new();
    let not_a_game = harness.root.path().join("Empty Folder");
    fs::create_dir_all(&not_a_game).unwrap();
    let error = harness
        .app
        .set_game_path(GameId::Anno117, Some(not_a_game.to_str().unwrap()))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);

    let install = harness
        .root
        .path()
        .join("Games With Spaces")
        .join("Anno 117");
    let exe = install.join(GameId::Anno117.executable());
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"").unwrap();
    harness
        .app
        .set_game_path(GameId::Anno117, Some(install.to_str().unwrap()))
        .unwrap();
    let game = &harness.app.list_games().unwrap()[0];
    assert_eq!(game.install_path.as_deref(), install.to_str());
    assert_eq!(game.store, Some(Store::Manual));

    harness.app.set_game_path(GameId::Anno117, None).unwrap();
    assert!(harness.app.list_games().unwrap()[0].install_path.is_none());
}

#[test]
fn settings_round_trip_and_start_with_the_documented_defaults() {
    let harness = Harness::new();
    let defaults = harness.app.settings();
    assert_eq!(defaults.backup_retention_days, Some(30));
    let changed = Settings {
        backup_retention_days: None,
        ..defaults
    };
    harness.app.save_settings(changed.clone()).unwrap();
    assert_eq!(harness.app.settings(), changed);
}

#[test]
fn profiles_save_switch_and_export() {
    let harness = Harness::new();
    harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");
    harness
        .app
        .save_profile(GameId::Anno117, "Builder")
        .unwrap();
    harness
        .app
        .switch_profile(GameId::Anno117, "Builder")
        .unwrap();
    let profiles = harness.app.list_profiles(GameId::Anno117).unwrap();
    assert!(
        profiles
            .iter()
            .any(|profile| profile.name == "Builder" && profile.active)
    );
    let exported = harness
        .app
        .export_profile(GameId::Anno117, "Builder")
        .unwrap();
    assert!(exported.contains("roman-roads"));
}

#[test]
fn load_order_and_health_cover_the_scanned_mods() {
    let harness = Harness::new();
    harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");
    let order = harness.app.load_order(GameId::Anno117).unwrap();
    assert_eq!(order.entries.len(), 1);
    let checks = harness.app.local_health_checks(GameId::Anno117).unwrap();
    assert!(checks.iter().any(|check| check.id == "game-found"));
}

#[test]
fn a_missing_loader_log_reads_as_empty() {
    let harness = Harness::new();
    assert!(harness.app.loader_log(GameId::Anno117).unwrap().is_empty());
}

#[test]
fn launching_without_an_install_is_a_typed_error() {
    let harness = Harness::new();
    let error = harness.app.launch_game(GameId::Anno117).unwrap_err();
    assert_eq!(error.code, ErrorCode::GameNotFound);
}

#[test]
fn launching_a_running_game_is_refused() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });
    harness.set_running(true);
    let error = harness.app.launch_game(GameId::Anno117).unwrap_err();
    assert_eq!(error.code, ErrorCode::GameRunning);
    assert_eq!(error.message, "Anno117.exe is already running.");
}
