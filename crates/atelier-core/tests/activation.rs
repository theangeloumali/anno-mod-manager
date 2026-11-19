mod common;

use std::fs;
use std::path::Path;

use atelier_core::activation::{self, Snapshots};
use atelier_core::{ActivationScheme, Error, GameId, Mod, ModLocation, scan};
use common::*;

fn scan_mods(game: GameId, dir: &Path) -> Vec<Mod> {
    let mut mods = scan::scan(game, &[documents(dir)]).unwrap().mods;
    let state = activation::read(game, dir).unwrap();
    activation::apply(&state, &mut mods);
    mods
}

fn find<'a>(mods: &'a [Mod], id: &str) -> &'a Mod {
    mods.iter().find(|m| m.id == id).unwrap()
}

fn write(dir: &Path, relative: &str, text: &str) {
    let path = dir.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn missing_profile_file_enables_everything() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "profile.txt",
        "## Active profile\nActiveProfile default\n",
    );
    assert_eq!(
        activation::detect_scheme(GameId::Anno117, tmp.path()),
        ActivationScheme::Profile
    );
    let state = activation::read(GameId::Anno117, tmp.path()).unwrap();
    assert!(state.is_active("anything", "Anything"));
    assert_eq!(state.active_profile(), Some("default"));
}

#[test]
fn profile_txt_pointing_at_a_missing_profile_enables_everything() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "profile.txt", "ActiveProfile ghost\n");
    let state = activation::read(GameId::Anno117, tmp.path()).unwrap();
    assert!(state.is_active("anything", "Anything"));
    assert_eq!(state.active_profile(), Some("ghost"));
}

#[test]
fn profile_file_rules_apply() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "profile.txt", "ActiveProfile mine\n");
    write(
        tmp.path(),
        "profiles/mine.txt",
        "## comment\nEnableNewMods\nDisplayName Mine\nkeep-on\n# turned-off\nstale-mod # not installed\n",
    );
    let state = activation::read(GameId::Anno117, tmp.path()).unwrap();
    assert!(state.is_active("keep-on", "x"));
    assert!(!state.is_active("turned-off", "x"));
    assert!(state.is_active("stale-mod", "x"));
    assert!(state.is_active("brand-new", "x"));

    write(
        tmp.path(),
        "profiles/mine.txt",
        "# EnableNewMods\nkeep-on\n",
    );
    let state = activation::read(GameId::Anno117, tmp.path()).unwrap();
    assert!(!state.is_active("brand-new", "x"));
}

#[test]
fn legacy_active_profile_txt_is_read() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "active-profile.txt",
        "## Legacy\n# EnableNewMods\nalpha\n# beta\n## gamma is a comment\n",
    );
    assert_eq!(
        activation::detect_scheme(GameId::Anno117, tmp.path()),
        ActivationScheme::Legacy
    );
    let state = activation::read(GameId::Anno117, tmp.path()).unwrap();
    assert!(state.is_active("alpha", "x"));
    assert!(!state.is_active("beta", "x"));
    assert!(!state.is_active("gamma", "x"));
}

#[test]
fn toggling_writes_the_active_profile_and_keeps_comments() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "profile.txt",
        "## keep me\nActiveProfile default\n",
    );
    write_mod(tmp.path(), "A", &info117("a", ""));
    let mods = scan_mods(GameId::Anno117, tmp.path());
    let snaps = snapshots(tmp.path());

    activation::set_active(
        GameId::Anno117,
        &documents(tmp.path()),
        find(&mods, "a"),
        false,
        &snaps,
    )
    .unwrap();
    assert!(!find(&scan_mods(GameId::Anno117, tmp.path()), "a").active);
    assert!(tmp.path().join("profiles/default.txt").exists());
    let profile_txt = fs::read_to_string(tmp.path().join("profile.txt")).unwrap();
    assert!(profile_txt.contains("## keep me"));

    activation::set_active(
        GameId::Anno117,
        &documents(tmp.path()),
        find(&mods, "a"),
        true,
        &snaps,
    )
    .unwrap();
    assert!(find(&scan_mods(GameId::Anno117, tmp.path()), "a").active);
    assert!(fs::read_dir(tmp.path().join("snapshots")).unwrap().count() >= 1);
}

#[test]
fn legacy_toggle_preserves_comments() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "active-profile.txt",
        "## Legacy\nEnableNewMods\n",
    );
    write_mod(tmp.path(), "A", &info117("a", ""));
    let mods = scan_mods(GameId::Anno117, tmp.path());
    activation::set_active(
        GameId::Anno117,
        &documents(tmp.path()),
        find(&mods, "a"),
        false,
        &snapshots(tmp.path()),
    )
    .unwrap();
    let text = fs::read_to_string(tmp.path().join("active-profile.txt")).unwrap();
    assert!(text.contains("## Legacy"));
    assert!(text.contains("# a"));
}

