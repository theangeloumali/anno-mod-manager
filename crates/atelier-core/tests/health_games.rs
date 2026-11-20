mod common;

use std::fs;
use std::path::Path;

use atelier_core::games::{self, LaunchTarget, Probe};
use atelier_core::health::{self, Environment, log};
use atelier_core::{ActivationScheme, GameId, LogLevel, ModLocation, Severity, Store};
use common::*;

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

#[test]
fn parses_log_lines_with_mod_source_and_continuations() {
    let entries = log::parse(
        "[2026-11-19 13:55:09.882] [warning] my-mod: No matching node for Path \"//A[B='1']\" (data/x.xml:8)\n\
         [2026-11-19 13:55:10.000] [info] Option: my-mod: true\n  continued\n\
         [2026-11-19 13:55:11.000] [error] Something broke\n",
    );
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].time, "2026-11-19 13:55:09.882");
    assert_eq!(entries[0].level, LogLevel::Warning);
    assert_eq!(entries[0].mod_id.as_deref(), Some("my-mod"));
    assert_eq!(
        entries[0].message,
        "No matching node for Path \"//A[B='1']\""
    );
    assert_eq!(entries[0].source.as_deref(), Some("data/x.xml:8"));
    assert_eq!(entries[1].mod_id, None);
    assert_eq!(entries[1].message, "Option: my-mod: true\n  continued");
    assert_eq!(entries[2].level, LogLevel::Error);
}

#[test]
fn extracts_enabled_mods_from_the_log() {
    let entries = log::parse(
        "[t] [info] Enabled mod: C:/x/mods/[A] B/[C] D [SubMod] (sub-id@0.1)\n[t] [info] Enabled mod: C:/x/mods/E  (e@1.0.0)\n",
    );
    let enabled = log::enabled_mods(&entries);
    assert_eq!(enabled[0].id, "sub-id");
    assert_eq!(enabled[0].version, "0.1");
    assert_eq!(enabled[0].path, "C:/x/mods/[A] B/[C] D [SubMod]");
    assert_eq!(enabled[1].path, "C:/x/mods/E");
}

#[test]
fn environment_checks_flag_duplicates_and_leave_the_games_own_python_dll_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("Anno 1800");
    touch(&install.join("Bin/Win64/Anno1800.exe"));
    touch(&install.join("Bin/Win64/python35.dll"));
    let mods_dir = tmp.path().join("mods");
    fs::create_dir(&mods_dir).unwrap();
    let mut twin = test_mod("twin", "Cat", "Twin");
    twin.location = ModLocation::Game;
    let mods = [test_mod("twin", "Cat", "Twin"), twin];

    let checks = health::check_environment(&Environment {
        game: GameId::Anno1800,
        install_path: Some(&install),
        mods_dir: &mods_dir,
        mods: &mods,
    });
    let level = |id: &str| checks.iter().find(|c| c.id == id).unwrap().level;
    assert_eq!(level("game-found"), Severity::Ok);
    assert_eq!(level("mods-writable"), Severity::Ok);
    // python35.dll is also the game's own DLL, so on its own it is no old loader.
    assert_eq!(level("legacy-mod-loader"), Severity::Ok);
    assert_eq!(level("duplicate-ids"), Severity::Warning);
    assert_eq!(level("activation-scheme"), Severity::Ok);
    assert!(
        checks
            .iter()
            .any(|c| c.id == "duplicate-ids" && c.detail.contains("twin"))
    );
    assert!(fs::read_dir(&mods_dir).unwrap().next().is_none());
}

#[test]
fn the_old_1800_mod_loader_is_an_error_named_by_the_renamed_game_dll() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("Anno 1800");
    touch(&install.join("Bin/Win64/Anno1800.exe"));
    touch(&install.join("Bin/Win64/python35.dll"));
    // The old loader's installer renames the game's DLL to make room for its own.
    touch(&install.join("Bin/Win64/python35_ubi.dll"));
    fs::write(install.join("Bin/Win64/python35.dll"), old_loader_bytes()).unwrap();
    let mods_dir = tmp.path().join("mods");
    fs::create_dir(&mods_dir).unwrap();

    let checks = health::check_environment(&Environment {
        game: GameId::Anno1800,
        install_path: Some(&install),
        mods_dir: &mods_dir,
        mods: &[],
    });
    let loader = checks.iter().find(|c| c.id == "legacy-mod-loader").unwrap();
    assert_eq!(loader.level, Severity::Error);
    assert!(loader.title.contains("Old Anno 1800 Mod Loader"));
}

