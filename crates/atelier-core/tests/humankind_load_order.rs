mod common;

use std::fs;
use std::path::Path;

use atelier_core::humankind::history::{self, HistoryEntry};
use atelier_core::humankind::load_order::{self, OrderError};
use atelier_core::humankind::profiles::{self, HumankindProfile};
use atelier_core::humankind::registry::BASE_MODULE;
use atelier_core::{Error, GameId, Mod};
use common::{ids, snapshots, test_mod};

fn module(id: &str, category: &str) -> Mod {
    Mod {
        game: GameId::Humankind,
        ..test_mod(id, category, id)
    }
}

/// standalone `solo`, conversions `conv` and `conv2`, extensions `ext` and `ext2`.
fn installed() -> Vec<Mod> {
    vec![
        module("solo", "Standalone"),
        module("solo2", "Standalone"),
        module("conv", "Conversion"),
        module("conv2", "Conversion"),
        module("ext", "Extension"),
        module("ext2", "Extension"),
        module("unverified", ""),
    ]
}

fn order(values: &[&str]) -> Vec<String> {
    ids(values)
}

fn validate(values: &[&str]) -> Result<(), OrderError> {
    load_order::validate(&order(values), &load_order::module_types(&installed()))
}

// normalize

#[test]
fn normalize_puts_the_base_first_and_drops_duplicates() {
    let normalized = load_order::normalize(&order(&["ext", BASE_MODULE, "Ext", "ext2", "ext"]));
    assert_eq!(normalized, order(&[BASE_MODULE, "ext", "ext2"]));
}

#[test]
fn normalize_adds_a_missing_base_and_handles_an_empty_order() {
    assert_eq!(load_order::normalize(&[]), order(&[BASE_MODULE]));
    assert_eq!(
        load_order::normalize(&order(&["ext"])),
        order(&[BASE_MODULE, "ext"])
    );
}

#[test]
fn normalize_maps_long_names_and_drops_blanks() {
    let normalized = load_order::normalize(&order(&[
        "Comp Super Pack & VIP.a4062ec68ec9491cb1db0e70fb8e3f38.0.17",
        "  ",
        "",
    ]));
    assert_eq!(normalized, order(&[BASE_MODULE, "comp super pack & vip"]));
}

// validate

#[test]
fn valid_orders_pass() {
    assert_eq!(validate(&[BASE_MODULE]), Ok(()));
    assert_eq!(
        validate(&[BASE_MODULE, "solo", "conv", "ext", "ext2"]),
        Ok(())
    );
    assert_eq!(validate(&[BASE_MODULE, "conv", "ext2", "ext"]), Ok(()));
}

#[test]
fn an_order_without_the_base_first_is_refused() {
    assert_eq!(validate(&[]), Err(OrderError::BaseNotFirst));
    assert_eq!(validate(&["ext"]), Err(OrderError::BaseNotFirst));
    assert_eq!(
        validate(&["ext", BASE_MODULE]),
        Err(OrderError::BaseNotFirst)
    );
}

#[test]
fn duplicates_are_refused() {
    assert_eq!(
        validate(&[BASE_MODULE, "ext", "EXT"]),
        Err(OrderError::Duplicate("ext".into()))
    );
}

#[test]
fn two_standalones_or_two_conversions_are_refused() {
    assert_eq!(
        validate(&[BASE_MODULE, "solo", "solo2"]),
        Err(OrderError::TwoStandalones("solo".into(), "solo2".into()))
    );
    assert_eq!(
        validate(&[BASE_MODULE, "conv", "conv2"]),
        Err(OrderError::TwoConversions("conv".into(), "conv2".into()))
    );
}

#[test]
fn a_standalone_must_follow_the_base() {
    assert_eq!(
        validate(&[BASE_MODULE, "ext", "solo"]),
        Err(OrderError::StandaloneMisplaced("solo".into()))
    );
}

#[test]
fn a_conversion_must_precede_every_extension() {
    assert_eq!(
        validate(&[BASE_MODULE, "ext", "conv"]),
        Err(OrderError::ConversionMisplaced("conv".into()))
    );
    assert_eq!(
        validate(&[BASE_MODULE, "conv", "solo"]),
        Err(OrderError::StandaloneMisplaced("solo".into()))
    );
}

#[test]
fn an_unverified_or_unknown_type_counts_as_an_extension() {
    let types = load_order::module_types(&installed());
    assert_eq!(
        load_order::module_type(&types, "unverified"),
        atelier_core::humankind::bundle::ModuleType::Extension
    );
    assert_eq!(
        validate(&[BASE_MODULE, "unverified", "solo"]),
        Err(OrderError::StandaloneMisplaced("solo".into()))
    );
    assert_eq!(
        validate(&[BASE_MODULE, "solo", "unverified", "ext"]),
        Ok(())
    );
    assert_eq!(validate(&[BASE_MODULE, "not installed"]), Ok(()));
}

// enable / disable

fn enable(values: &[&str], module: &str) -> Vec<String> {
    load_order::enable(
        &order(values),
        module,
        &load_order::module_types(&installed()),
    )
}