#[test]
fn snapshots_keep_only_the_newest() {
    let tmp = tempfile::tempdir().unwrap();
    let snaps = Snapshots::new(tmp.path().join("snaps"), 2);
    let file = tmp.path().join("profile.txt");
    assert_eq!(snaps.take(&file).unwrap(), None);
    for round in 0..3 {
        fs::write(&file, format!("round {round}")).unwrap();
        snaps.take(&file).unwrap();
    }
    let mut kept: Vec<_> = fs::read_dir(tmp.path().join("snaps"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    kept.sort();
    assert_eq!(kept.len(), 2);
    assert_eq!(fs::read_to_string(&kept[1]).unwrap(), "round 2");
}

#[test]
fn activation_json_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    write(
        tmp.path(),
        "activation.json",
        r#"{"disabledIds":["other"],"extra":1}"#,
    );
    write_mod(tmp.path(), "A", &info1800("a"));
    let mods = scan_mods(GameId::Anno1800, tmp.path());
    assert!(find(&mods, "a").active);
    let snaps = snapshots(tmp.path());

    activation::set_active(
        GameId::Anno1800,
        &documents(tmp.path()),
        find(&mods, "a"),
        false,
        &snaps,
    )
    .unwrap();
    let state = activation::read(GameId::Anno1800, tmp.path()).unwrap();
    assert!(!state.is_active("a", "A"));
    assert!(!state.is_active("other", "Other"));
    let json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(tmp.path().join("activation.json")).unwrap())
            .unwrap();
    assert_eq!(json["extra"], 1);

    activation::set_active(
        GameId::Anno1800,
        &documents(tmp.path()),
        find(&mods, "a"),
        true,
        &snaps,
    )
    .unwrap();
    assert!(
        activation::read(GameId::Anno1800, tmp.path())
            .unwrap()
            .is_active("a", "A")
    );
}

#[test]
fn missing_1800_mods_folder_is_created_on_first_write() {
    let tmp = tempfile::tempdir().unwrap();
    let mods_dir = tmp.path().join("Anno 1800/mods");
    let mut target = test_mod("a", "Cat", "A");
    target.game = GameId::Anno1800;
    activation::set_active(
        GameId::Anno1800,
        &documents(&mods_dir),
        &target,
        false,
        &snapshots(tmp.path()),
    )
    .unwrap();
    assert!(mods_dir.join("activation.json").exists());
}

#[test]
fn dash_prefixed_1800_folder_is_inactive_and_enabling_renames_it() {
    let tmp = tempfile::tempdir().unwrap();
    write_mod(tmp.path(), "-Off", &info1800("off"));
    let mods = scan_mods(GameId::Anno1800, tmp.path());
    assert!(!find(&mods, "off").active);
    activation::set_active(
        GameId::Anno1800,
        &documents(tmp.path()),
        find(&mods, "off"),
        true,
        &snapshots(tmp.path()),
    )
    .unwrap();
    assert!(tmp.path().join("Off/modinfo.json").exists());
    assert!(find(&scan_mods(GameId::Anno1800, tmp.path()), "off").active);
}

#[test]
fn read_only_locations_refuse_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let target = test_mod("a", "Cat", "A");
    for location in [ModLocation::Game, ModLocation::Ubi] {
        let result = activation::set_active(
            GameId::Anno117,
            &folder_at(tmp.path(), location),
            &target,
            false,
            &snapshots(tmp.path()),
        );
        assert!(matches!(result, Err(Error::ReadOnly(_))));
    }
    assert!(!tmp.path().join("profile.txt").exists());
}

#[test]
fn sub_mods_follow_their_parent() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "profile.txt", "ActiveProfile mine\n");
    write(tmp.path(), "profiles/mine.txt", "EnableNewMods\n# parent\n");
    write_mod(tmp.path(), "P", &info117("parent", ""));
    write_mod(tmp.path(), "P/C [SubMod]", &info117("child", ""));
    let mods = scan_mods(GameId::Anno117, tmp.path());
    assert!(!find(&mods, "child").active);
    let result = activation::set_active(
        GameId::Anno117,
        &documents(tmp.path()),
        find(&mods, "child"),
        true,
        &snapshots(tmp.path()),
    );
    assert!(matches!(result, Err(Error::SubMod(_))));
}