#[test]
fn environment_checks_report_a_missing_game() {
    let tmp = tempfile::tempdir().unwrap();
    let checks = health::check_environment(&Environment {
        game: GameId::Anno117,
        install_path: None,
        mods_dir: &tmp.path().join("missing"),
        mods: &[],
    });
    let level = |id: &str| checks.iter().find(|c| c.id == id).unwrap().level;
    assert_eq!(level("game-found"), Severity::Error);
    assert_eq!(level("mods-writable"), Severity::Warning);
    assert_eq!(level("duplicate-ids"), Severity::Ok);
    assert!(checks.iter().all(|c| c.id != "legacy-mod-loader"));
}

#[test]
fn reads_steam_library_folders() {
    let vdf = r#""libraryfolders" { "0" { "path" "C:\\Program Files (x86)\\Steam" } "1" { "path"   "D:\\SteamLibrary" } }"#;
    assert_eq!(
        games::steam_library_paths(vdf),
        [
            Path::new(r"C:\Program Files (x86)\Steam"),
            Path::new(r"D:\SteamLibrary")
        ]
    );
}

#[test]
fn detects_steam_ubisoft_and_epic_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let library = tmp.path().join("SteamLibrary");
    touch(&library.join("steamapps/appmanifest_3274580.acf"));
    fs::write(
        library.join("steamapps/appmanifest_3274580.acf"),
        "\"AppState\" { \"installdir\" \"Anno 117 - Pax Romana\" }",
    )
    .unwrap();
    let steam_game = library.join("steamapps/common/Anno 117 - Pax Romana");
    touch(&steam_game.join("Bin/Win64/Anno117.exe"));
    let steam = Probe {
        steam_libraries: vec![library],
        ..Probe::default()
    };
    assert_eq!(
        games::detect_install(GameId::Anno117, &steam),
        Some((steam_game, Store::Steam))
    );
    assert_eq!(games::detect_install(GameId::Anno1800, &steam), None);

    let ubi_game = tmp.path().join("Ubisoft/Anno 1800");
    touch(&ubi_game.join("Bin/Win64/Anno1800.exe"));
    let ubisoft = Probe {
        ubisoft_install: Some(ubi_game.clone()),
        ..Probe::default()
    };
    assert_eq!(
        games::detect_install(GameId::Anno1800, &ubisoft),
        Some((ubi_game.clone(), Store::Ubisoft))
    );

    let manifests = tmp.path().join("Manifests");
    fs::create_dir(&manifests).unwrap();
    let item = serde_json::json!({"DisplayName": "Anno 1800", "InstallLocation": ubi_game});
    fs::write(manifests.join("x.item"), item.to_string()).unwrap();
    let epic = Probe {
        epic_manifests: Some(manifests),
        ..Probe::default()
    };
    assert_eq!(
        games::detect_install(GameId::Anno1800, &epic),
        Some((ubi_game, Store::Epic))
    );
}

#[test]
fn builds_mod_folders_and_game_info() {
    let docs = Path::new("/home/u/Documents");
    let install = Path::new("/games/anno117");
    let folders = games::mod_folders(GameId::Anno117, Some(install), docs);
    let summary: Vec<_> = folders
        .iter()
        .map(|f| (f.path.clone(), f.location))
        .collect();
    assert_eq!(
        summary,
        [
            (install.join("mods").join(".ubi"), ModLocation::Ubi),
            (install.join("mods"), ModLocation::Game),
            (
                docs.join("Anno 117 - Pax Romana").join("mods"),
                ModLocation::Documents
            ),
        ]
    );
    assert_eq!(games::mod_folders(GameId::Anno1800, None, docs).len(), 1);

    let info = games::game_info(
        GameId::Anno1800,
        Some((install.to_path_buf(), Store::Steam)),
        docs,
        false,
    );
    assert_eq!(info.display_name, "Anno 1800");
    assert_eq!(info.store, Some(Store::Steam));
    assert_eq!(info.activation_scheme, ActivationScheme::ActivationJson);
    assert_eq!(
        Path::new(&info.log_path),
        docs.join("Anno 1800").join("log").join("mod-loader.log")
    );
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["activationScheme"], "activation-json");
    assert_eq!(json["id"], "anno1800");
    assert!(json["installPath"].is_string());
}

