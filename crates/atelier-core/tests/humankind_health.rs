mod common;

use std::fs;
use std::path::{Path, PathBuf};

use atelier_core::health::humankind::{self, SteamUserConfig};
use atelier_core::health::{self, Environment};
use atelier_core::{GameId, HealthCheck, Mod, ModLocation, Severity, games, scan};

const GUID: &str = "a76c474eb14d46cab94bde2e802b5ba8";
const OTHER_GUID: &str = "cda96359ca2548959fe428f029e7fe75";

fn touch(path: &Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "").unwrap();
}

/// A module folder as the game lays it out: `<long>/<long>` under a mods root.
fn add_module(root: &Path, folder: &str, long_name: &str) {
    touch(&root.join(folder).join(long_name));
}

fn hk_mod(id: &str) -> Mod {
    Mod {
        game: GameId::Humankind,
        ..common::test_mod(id, "", id)
    }
}

fn registry(game_dir: &Path, modules: &str) {
    let path = game_dir.join("Users/0x01/Registry.xml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!("<Registry><RuntimeModuleConfiguration>{modules}</RuntimeModuleConfiguration></Registry>"),
    )
    .unwrap();
}

fn localconfig(app_options: &str) -> String {
    format!(
        r#""UserLocalConfigStore" {{ "Software" {{ "Valve" {{ "Steam" {{ "apps" {{
            "289070" {{ "LaunchOptions" "-force-vulkan" }}
            "1124300" {{ "LastPlayed" "1700000000" {app_options} "cloud" {{ "last_sync_state" "synchronized" }} }}
        }} }} }} }} }}"#
    )
}

fn find<'a>(checks: &'a [HealthCheck], id: &str) -> Option<&'a HealthCheck> {
    checks.iter().find(|check| check.id == id)
}

// missing modules

#[test]
fn a_listed_module_that_is_not_installed_is_an_error_naming_it() {
    let listed = common::ids(&["project mercury", "deepsea reloaded", "no city cap"]);
    let check = humankind::missing_modules(&listed, &[hk_mod("deepsea reloaded")]);
    assert_eq!(check.id, "missing-modules");
    assert_eq!(check.level, Severity::Error);
    assert!(check.detail.contains("no city cap"));
    assert!(!check.detail.contains("deepsea reloaded"));
    assert!(!check.detail.contains("project mercury"));
}

#[test]
fn listed_long_names_match_installed_short_names() {
    let listed = vec![
        "project mercury".to_string(),
        format!("DeepSea Reloaded.{GUID}.3.3"),
    ];
    let check = humankind::missing_modules(&listed, &[hk_mod("deepsea reloaded")]);
    assert_eq!(check.level, Severity::Ok);
}

#[test]
fn the_base_game_alone_is_never_missing() {
    let check = humankind::missing_modules(&common::ids(&["project mercury"]), &[]);
    assert_eq!(check.level, Severity::Ok);
}

// duplicate guids

#[test]
fn the_same_guid_in_two_folders_is_a_warning_naming_both() {
    let tmp = tempfile::tempdir().unwrap();
    let community = tmp.path().join("Community");
    let workshop = tmp.path().join("Workshop");
    let long = format!("DeepSea Reloaded.{GUID}.3.3");
    add_module(&community, &long, &long);
    // A renamed copy keeps the guid, so its short name no longer gives it away.
    let renamed = format!("DeepSea Old.{GUID}.3.2");
    add_module(&workshop, "123", &renamed);
    let other = format!("No City Cap.{OTHER_GUID}.1.13");
    add_module(&workshop, "456", &other);
    let folders = [
        common::folder_at(&workshop, ModLocation::Workshop),
        common::documents(&community),
    ];
    let mods = scan::scan(GameId::Humankind, &folders).unwrap().mods;

    let modules = humankind::installed_modules(&folders, &mods);
    assert_eq!(modules.len(), 3);
    let check = humankind::duplicate_guids(&modules);
    assert_eq!(check.id, "duplicate-guids");
    assert_eq!(check.level, Severity::Warning);
    assert!(check.detail.contains(GUID));
    assert!(check.detail.contains("DeepSea Reloaded"));
    assert!(check.detail.contains("DeepSea Old"));
    assert!(!check.detail.contains("No City Cap"));
}

#[test]
fn distinct_guids_are_ok() {
    let tmp = tempfile::tempdir().unwrap();
    let community = tmp.path().join("Community");
    let long = format!("DeepSea Reloaded.{GUID}.3.3");
    add_module(&community, &long, &long);
    let folders = [common::documents(&community)];
    let mods = scan::scan(GameId::Humankind, &folders).unwrap().mods;
    let check = humankind::duplicate_guids(&humankind::installed_modules(&folders, &mods));
    assert_eq!(check.level, Severity::Ok);
}

