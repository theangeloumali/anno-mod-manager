mod common;

use std::fs;

use atelier_app::{ErrorCode, Settings};
use atelier_core::games::LaunchTarget;
use atelier_core::{GameId, ModLocation, Store};
use common::*;

#[test]
fn no_game_detected_still_lists_every_game_without_an_install() {
    let harness = Harness::new();
    let games = harness.app.list_games().unwrap();
    assert_eq!(games.len(), 3);
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

#[test]
fn toggles_from_many_threads_at_once_are_all_kept() {
    let harness = Harness::new();
    let ids: Vec<String> = (0..12).map(|n| format!("mod-{n}")).collect();
    for id in &ids {
        harness.add_mod(GameId::Anno117, id, id, "1.0.0");
    }
    std::thread::scope(|scope| {
        for id in &ids {
            let app = harness.app.clone();
            scope.spawn(move || app.set_mod_active(GameId::Anno117, id, false).unwrap());
        }
    });

    let mods = harness.app.list_mods(GameId::Anno117).unwrap();
    assert!(
        mods.iter().all(|item| !item.active),
        "a read-modify-write lost a toggle"
    );
}

#[test]
fn a_game_folder_inside_the_mods_folder_is_refused() {
    let harness = Harness::new();
    let fake = harness.documents_mods(GameId::Anno117).join("Evil");
    let exe = fake.join(GameId::Anno117.executable());
    std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
    std::fs::write(&exe, b"not the game").unwrap();

    let error = harness
        .app
        .set_game_path(GameId::Anno117, Some(fake.to_str().unwrap()))
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::InvalidInput);
    let launch = harness.app.launch_game(GameId::Anno117).unwrap_err();
    assert_eq!(launch.code, ErrorCode::GameNotFound);
}

#[test]
fn a_humankind_game_folder_inside_any_mod_root_is_refused() {
    let harness = Harness::with(Options {
        install: Some(GameId::Humankind),
        ..Options::default()
    });
    let install = harness.install_dir();
    let workshop = install.parent().unwrap().parent().unwrap();
    let mod_roots = [
        install.join("Public").join("Modification"),
        workshop.join("workshop").join("content").join("1124300"),
    ];
    for root in mod_roots {
        let fake = root.join("Evil");
        fs::create_dir_all(&fake).unwrap();
        fs::write(fake.join(GameId::Humankind.executable()), b"not the game").unwrap();
        let error = harness
            .app
            .set_game_path(GameId::Humankind, Some(fake.to_str().unwrap()))
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{}", fake.display());
    }
}

#[test]
fn uninstall_keeps_a_backup_that_restore_puts_back() {
    let harness = Harness::new();
    let dir = harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");

    harness
        .app
        .uninstall_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    assert!(!dir.exists());

    harness
        .app
        .restore_mod(GameId::Anno117, "roman-roads")
        .unwrap();
    let mods = harness.app.list_mods(GameId::Anno117).unwrap();
    assert_eq!(mods.len(), 1);
    assert_eq!(mods[0].id, "roman-roads");
    assert_eq!(mods[0].version, "1.0.0");
}

#[test]
fn restore_never_brings_back_the_other_games_backup() {
    let harness = Harness::new();
    let anno117 = harness.add_mod(GameId::Anno117, "Shared", "shared", "1.0.0");
    harness
        .app
        .uninstall_mod(GameId::Anno117, "shared")
        .unwrap();

    assert!(harness.app.restore_mod(GameId::Anno1800, "shared").is_err());
    assert!(harness.app.list_mods(GameId::Anno1800).unwrap().is_empty());

    harness.app.restore_mod(GameId::Anno117, "shared").unwrap();
    assert!(
        anno117
            .parent()
            .unwrap()
            .read_dir()
            .unwrap()
            .next()
            .is_some()
    );
}

#[test]
fn writes_wait_out_a_just_launched_game_before_it_shows_as_running() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });
    harness.add_mod(GameId::Anno117, "Roman Roads", "roman-roads", "1.0.0");

    harness.app.launch_game(GameId::Anno117).unwrap();
    // The process check still says "not running", as it does for the first seconds on Windows.
    let refused = harness
        .app
        .set_mod_active(GameId::Anno117, "roman-roads", false)
        .unwrap_err();

    assert_eq!(refused.code, ErrorCode::GameRunning);
}

/// Anno 1800 with the old community loader installed over the game's own python35.dll.
fn with_legacy_loader() -> (Harness, std::path::PathBuf) {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno1800),
        ..Options::default()
    });
    let bin = harness.install_dir().join("Bin").join("Win64");
    fs::write(bin.join("python35.dll"), old_loader()).unwrap();
    fs::write(bin.join("python35_ubi.dll"), b"game python").unwrap();
    (harness, bin)
}

