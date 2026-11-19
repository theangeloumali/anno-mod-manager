mod common;

use std::fs;

use atelier_core::scan::{self, SkipReason};
use atelier_core::{
    Difficulty, GameId, ModLocation, ModOptionType, ModOrigin, OptionValue, modinfo,
    normalize_mod_id,
};
use common::*;

#[test]
fn normalizes_mod_ids_like_the_game() {
    assert_eq!(normalize_mod_id("My_Mod Name!!"), "my-mod-name");
    assert_eq!(normalize_mod_id("Foo--Bar"), "foo-bar");
    assert_eq!(normalize_mod_id("*"), "*");
    assert_eq!(normalize_mod_id("-Disabled Mod"), "disabled-mod");
}

#[test]
fn parses_the_117_dependencies_object() {
    let info = modinfo::parse(
        br#"{"Anno":8,"ModID":"No-Road","Version":"1.2.1","Difficulty":"easier",
        "GameSetup":{"SafeToRemove":true,"Multiplayer":true},
        "Category":{"German":"Spiel","English":"Gameplay"},"ModName":{"German":"Keine","English":"No road"},
        "Description":"Free roads","CreatorName":"Nox","KnownIssues":["Breaks saves"],
        "Dependencies":{"Require":["Base_Mod"],"Optional":["opt"],"LoadAfter":["*","Other"],
        "Deprecate":["old"],"Incompatible":["bad"]}}"#,
    )
    .unwrap();
    assert_eq!(info.anno, Some(8));
    assert_eq!(info.game(), GameId::Anno117);
    assert_eq!(info.mod_id.as_deref(), Some("no-road"));
    assert_eq!(info.version, "1.2.1");
    assert_eq!(info.name.as_deref(), Some("No road"));
    assert_eq!(info.category.as_deref(), Some("Gameplay"));
    assert_eq!(info.description.as_deref(), Some("Free roads"));
    assert_eq!(info.author.as_deref(), Some("Nox"));
    assert_eq!(info.difficulty, Some(Difficulty::Easier));
    assert_eq!(info.setup.safe_to_remove, Some(true));
    assert_eq!(info.setup.requires_new_game, None);
    assert_eq!(info.known_issues, ids(&["Breaks saves"]));
    assert_eq!(info.dependencies.require, ids(&["base-mod"]));
    assert_eq!(info.dependencies.optional, ids(&["opt"]));
    assert_eq!(info.dependencies.load_after, ids(&["*", "other"]));
    assert_eq!(info.dependencies.deprecate, ids(&["old"]));
    assert_eq!(info.dependencies.incompatible, ids(&["bad"]));
}

#[test]
fn parses_the_flat_1800_keys() {
    let info = modinfo::parse(
        br#"{"ModID":"Better Ships","Version":"2.0","ModName":{"English":"Better Ships"},
        "ModDependencies":["Shared_Lib"],"OptionalDependencies":["x"],"LoadAfterIds":["y"],
        "IncompatibleIds":["z"],"DeprecateIds":["w"],
        "DLCDependencies":[{"DLC":"Docklands","Dependant":"required"}],"ModioResourceId":1234}"#,
    )
    .unwrap();
    assert_eq!(info.anno, None);
    assert_eq!(info.game(), GameId::Anno1800);
    assert_eq!(info.mod_id.as_deref(), Some("better-ships"));
    assert_eq!(info.dependencies.require, ids(&["shared-lib"]));
    assert_eq!(info.dependencies.optional, ids(&["x"]));
    assert_eq!(info.dependencies.load_after, ids(&["y"]));
    assert_eq!(info.dependencies.incompatible, ids(&["z"]));
    assert_eq!(info.dependencies.deprecate, ids(&["w"]));
    assert_eq!(info.dlc_dependencies[0].dlc, "Docklands");
    assert_eq!(info.dlc_dependencies[0].dependant, "required");
    assert_eq!(info.modio_id, Some(1234));
}

#[test]
fn defaults_the_version_and_falls_back_to_the_first_language() {
    let info = modinfo::parse(br#"{"ModID":"x","ModName":{"Polish":"Nazwa"},"Category":"Plain"}"#)
        .unwrap();
    assert_eq!(info.version, "1.0.0");
    assert_eq!(info.name.as_deref(), Some("Nazwa"));
    assert_eq!(info.category.as_deref(), Some("Plain"));
}

#[test]
fn tolerates_bom_comments_and_trailing_commas() {
    let bytes = fs::read(fixture("core/modinfo-bom-jsonc.jsonc")).unwrap();
    let info = modinfo::parse(&bytes).unwrap();
    assert_eq!(info.mod_id.as_deref(), Some("jsonc-mod"));
    assert_eq!(info.name.as_deref(), Some("Url // not a comment"));
    assert_eq!(info.version, "1.0.1");
}

#[test]
fn decodes_gb18030_modinfo() {
    let bytes = fs::read(fixture("core/modinfo-gb18030.json")).unwrap();
    let info = modinfo::parse(&bytes).unwrap();
    assert_eq!(info.name.as_deref(), Some("中文模组"));
    assert_eq!(info.category.as_deref(), Some("内容"));
}

#[test]
fn rejects_broken_json() {
    assert!(modinfo::parse(b"{ not json").is_err());
}

#[test]
fn reads_option_definitions() {
    let info = modinfo::parse(
        br##"{"ModID":"opt","Options":{
        "Speed":{"type":"slider","default":2,"label":{"English":"Speed"},"min":1,"max":5,"step":1,
                 "group":"Economy","description":{"English":"How fast"}},
        "Enabled":{"default":true},
        "Mode":{"type":"enum","default":"fast","values":["fast","slow"]},
        "Tint":{"type":"colour","default":"#ffffff"}}}"##,
    )
    .unwrap();
    let find = |key: &str| {
        info.options
            .iter()
            .find(|option| option.key == key)
            .unwrap()
    };
    let speed = find("Speed");
    assert_eq!(speed.option_type, ModOptionType::Slider);
    assert_eq!(speed.value, OptionValue::Number(2.0));
    assert_eq!(speed.default_value, OptionValue::Number(2.0));
    assert_eq!(
        (speed.min, speed.max, speed.step),
        (Some(1.0), Some(5.0), Some(1.0))
    );
    assert_eq!(speed.group.as_deref(), Some("Economy"));
    assert_eq!(speed.hint.as_deref(), Some("How fast"));
    let enabled = find("Enabled");
    assert_eq!(enabled.option_type, ModOptionType::Toggle);
    assert_eq!(enabled.label, "Enabled");
    assert_eq!(find("Mode").values, Some(ids(&["fast", "slow"])));
    assert_eq!(find("Tint").option_type, ModOptionType::Color);
}