// pagefile

fn pagefile(entries: &[&str]) -> HealthCheck {
    humankind::pagefile(&common::ids(entries))
}

#[test]
fn a_system_managed_pagefile_warns_and_explains_the_flat_tile_bug() {
    for entries in [
        &[r"?:\pagefile.sys"][..],
        &[r"C:\pagefile.sys 0 0"],
        &[r"C:\pagefile.sys"],
    ] {
        let check = pagefile(entries);
        assert_eq!(check.id, "pagefile");
        assert_eq!(check.level, Severity::Warning, "{entries:?}");
        assert!(check.title.contains("system-managed"), "{entries:?}");
        assert!(check.detail.contains("flat-tile"));
        assert!(check.detail.contains("Huge"));
    }
}

#[test]
fn a_fixed_pagefile_under_32_gb_warns() {
    let check = pagefile(&[r"C:\pagefile.sys 16384 16384"]);
    assert_eq!(check.level, Severity::Warning);
    assert!(check.title.contains("under 32 GB"));
    assert!(check.detail.contains("16 GB"));
}

#[test]
fn fixed_pagefiles_adding_up_to_32_gb_are_ok() {
    assert_eq!(
        pagefile(&[r"C:\pagefile.sys 32768 32768"]).level,
        Severity::Ok
    );
    assert_eq!(
        pagefile(&[
            r"C:\pagefile.sys 8192 16384",
            r"D:\pagefile.sys 16384 16384"
        ])
        .level,
        Severity::Ok
    );
}

#[test]
fn no_pagefile_at_all_warns() {
    let check = pagefile(&[]);
    assert_eq!(check.level, Severity::Warning);
    assert!(check.title.contains("No pagefile"));
}

// -force-vulkan