#[test]
fn mod_serializes_to_the_ui_contract() {
    let json = serde_json::to_value(test_mod("a", "Cat", "A")).unwrap();
    for key in [
        "subModIds",
        "sizeBytes",
        "modioId",
        "parentId",
        "hasOptions",
        "knownIssues",
    ] {
        assert!(json.get(key).is_some(), "missing {key}");
    }
    assert_eq!(json["location"], "documents");
    assert_eq!(json["setup"], serde_json::json!({}));
}

/// Bytes carrying the old loader's version-info description, as Windows stores it.
fn old_loader_bytes() -> Vec<u8> {
    "Anno 1800 Mod Loader"
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect()
}

#[test]
fn a_leftover_original_beside_the_games_own_dll_is_no_old_loader() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("Anno 1800");
    fs::create_dir_all(install.join("Bin/Win64")).unwrap();
    fs::write(install.join("Bin/Win64/python35.dll"), b"Python Core").unwrap();
    touch(&install.join("Bin/Win64/python35_ubi.dll"));
    assert_eq!(
        health::legacy_loader(&install),
        health::LegacyLoader::Absent
    );
}

#[test]
fn an_old_loader_whose_original_is_gone_is_still_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("Anno 1800");
    touch(&install.join("Bin/Win64/Anno1800.exe"));
    fs::write(install.join("Bin/Win64/python35.dll"), old_loader_bytes()).unwrap();
    let mods_dir = tmp.path().join("mods");
    fs::create_dir(&mods_dir).unwrap();

    assert_eq!(
        health::legacy_loader(&install),
        health::LegacyLoader::Stranded
    );
    assert!(!health::legacy_loader_removable(&install));
    let checks = health::check_environment(&Environment {
        game: GameId::Anno1800,
        install_path: Some(&install),
        mods_dir: &mods_dir,
        mods: &[],
    });
    let loader = checks.iter().find(|c| c.id == "legacy-mod-loader").unwrap();
    assert_eq!(loader.level, Severity::Error);
    assert!(loader.detail.contains("Verify the game files"));
}

#[test]
fn a_missing_game_python_dll_is_reported_even_without_any_loader() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path().join("Anno 1800");
    touch(&install.join("Bin/Win64/Anno1800.exe"));
    let mods_dir = tmp.path().join("mods");
    fs::create_dir(&mods_dir).unwrap();

    let checks = health::check_environment(&Environment {
        game: GameId::Anno1800,
        install_path: Some(&install),
        mods_dir: &mods_dir,
        mods: &[],
    });
    let loader = checks.iter().find(|c| c.id == "legacy-mod-loader").unwrap();
    assert_eq!(loader.level, Severity::Error);
    assert!(loader.title.contains("python35.dll is missing"));
}

#[test]
fn a_steam_copy_launches_through_steam() {
    let target = games::launch_target(GameId::Anno117, Path::new("D:/Anno 117"), Store::Steam);
    assert_eq!(
        target,
        LaunchTarget::Url("steam://rungameid/3274580".into())
    );
}

#[test]
fn a_ubisoft_copy_runs_its_executable() {
    let install = Path::new("C:/Ubisoft/Anno 1800");
    let target = games::launch_target(GameId::Anno1800, install, Store::Ubisoft);
    assert_eq!(
        target,
        LaunchTarget::Exe {
            exe: install.join("Bin").join("Win64").join("Anno1800.exe"),
            work_dir: install.join("Bin").join("Win64"),
        }
    );
}