/// A DLL carrying the old loader's version-info description, as Windows stores it.
fn old_loader() -> Vec<u8> {
    "Anno 1800 Mod Loader"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn shows_legacy_loader(harness: &Harness) -> bool {
    let games = harness.app.list_games().unwrap();
    games
        .iter()
        .find(|game| game.id == GameId::Anno1800)
        .unwrap()
        .legacy_mod_loader
}

#[test]
fn removing_the_old_loader_backs_it_up_and_puts_the_games_dll_back() {
    let (harness, bin) = with_legacy_loader();
    assert!(shows_legacy_loader(&harness));

    let backup = harness
        .app
        .remove_legacy_loader(GameId::Anno1800)
        .unwrap()
        .unwrap();

    assert_eq!(fs::read(bin.join("python35.dll")).unwrap(), b"game python");
    assert!(!bin.join("python35_ubi.dll").exists());
    assert_eq!(fs::read(&backup).unwrap(), old_loader());
    assert!(!shows_legacy_loader(&harness));
    assert!(backup.starts_with(harness.root.path().join("data")));
    let checks = harness.app.local_health_checks(GameId::Anno1800).unwrap();
    let loader = checks.iter().find(|c| c.id == "legacy-mod-loader").unwrap();
    assert_eq!(loader.level, atelier_core::Severity::Ok);
}

#[test]
fn removing_the_old_loader_when_none_is_installed_changes_nothing() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno1800),
        ..Options::default()
    });
    let bin = harness.install_dir().join("Bin").join("Win64");
    fs::write(bin.join("python35.dll"), b"game python").unwrap();

    let error = harness
        .app
        .remove_legacy_loader(GameId::Anno1800)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(fs::read(bin.join("python35.dll")).unwrap(), b"game python");
}

#[test]
fn removing_the_old_loader_waits_until_the_game_is_closed() {
    let (harness, bin) = with_legacy_loader();
    harness.set_running(true);

    let error = harness
        .app
        .remove_legacy_loader(GameId::Anno1800)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::GameRunning);
    assert_eq!(fs::read(bin.join("python35.dll")).unwrap(), old_loader());
    assert_eq!(
        fs::read(bin.join("python35_ubi.dll")).unwrap(),
        b"game python"
    );
}

#[test]
fn a_loader_already_deleted_by_hand_still_gets_the_games_dll_back() {
    let (harness, bin) = with_legacy_loader();
    fs::remove_file(bin.join("python35.dll")).unwrap();

    let backup = harness.app.remove_legacy_loader(GameId::Anno1800).unwrap();

    assert_eq!(backup, None);
    assert_eq!(fs::read(bin.join("python35.dll")).unwrap(), b"game python");
    assert!(!bin.join("python35_ubi.dll").exists());
}

#[test]
fn a_leftover_original_never_replaces_the_games_current_dll() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno1800),
        ..Options::default()
    });
    let bin = harness.install_dir().join("Bin").join("Win64");
    fs::write(bin.join("python35.dll"), b"current game python").unwrap();
    fs::write(bin.join("python35_ubi.dll"), b"older game python").unwrap();

    let error = harness
        .app
        .remove_legacy_loader(GameId::Anno1800)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(
        fs::read(bin.join("python35.dll")).unwrap(),
        b"current game python"
    );
    assert!(!shows_legacy_loader(&harness));
}

#[test]
fn launching_a_steam_copy_goes_through_steam() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });
    harness.app.launch_game(GameId::Anno117).unwrap();
    assert_eq!(
        *harness.launches.lock().unwrap(),
        vec![LaunchTarget::Url("steam://rungameid/3274580".into())]
    );
}

#[test]
fn launching_a_ubisoft_copy_runs_its_executable() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno1800),
        store: Store::Ubisoft,
        ..Options::default()
    });
    harness.app.launch_game(GameId::Anno1800).unwrap();
    let bin = harness.install_dir().join("Bin").join("Win64");
    assert_eq!(
        *harness.launches.lock().unwrap(),
        vec![LaunchTarget::Exe {
            exe: bin.join("Anno1800.exe"),
            work_dir: bin,
        }]
    );
}

#[test]
fn a_picked_folder_that_is_the_steam_install_launches_through_steam() {
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });
    let steam_copy = harness.install_dir();
    harness
        .app
        .set_game_path(GameId::Anno117, Some(steam_copy.to_str().unwrap()))
        .unwrap();

    harness.app.launch_game(GameId::Anno117).unwrap();

    assert_eq!(
        *harness.launches.lock().unwrap(),
        vec![LaunchTarget::Url("steam://rungameid/3274580".into())]
    );
}

#[test]
fn a_picked_copy_steam_does_not_run_starts_its_own_executable() {
    // Steam would start its own install, not the copy picked here.
    let harness = Harness::with(Options {
        install: Some(GameId::Anno117),
        ..Options::default()
    });
    let other = harness.root.path().join("Old Copy").join("Anno 117");
    let exe = other.join(GameId::Anno117.executable());
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"").unwrap();
    harness
        .app
        .set_game_path(GameId::Anno117, Some(other.to_str().unwrap()))
        .unwrap();

    harness.app.launch_game(GameId::Anno117).unwrap();

    assert!(matches!(
        harness.launches.lock().unwrap().as_slice(),
        [LaunchTarget::Exe { exe: launched, .. }] if *launched == exe
    ));
}