#[test]
fn reads_launch_options_from_the_apps_own_block_only() {
    let text = localconfig(r#""LaunchOptions" "-windowed -force-vulkan""#);
    assert_eq!(
        humankind::launch_options(&text, 1_124_300),
        vec!["-windowed -force-vulkan".to_string()]
    );
    assert!(humankind::launch_options(&localconfig(""), 1_124_300).is_empty());
}

#[test]
fn force_vulkan_warns_naming_the_steam_users_that_set_it() {
    let configs = [
        SteamUserConfig {
            user: "111".into(),
            localconfig: localconfig(""),
        },
        SteamUserConfig {
            user: "222".into(),
            localconfig: localconfig(r#""LaunchOptions" "-force-vulkan""#),
        },
    ];
    let check = humankind::force_vulkan(&configs).unwrap();
    assert_eq!(check.id, "force-vulkan");
    assert_eq!(check.level, Severity::Warning);
    assert!(check.detail.contains("222"));
    assert!(!check.detail.contains("111"));
}

#[test]
fn force_vulkan_is_ok_when_no_user_sets_it_and_skipped_without_a_config() {
    let configs = [SteamUserConfig {
        user: "111".into(),
        localconfig: localconfig(r#""LaunchOptions" "-windowed""#),
    }];
    assert_eq!(
        humankind::force_vulkan(&configs).unwrap().level,
        Severity::Ok
    );
    assert!(humankind::force_vulkan(&[]).is_none());
}

#[test]
fn finds_localconfig_for_every_steam_user() {
    let tmp = tempfile::tempdir().unwrap();
    let steam = tmp.path().join("Steam");
    for user in ["111", "222"] {
        let path = steam
            .join("userdata")
            .join(user)
            .join("config/localconfig.vdf");
        touch(&path);
        fs::write(path, localconfig("")).unwrap();
    }
    fs::create_dir_all(steam.join("userdata/333/config")).unwrap();
    let configs = humankind::steam_user_configs(&[steam.clone(), steam]);
    let users: Vec<&str> = configs.iter().map(|config| config.user.as_str()).collect();
    assert_eq!(users, ["111", "222"]);
}

// BepInEx and ModListGuard

#[test]
fn bepinex_and_modlistguard_presence_are_ok_rows() {
    let tmp = tempfile::tempdir().unwrap();
    let install = tmp.path();
    assert!(!humankind::bepinex_installed(install));
    let absent = humankind::bepinex(install);
    assert_eq!(
        (absent.id.as_str(), absent.level),
        ("bepinex", Severity::Ok)
    );
    assert!(absent.title.contains("not installed"));

    touch(&install.join("winhttp.dll"));
    touch(&install.join("BepInEx/core/BepInEx.dll"));
    assert!(humankind::bepinex_installed(install));
    assert!(!humankind::bepinex(install).title.contains("not installed"));

    assert!(!humankind::modlistguard_installed(install));
    touch(&install.join("BepInEx/plugins/ZKidz/ModListGuard.dll"));
    assert!(humankind::modlistguard_installed(install));
    let guard = humankind::modlistguard(install);
    assert_eq!(
        (guard.id.as_str(), guard.level),
        ("modlistguard", Severity::Ok)
    );
    assert!(!guard.title.contains("not installed"));
}

// the Humankind arm of check_environment

struct Layout {
    _tmp: tempfile::TempDir,
    install: PathBuf,
    documents: PathBuf,
}

/// `<library>/steamapps/common/Humankind`, with the library also standing in as the Steam root.
fn layout() -> Layout {
    let tmp = tempfile::tempdir().unwrap();
    let library = tmp.path().join("Steam");
    let install = library.join("steamapps/common/Humankind");
    touch(&install.join(GameId::Humankind.executable()));
    let documents = tmp.path().join("Documents");
    fs::create_dir_all(documents.join("Humankind")).unwrap();
    Layout {
        _tmp: tmp,
        install,
        documents,
    }
}

fn humankind_checks(layout: &Layout) -> Vec<HealthCheck> {
    let folders = games::mod_folders(GameId::Humankind, Some(&layout.install), &layout.documents);
    let mods = scan::scan(GameId::Humankind, &folders).unwrap().mods;
    let mods_dir = games::documents_mods_dir(GameId::Humankind, &layout.documents);
    health::check_environment(&Environment {
        game: GameId::Humankind,
        install_path: Some(&layout.install),
        mods_dir: &mods_dir,
        mods: &mods,
    })
}

#[test]
fn the_humankind_arm_runs_every_humankind_check() {
    let layout = layout();
    let game_dir = layout.documents.join("Humankind");
    let community = game_dir.join("Community");
    let long = format!("DeepSea Reloaded.{GUID}.3.3");
    add_module(&community, &long, &long);
    registry(&game_dir, "project mercury/deepsea reloaded/no city cap");
    let user_config = layout
        .install
        .join("../../../userdata/42/config/localconfig.vdf");
    touch(&user_config);
    fs::write(
        &user_config,
        localconfig(r#""LaunchOptions" "-force-vulkan""#),
    )
    .unwrap();

    let checks = humankind_checks(&layout);
    let level = |id: &str| find(&checks, id).unwrap_or_else(|| panic!("no {id}")).level;
    assert_eq!(level("game-found"), Severity::Ok);
    assert_eq!(level("documents-writable"), Severity::Ok);
    assert_eq!(level("missing-modules"), Severity::Error);
    assert!(
        find(&checks, "missing-modules")
            .unwrap()
            .detail
            .contains("no city cap")
    );
    assert_eq!(level("duplicate-guids"), Severity::Ok);
    assert_eq!(level("force-vulkan"), Severity::Warning);
    assert_eq!(level("bepinex"), Severity::Ok);
    assert_eq!(level("modlistguard"), Severity::Ok);
    // The Community folder appears with the first install, so its absence is no warning.
    assert!(find(&checks, "mods-writable").is_none());
    assert!(
        fs::read_dir(&game_dir)
            .unwrap()
            .all(|entry| { entry.unwrap().file_name() != ".atelier-write-probe" })
    );
    #[cfg(not(windows))]
    assert!(find(&checks, "pagefile").is_none());
}

#[test]
fn without_a_registry_or_steam_config_those_rows_are_skipped() {
    let layout = layout();
    let checks = humankind_checks(&layout);
    assert!(find(&checks, "missing-modules").is_none());
    assert!(find(&checks, "force-vulkan").is_none());
    assert_eq!(
        find(&checks, "documents-writable").unwrap().level,
        Severity::Ok
    );
}

#[test]
fn anno_keeps_its_mods_folder_check() {
    let tmp = tempfile::tempdir().unwrap();
    let checks = health::check_environment(&Environment {
        game: GameId::Anno117,
        install_path: None,
        mods_dir: &tmp.path().join("mods"),
        mods: &[],
    });
    assert!(find(&checks, "mods-writable").is_some());
    assert!(find(&checks, "documents-writable").is_none());
    assert!(find(&checks, "bepinex").is_none());
}