#[test]
fn profiles_can_be_listed_switched_saved_exported_and_deleted() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = documents(tmp.path());
    let snaps = snapshots(tmp.path());
    write(tmp.path(), "profile.txt", "ActiveProfile default\n");
    write_mod(tmp.path(), "A", &info117("a", ""));
    write_mod(tmp.path(), "B", &info117("b", ""));
    let mut mods = scan_mods(GameId::Anno117, tmp.path());
    mods.iter_mut().find(|m| m.id == "b").unwrap().active = false;

    activation::save_profile(&docs, "Raid", &mods, &snaps).unwrap();
    let names: Vec<_> = activation::list_profiles(tmp.path())
        .unwrap()
        .into_iter()
        .map(|p| (p.name, p.built_in, p.active))
        .collect();
    assert_eq!(
        names,
        [
            ("Default".to_string(), true, true),
            ("Vanilla".to_string(), true, false),
            ("Raid".to_string(), false, false)
        ]
    );

    activation::switch_profile(&docs, "Raid", &snaps).unwrap();
    let switched = scan_mods(GameId::Anno117, tmp.path());
    assert!(find(&switched, "a").active);
    assert!(!find(&switched, "b").active);

    let export: serde_json::Value =
        serde_json::from_str(&activation::export_profile(tmp.path(), "Raid").unwrap()).unwrap();
    assert_eq!(export["name"], "Raid");
    assert!(
        export["mods"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({"id":"b","enabled":false}))
    );

    assert!(matches!(
        activation::delete_profile(&docs, "Vanilla", &snaps),
        Err(Error::BuiltInProfile(_))
    ));
    assert!(matches!(
        activation::delete_profile(&docs, "Raid", &snaps),
        Err(Error::ProfileActive(_))
    ));
    assert!(matches!(
        activation::switch_profile(&docs, "ghost", &snaps),
        Err(Error::ProfileNotFound(_))
    ));
    assert!(matches!(
        activation::save_profile(&docs, "../evil", &mods, &snaps),
        Err(Error::InvalidProfileName(_))
    ));

    activation::switch_profile(&docs, "Vanilla", &snaps).unwrap();
    assert!(!find(&scan_mods(GameId::Anno117, tmp.path()), "a").active);
    activation::delete_profile(&docs, "Raid", &snaps).unwrap();
    assert_eq!(activation::list_profiles(tmp.path()).unwrap().len(), 2);
}

#[test]
fn set_off_in_profile_turns_only_the_named_mods_off() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = documents(tmp.path());
    let snaps = snapshots(tmp.path());
    write(tmp.path(), "profile.txt", "ActiveProfile default\n");
    write_mod(tmp.path(), "A", &info117("a", ""));
    write_mod(tmp.path(), "B", &info117("b", ""));
    let mods = scan_mods(GameId::Anno117, tmp.path());
    activation::save_profile(&docs, "Raid", &mods, &snaps).unwrap();

    // Default has no file yet: the first write creates it.
    activation::set_off_in_profile(&docs, "Default", &["b".to_string()], &snaps).unwrap();
    assert!(tmp.path().join("profiles/default.txt").exists());
    let default = scan_mods(GameId::Anno117, tmp.path());
    assert!(find(&default, "a").active);
    assert!(!find(&default, "b").active);

    activation::set_off_in_profile(&docs, "Raid", &["a".to_string()], &snaps).unwrap();
    activation::switch_profile(&docs, "Raid", &snaps).unwrap();
    let raid = scan_mods(GameId::Anno117, tmp.path());
    assert!(!find(&raid, "a").active);
    assert!(find(&raid, "b").active);

    assert!(matches!(
        activation::set_off_in_profile(&docs, "ghost", &["a".to_string()], &snaps),
        Err(Error::ProfileNotFound(_))
    ));
}

#[test]
fn a_profile_name_that_escapes_the_profiles_folder_falls_back_to_default() {
    let tmp = tempfile::tempdir().unwrap();
    let mods_dir = tmp.path().join("mods");
    write(&mods_dir, "profile.txt", "ActiveProfile ../../escaped\n");
    write_mod(&mods_dir, "A", &info117("a", ""));
    let mods = scan_mods(GameId::Anno117, &mods_dir);

    activation::set_active(
        GameId::Anno117,
        &documents(&mods_dir),
        find(&mods, "a"),
        false,
        &snapshots(tmp.path()),
    )
    .unwrap();

    assert!(!tmp.path().join("escaped.txt").exists());
    assert!(mods_dir.join("profiles/default.txt").exists());
}

#[test]
fn profile_names_match_without_case_beyond_ascii_as_on_windows() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = documents(tmp.path());
    let snaps = snapshots(tmp.path());
    write(tmp.path(), "profile.txt", "ActiveProfile default\n");
    write_mod(tmp.path(), "A", &info117("a", ""));
    let mods = scan_mods(GameId::Anno117, tmp.path());

    activation::save_profile(&docs, "Été", &mods, &snaps).unwrap();
    activation::save_profile(&docs, "été", &mods, &snaps).unwrap();

    let custom: Vec<_> = activation::list_profiles(tmp.path())
        .unwrap()
        .into_iter()
        .filter(|profile| !profile.built_in)
        .collect();
    assert_eq!(custom.len(), 1, "NTFS would see one file");
}

#[test]
fn legacy_scheme_has_no_profiles() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "active-profile.txt", "EnableNewMods\n");
    assert!(matches!(
        activation::list_profiles(tmp.path()),
        Err(Error::NoProfiles(ActivationScheme::Legacy))
    ));
}