#[test]
fn empty_mods_folder_yields_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let result = scan::scan(GameId::Anno117, &[documents(tmp.path())]).unwrap();
    assert!(result.mods.is_empty());
    assert!(result.skipped.is_empty());
}

#[test]
fn missing_mods_folder_is_not_an_error() {
    let tmp = tempfile::tempdir().unwrap();
    let result = scan::scan(GameId::Anno1800, &[documents(&tmp.path().join("nope"))]).unwrap();
    assert!(result.mods.is_empty());
}

#[test]
fn folder_without_modinfo_is_skipped_and_reported() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "Good", &info117("good", ""));
    fs::create_dir(tmp.path().join("Broken")).unwrap();
    let result = scan::scan(GameId::Anno117, &[documents(tmp.path())]).unwrap();
    assert_eq!(result.mods.len(), 1);
    assert_eq!(result.skipped.len(), 1);
    assert!(result.skipped[0].path.ends_with("Broken"));
    assert_eq!(result.skipped[0].reason, SkipReason::MissingModinfo);
}

#[test]
fn invalid_modinfo_is_skipped_with_a_reason() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "Bad", "{oops");
    let result = scan::scan(GameId::Anno117, &[documents(tmp.path())]).unwrap();
    assert!(result.mods.is_empty());
    assert!(matches!(result.skipped[0].reason, SkipReason::Invalid(_)));
}

#[test]
fn sub_mods_nest_under_their_parent() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "[Content] Parent", &info117("parent", ""));
    write_mod(
        tmp.path(),
        "[Content] Parent/[Raw] Child [SubMod]",
        &info117("child", ""),
    );
    fs::create_dir_all(tmp.path().join("[Content] Parent/data")).unwrap();
    fs::write(tmp.path().join("[Content] Parent/data/assets.xml"), "<x/>").unwrap();
    let result = scan::scan(GameId::Anno117, &[documents(tmp.path())]).unwrap();
    let find = |id: &str| result.mods.iter().find(|m| m.id == id).unwrap();
    assert_eq!(find("child").parent_id.as_deref(), Some("parent"));
    assert_eq!(
        find("child").folder,
        "[Content] Parent/[Raw] Child [SubMod]"
    );
    assert_eq!(find("parent").sub_mod_ids, ids(&["child"]));
    assert!(find("parent").size_bytes > find("child").size_bytes);
    assert_eq!(result.mods.len(), 2);
}

#[test]
fn dash_and_dot_folders_are_ignored_for_117() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "-Off", &info117("off", ""));
    write_mod(tmp.path(), ".hidden", &info117("hidden", ""));
    let result = scan::scan(GameId::Anno117, &[documents(tmp.path())]).unwrap();
    assert!(result.mods.is_empty());
    assert!(result.skipped.is_empty());
}

#[test]
fn dash_folders_stay_visible_for_1800() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "-Off", &info1800("off"));
    let result = scan::scan(GameId::Anno1800, &[documents(tmp.path())]).unwrap();
    assert_eq!(result.mods[0].folder, "-Off");
}

#[test]
fn mod_without_modid_falls_back_to_the_folder_name() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "-My Cool_Mod", r#"{"ModName":"Cool"}"#);
    let result = scan::scan(GameId::Anno1800, &[documents(tmp.path())]).unwrap();
    assert_eq!(result.mods[0].id, "my-cool-mod");
    assert_eq!(result.mods[0].name, "Cool");
}

#[test]
fn scans_every_location_and_keeps_duplicates() {
    let tmp = tempfile::tempdir().unwrap();
    let folders = [
        folder_at(&tmp.path().join("ubi"), ModLocation::Ubi),
        folder_at(&tmp.path().join("game"), ModLocation::Game),
        documents(&tmp.path().join("docs")),
    ];
    for folder in &folders {
        write_mod(&folder.path, "Twin", &info117("twin", ""));
    }
    let result = scan::scan(GameId::Anno117, &folders).unwrap();
    let locations: Vec<_> = result.mods.iter().map(|m| m.location).collect();
    assert_eq!(
        locations,
        [ModLocation::Ubi, ModLocation::Game, ModLocation::Documents]
    );
    assert_eq!(result.mods[0].origin, ModOrigin::Modio);
    assert_eq!(result.mods[2].origin, ModOrigin::Local);
    assert_eq!(
        scan::mod_dir(&folders, &result.mods[1]),
        Some(tmp.path().join("game").join("Twin"))
    );
}