#[test]
fn enabling_an_extension_appends_it() {
    assert_eq!(
        enable(&[BASE_MODULE, "ext"], "ext2"),
        order(&[BASE_MODULE, "ext", "ext2"])
    );
    assert_eq!(enable(&[], "ext"), order(&[BASE_MODULE, "ext"]));
}

#[test]
fn enabling_a_listed_module_or_the_base_changes_nothing() {
    let current = [BASE_MODULE, "solo", "ext"];
    assert_eq!(enable(&current, "ext"), order(&current));
    assert_eq!(enable(&current, "SOLO"), order(&current));
    assert_eq!(enable(&current, BASE_MODULE), order(&current));
}

#[test]
fn a_second_standalone_replaces_the_first() {
    assert_eq!(
        enable(&[BASE_MODULE, "solo", "conv", "ext"], "solo2"),
        order(&[BASE_MODULE, "solo2", "conv", "ext"])
    );
    assert_eq!(
        enable(&[BASE_MODULE, "conv", "ext"], "solo"),
        order(&[BASE_MODULE, "solo", "conv", "ext"])
    );
}

#[test]
fn a_second_conversion_replaces_the_first() {
    assert_eq!(
        enable(&[BASE_MODULE, "solo", "conv", "ext"], "conv2"),
        order(&[BASE_MODULE, "solo", "conv2", "ext"])
    );
    assert_eq!(
        enable(&[BASE_MODULE, "ext"], "conv"),
        order(&[BASE_MODULE, "conv", "ext"])
    );
}

#[test]
fn disabling_removes_the_module() {
    assert_eq!(
        load_order::disable(&order(&[BASE_MODULE, "solo", "ext"]), "Solo").unwrap(),
        order(&[BASE_MODULE, "ext"])
    );
    assert_eq!(
        load_order::disable(&order(&[BASE_MODULE]), "ext").unwrap(),
        order(&[BASE_MODULE])
    );
}

#[test]
fn the_base_cannot_be_disabled() {
    let error = load_order::disable(&order(&[BASE_MODULE, "ext"]), BASE_MODULE).unwrap_err();
    assert!(matches!(error, Error::BaseModule(name) if name == BASE_MODULE));
}

// missing

#[test]
fn missing_lists_modules_that_are_not_installed_once() {
    let current = order(&[BASE_MODULE, "ext", "gone", "Gone", "also gone"]);
    assert_eq!(
        load_order::missing(&current, &installed()),
        order(&["gone", "also gone"])
    );
    assert!(load_order::missing(&order(&[BASE_MODULE]), &[]).is_empty());
    assert!(load_order::missing(&[], &[]).is_empty());
}

#[test]
fn xml_special_characters_survive_the_order_functions() {
    let mods = vec![
        module("comp super pack & vip", "Extension"),
        module("a<b>", ""),
    ];
    let types = load_order::module_types(&mods);
    let current = load_order::enable(&order(&[BASE_MODULE]), "comp super pack & vip", &types);
    let current = load_order::enable(&current, "a<b>", &types);
    assert_eq!(
        current,
        order(&[BASE_MODULE, "comp super pack & vip", "a<b>"])
    );
    assert!(load_order::missing(&current, &mods).is_empty());
    assert_eq!(load_order::validate(&current, &types), Ok(()));
}

// profiles

fn profiles_file(root: &Path) -> std::path::PathBuf {
    profiles::profiles_path(&root.join("data"))
}

#[test]
fn profiles_live_under_the_humankind_data_folder() {
    assert_eq!(
        profiles::profiles_path(Path::new("data")),
        Path::new("data").join("humankind").join("profiles.json")
    );
}

#[test]
fn no_profiles_file_lists_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(
        profiles::list(&profiles_file(tmp.path()))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn saved_profiles_list_in_save_order_with_normalized_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    let snaps = snapshots(tmp.path());
    profiles::save(&path, "Coop", &order(&["ext", "ext"]), &snaps).unwrap();
    profiles::save(&path, "Solo run", &order(&[BASE_MODULE, "solo"]), &snaps).unwrap();
    assert_eq!(
        profiles::list(&path).unwrap(),
        vec![
            HumankindProfile {
                name: "Coop".into(),
                modules: order(&[BASE_MODULE, "ext"]),
            },
            HumankindProfile {
                name: "Solo run".into(),
                modules: order(&[BASE_MODULE, "solo"]),
            },
        ]
    );
}

#[test]
fn saving_a_name_that_differs_only_by_case_replaces_that_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    let snaps = snapshots(tmp.path());
    profiles::save(&path, "Coop", &order(&["ext"]), &snaps).unwrap();
    profiles::save(&path, "COOP", &order(&["ext2"]), &snaps).unwrap();
    let listed = profiles::list(&path).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "Coop");
    assert_eq!(listed[0].modules, order(&[BASE_MODULE, "ext2"]));
    assert_eq!(profiles::find(&path, "coop").unwrap(), listed[0]);
}

#[test]
fn invalid_profile_names_are_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    let snaps = snapshots(tmp.path());
    let too_long = "x".repeat(65);
    for name in ["", "   ", "../escape", "a/b", "dot.name", too_long.as_str()] {
        let error = profiles::save(&path, name, &[], &snaps).unwrap_err();
        assert!(matches!(error, Error::InvalidProfileName(_)), "{name}");
    }
    assert!(!path.exists());
}

#[test]
fn deleting_removes_only_that_profile() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    let snaps = snapshots(tmp.path());
    profiles::save(&path, "Coop", &[], &snaps).unwrap();
    profiles::save(&path, "Solo", &[], &snaps).unwrap();
    profiles::delete(&path, "coop", &snaps).unwrap();
    let names: Vec<String> = profiles::list(&path)
        .unwrap()
        .into_iter()
        .map(|profile| profile.name)
        .collect();
    assert_eq!(names, order(&["Solo"]));
}

#[test]
fn unknown_profiles_are_not_found() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    let snaps = snapshots(tmp.path());
    assert!(matches!(
        profiles::delete(&path, "Nope", &snaps),
        Err(Error::ProfileNotFound(_))
    ));
    assert!(matches!(
        profiles::find(&path, "Nope"),
        Err(Error::ProfileNotFound(_))
    ));
}

#[test]
fn a_corrupt_profiles_file_is_an_error_not_an_empty_list() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{ not json").unwrap();
    assert!(matches!(profiles::list(&path), Err(Error::Json(_))));
    let snaps = snapshots(tmp.path());
    assert!(profiles::save(&path, "Coop", &[], &snaps).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "{ not json");
}

#[test]
fn rewriting_the_profiles_file_snapshots_the_old_one() {
    let tmp = tempfile::tempdir().unwrap();
    let path = profiles_file(tmp.path());
    let snaps = snapshots(tmp.path());
    profiles::save(&path, "Coop", &[], &snaps).unwrap();
    profiles::save(&path, "Solo", &[], &snaps).unwrap();
    assert_eq!(
        fs::read_dir(tmp.path().join("snapshots")).unwrap().count(),
        1
    );
}

// history

fn history_file(root: &Path) -> std::path::PathBuf {
    history::history_path(&root.join("Humankind"))
}

fn write_history(root: &Path, contents: &str) -> std::path::PathBuf {
    let path = history_file(root);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, contents).unwrap();
    path
}

#[test]
fn history_lives_in_the_mod_list_guard_folder() {
    assert_eq!(
        history::history_path(Path::new("Humankind")),
        Path::new("Humankind")
            .join("ModListGuard")
            .join("history.jsonl")
    );
}

#[test]
fn a_missing_or_empty_history_reads_as_no_entries() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(
        history::read_history(&history_file(tmp.path()))
            .unwrap()
            .is_empty()
    );
    let path = write_history(tmp.path(), "");
    assert!(history::read_history(&path).unwrap().is_empty());
}

#[test]
fn history_reads_newest_first_with_split_lists() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_history(
        tmp.path(),
        concat!(
            r#"{"time":"2026-11-20T01:02:03.0000000Z","reason":"startup","culprit":null,"old":"project mercury\/ext","new":"project mercury"}"#,
            "\n",
            r#"{"time":"2026-11-21T03:04:05.0000000Z","reason":"reset","culprit":"Amplitude.Mods","old":"project mercury/comp super pack & vip//","new":""}"#,
            "\r\n",
        ),
    );
    assert_eq!(
        history::read_history(&path).unwrap(),
        vec![
            HistoryEntry {
                time: "2026-11-21T03:04:05.0000000Z".into(),
                reason: "reset".into(),
                culprit: Some("Amplitude.Mods".into()),
                old: order(&[BASE_MODULE, "comp super pack & vip"]),
                new: Vec::new(),
            },
            HistoryEntry {
                time: "2026-11-20T01:02:03.0000000Z".into(),
                reason: "startup".into(),
                culprit: None,
                old: order(&[BASE_MODULE, "ext"]),
                new: order(&[BASE_MODULE]),
            },
        ]
    );
}

#[test]
fn garbage_history_lines_are_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let mut contents = b"\xEF\xBB\xBF".to_vec();
    contents.extend_from_slice(
        concat!(
            "not json\n",
            "{\"time\":\"t1\"}\n",
            "\n",
            r#"{"time":"t2","reason":"r","old":"project mercury","new":"project mercury"}"#,
            "\n",
            "{\"time\":\"t3\",\"reason\":\"r\",\"old\":\"",
        )
        .as_bytes(),
    );
    contents.extend_from_slice(b"\xFF\xFE\n");
    let path = history_file(tmp.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, contents).unwrap();
    let entries = history::read_history(&path).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].time, "t2");
    assert_eq!(entries[0].culprit, None);
}

#[test]
fn a_bom_on_the_first_valid_line_is_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let mut contents = b"\xEF\xBB\xBF".to_vec();
    contents.extend_from_slice(br#"{"time":"t","reason":"r","culprit":null,"old":"a","new":"b"}"#);
    let path = history_file(tmp.path());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, contents).unwrap();
    assert_eq!(history::read_history(&path).unwrap().len(), 1);
}
