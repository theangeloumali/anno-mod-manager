mod common;

use std::fs;
use std::path::PathBuf;

use atelier_app::ErrorCode;
use atelier_core::{GameId, LoadReason, Profile};
use common::{Harness, Options};

const HUMANKIND: GameId = GameId::Humankind;
const BASE: &str = "project mercury";
const GUID: &str = "0123456789abcdef0123456789abcdef";

fn game_dir(harness: &Harness) -> PathBuf {
    harness.root.path().join("My Documents").join("Humankind")
}

/// A module whose bundle can't be read, so its type is unverified (an Extension).
fn add_module(harness: &Harness, name: &str) {
    let long = format!("{name}.{GUID}.1.0");
    let dir = game_dir(harness).join("Community").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(&long), "module").unwrap();
    fs::write(dir.join(format!("{long}.manifest")), "manifest").unwrap();
    // The game needs the bundle to load the module, so the scan only counts it with one.
    let bundle = format!("{}.assetbundle", long.to_lowercase());
    fs::write(dir.join(bundle), "bundle").unwrap();
}

fn registry_text(harness: &Harness) -> String {
    fs::read_to_string(harness.humankind_registry()).unwrap()
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

/// Two installed modules and a registry listing only the base game.
fn setup() -> Harness {
    let harness = Harness::new();
    add_module(&harness, "Alpha");
    add_module(&harness, "Beta");
    harness.write_humankind_registry(BASE);
    harness
}

fn write_history(harness: &Harness, lines: &[&str]) {
    let path = game_dir(harness).join("ModListGuard/history.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, lines.join("\n") + "\n").unwrap();
}

// set_load_order

#[test]
fn setting_the_load_order_writes_short_names_base_first() {
    let harness = setup();
    let written = harness
        .app
        .set_load_order(HUMANKIND, names(&["Beta", "alpha"]))
        .unwrap();
    assert_eq!(written, names(&[BASE, "beta", "alpha"]));
    assert!(registry_text(&harness).contains(&format!(
        "<RuntimeModuleConfiguration>{BASE}/beta/alpha</RuntimeModuleConfiguration>"
    )));
    assert_eq!(
        fs::read_dir(harness.root.path().join("data/snapshots"))
            .unwrap()
            .count(),
        1
    );
}

/// The UI lists only mods, so a missing base game is added; anything else wrong is reported.
#[test]
fn a_given_order_is_checked_as_given_not_silently_fixed() {
    let harness = setup();
    let before = registry_text(&harness);
    for (order, problem) in [
        (names(&["beta", "Beta"]), "listed twice"),
        (names(&["alpha", BASE]), "must come first"),
    ] {
        let error = harness.app.set_load_order(HUMANKIND, order).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput);
        assert!(error.message.contains(problem), "{}", error.message);
    }
    assert_eq!(registry_text(&harness), before);
}

#[test]
fn a_profile_saved_out_of_order_is_reported_when_applied() {
    let harness = setup();
    let file = harness.root.path().join("data/humankind/profiles.json");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    let odd = format!(r#"{{"profiles":[{{"name":"Odd","modules":["alpha","{BASE}"]}}]}}"#);
    fs::write(&file, odd).unwrap();
    let before = registry_text(&harness);
    let error = harness.app.apply_humankind_profile("Odd").unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(
        error.message.contains("must come first"),
        "{}",
        error.message
    );
    assert_eq!(registry_text(&harness), before);
}

#[test]
fn an_order_with_a_module_that_is_not_installed_is_refused() {
    let harness = setup();
    let before = registry_text(&harness);
    let error = harness
        .app
        .set_load_order(HUMANKIND, names(&["alpha", "gone"]))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(error.message.contains("gone"), "{}", error.message);
    assert!(error.message.contains("vanilla"), "{}", error.message);
    assert_eq!(registry_text(&harness), before);
}

#[test]
fn the_load_order_is_not_written_while_the_game_runs() {
    let harness = setup();
    harness.set_running(true);
    let error = harness
        .app
        .set_load_order(HUMANKIND, names(&["alpha"]))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::GameRunning);
}

#[test]
fn anno_has_no_humankind_load_order() {
    let harness = setup();
    let error = harness
        .app
        .set_load_order(GameId::Anno117, names(&["alpha"]))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotSupported);
    assert!(
        harness
            .app
            .launch_blockers(GameId::Anno117)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn no_registry_means_nothing_to_write_to() {
    let harness = Harness::new();
    add_module(&harness, "Alpha");
    let error = harness
        .app
        .set_load_order(HUMANKIND, names(&["alpha"]))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(harness.app.launch_blockers(HUMANKIND).unwrap().is_empty());
}

#[test]
fn module_names_with_xml_characters_are_escaped_in_the_registry() {
    let harness = setup();
    add_module(&harness, "Comp & <Co>");
    harness
        .app
        .set_load_order(HUMANKIND, names(&["comp & <co>"]))
        .unwrap();
    assert!(
        registry_text(&harness).contains("project mercury/comp &amp; &lt;co&gt;<"),
        "{}",
        registry_text(&harness)
    );
    assert!(harness.app.launch_blockers(HUMANKIND).unwrap().is_empty());
}

// launch_blockers

#[test]
fn launch_blockers_name_listed_modules_that_are_not_installed() {
    let harness = setup();
    harness.write_humankind_registry(&format!("{BASE}/alpha/gone/Gone"));
    assert_eq!(
        harness.app.launch_blockers(HUMANKIND).unwrap(),
        names(&["gone"])
    );
}

#[test]
fn launch_is_refused_while_a_listed_module_is_missing() {
    let harness = Harness::with(Options {
        install: Some(HUMANKIND),
        ..Options::default()
    });
    add_module(&harness, "Alpha");
    harness.write_humankind_registry(&format!("{BASE}/alpha/gone"));

    let error = harness.app.launch_game(HUMANKIND).unwrap_err();

    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(error.message.contains("gone"), "{}", error.message);
    assert!(harness.launches.lock().unwrap().is_empty());

    harness.write_humankind_registry(&format!("{BASE}/alpha"));
    harness.app.launch_game(HUMANKIND).unwrap();
    assert_eq!(harness.launches.lock().unwrap().len(), 1);
}

// profiles

fn profile(name: &str, active: bool) -> Profile {
    Profile {
        name: name.into(),
        built_in: false,
        active,
    }
}

#[test]
fn a_saved_profile_holds_the_current_order_and_shows_as_active() {
    let harness = setup();
    harness
        .app
        .set_load_order(HUMANKIND, names(&["beta", "alpha"]))
        .unwrap();
    harness.app.save_humankind_profile("Coop").unwrap();
    assert_eq!(
        harness.app.humankind_profiles().unwrap(),
        vec![profile("Coop", true)]
    );
    harness
        .app
        .set_load_order(HUMANKIND, names(&["alpha"]))
        .unwrap();
    assert_eq!(
        harness.app.humankind_profiles().unwrap(),
        vec![profile("Coop", false)]
    );
}

#[test]
fn applying_a_profile_writes_its_order() {
    let harness = setup();
    harness
        .app
        .set_load_order(HUMANKIND, names(&["beta", "alpha"]))
        .unwrap();
    harness.app.save_humankind_profile("Coop").unwrap();
    harness.app.set_load_order(HUMANKIND, Vec::new()).unwrap();
    harness.app.apply_humankind_profile("coop").unwrap();
    assert!(registry_text(&harness).contains(&format!("{BASE}/beta/alpha<")));
    assert_eq!(
        harness.app.humankind_profiles().unwrap(),
        vec![profile("Coop", true)]
    );
}

#[test]
fn applying_a_profile_whose_mods_are_gone_is_refused() {
    let harness = setup();
    harness
        .app
        .set_load_order(HUMANKIND, names(&["alpha", "beta"]))
        .unwrap();
    harness.app.save_humankind_profile("Coop").unwrap();
    harness.app.set_load_order(HUMANKIND, Vec::new()).unwrap();
    fs::remove_dir_all(game_dir(&harness).join("Community/Beta")).unwrap();
    let before = registry_text(&harness);
    let error = harness.app.apply_humankind_profile("Coop").unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    assert!(error.message.contains("beta"), "{}", error.message);
    assert_eq!(registry_text(&harness), before);
}

#[test]
fn profile_names_are_checked_and_unique_ignoring_case() {
    let harness = setup();
    let error = harness.app.save_humankind_profile("../evil").unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    harness.app.save_humankind_profile("Coop").unwrap();
    harness.app.save_humankind_profile("COOP").unwrap();
    assert_eq!(
        harness.app.humankind_profiles().unwrap(),
        vec![profile("Coop", true)]
    );
}

#[test]
fn deleting_a_profile_removes_it() {
    let harness = setup();
    harness.app.save_humankind_profile("Coop").unwrap();
    harness.app.delete_humankind_profile("coop").unwrap();
    assert!(harness.app.humankind_profiles().unwrap().is_empty());
    let error = harness.app.delete_humankind_profile("Coop").unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
    let error = harness.app.apply_humankind_profile("Coop").unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
}

#[test]
fn profiles_are_kept_in_the_data_folder_not_documents() {
    let harness = setup();
    harness.app.save_humankind_profile("Coop").unwrap();
    assert!(
        harness
            .root
            .path()
            .join("data/humankind/profiles.json")
            .is_file()
    );
}

// history

#[test]
fn no_history_file_lists_nothing() {
    let harness = setup();
    assert!(harness.app.mod_list_history().unwrap().is_empty());
}

#[test]
fn an_oversized_history_file_is_refused_with_a_clear_error() {
    let harness = setup();
    let path = game_dir(&harness).join("ModListGuard/history.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, vec![b'\n'; 9 * 1024 * 1024]).unwrap();
    let error = harness.app.mod_list_history().unwrap_err();
    assert_eq!(error.code, ErrorCode::Io);
    assert!(error.message.contains("too large"), "{}", error.message);
}

#[test]
fn history_lists_newest_first() {
    let harness = setup();
    write_history(
        &harness,
        &[
            r#"{"time":"2026-11-20T01:02:03.0000000Z","reason":"older","culprit":null,"old":"project mercury","new":"project mercury"}"#,
            "garbage",
            r#"{"time":"2026-11-21T03:04:05.0000000Z","reason":"reset","culprit":null,"old":"project mercury\/alpha\/beta","new":"project mercury"}"#,
        ],
    );
    let history = harness.app.mod_list_history().unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].reason, "reset");
    assert_eq!(history[0].old, names(&[BASE, "alpha", "beta"]));
}

#[test]
fn restoring_an_entry_saves_its_old_list_as_a_profile() {
    let harness = setup();
    write_history(
        &harness,
        &[
            r#"{"time":"2026-11-21T03:04:05.0000000Z","reason":"reset","culprit":null,"old":"project mercury\/beta\/alpha","new":"project mercury"}"#,
        ],
    );
    let entry = &harness.app.mod_list_history().unwrap()[0];
    let name = harness
        .app
        .restore_history_entry(&entry.time, &entry.old)
        .unwrap();
    assert_eq!(name, "Restored 2026-11-21 03-04-05 UTC");
    assert_eq!(
        harness.app.humankind_profiles().unwrap(),
        vec![profile(&name, false)]
    );
    assert!(registry_text(&harness).contains(&format!(">{BASE}<")));
    harness.app.apply_humankind_profile(&name).unwrap();
    assert!(registry_text(&harness).contains(&format!("{BASE}/beta/alpha<")));
}

#[test]
fn restoring_an_entry_that_does_not_exist_is_not_found() {
    let harness = setup();
    let error = harness
        .app
        .restore_history_entry("2026-11-21T03:04:05.0000000Z", &names(&[BASE]))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);
}

#[test]
fn restoring_finds_its_entry_after_the_game_logs_a_newer_one() {
    let harness = setup();
    let older = r#"{"time":"2026-11-21T03:04:05.0000000Z","reason":"reset","culprit":null,"old":"project mercury\/alpha","new":"project mercury"}"#;
    write_history(&harness, &[older]);
    let target = harness.app.mod_list_history().unwrap()[0].clone();
    let newer = r#"{"time":"2026-11-22T09:00:00.0000000Z","reason":"reset","culprit":null,"old":"project mercury\/beta","new":"project mercury"}"#;
    write_history(&harness, &[older, newer]);

    let name = harness
        .app
        .restore_history_entry(&target.time, &target.old)
        .unwrap();

    harness.app.apply_humankind_profile(&name).unwrap();
    assert!(registry_text(&harness).contains(&format!("{BASE}/alpha<")));
}

// the shared profile and load-order commands

#[test]
fn the_shared_profile_commands_use_humankind_profiles() {
    let harness = setup();
    harness.write_humankind_registry(&format!("{BASE}/alpha"));
    harness.app.save_profile(HUMANKIND, "Mine").unwrap();
    harness.write_humankind_registry(BASE);

    let listed = harness.app.list_profiles(HUMANKIND).unwrap();
    assert_eq!(listed, vec![profile("Mine", false)]);
    harness.app.switch_profile(HUMANKIND, "Mine").unwrap();
    assert!(registry_text(&harness).contains(&format!("{BASE}/alpha")));
    harness.app.delete_profile(HUMANKIND, "Mine").unwrap();
    assert!(harness.app.list_profiles(HUMANKIND).unwrap().is_empty());
    assert!(
        !game_dir(&harness)
            .join("Community")
            .join("profiles")
            .exists()
    );
}

#[test]
fn the_load_order_is_the_registry_order_set_by_the_player() {
    let harness = setup();
    harness.write_humankind_registry(&format!("{BASE}/beta/alpha"));

    let order = harness.app.load_order(HUMANKIND).unwrap();

    let ids: Vec<&str> = order.entries.iter().map(|e| e.mod_id.as_str()).collect();
    assert_eq!(ids, ["beta", "alpha"]);
    assert!(order.entries.iter().all(|e| e.reason == LoadReason::Player));
    assert_eq!(order.entries[0].position, 1);
}

#[test]
fn exporting_a_humankind_profile_lists_its_modules_in_order() {
    let harness = setup();
    harness.write_humankind_registry(&format!("{BASE}/beta/alpha"));
    harness.app.save_profile(HUMANKIND, "Mine").unwrap();

    let text = harness.app.export_profile(HUMANKIND, "mine").unwrap();

    assert_eq!(text, "project mercury\nbeta\nalpha\n");
    assert!(harness.app.export_profile(HUMANKIND, "Nope").is_err());
}

#[test]
fn restoring_two_entries_from_the_same_second_keeps_both() {
    let harness = setup();
    write_history(
        &harness,
        &[
            r#"{"time":"2026-11-21T03:04:05.1000000Z","reason":"reset","culprit":null,"old":"project mercury\/alpha","new":"project mercury"}"#,
            r#"{"time":"2026-11-21T03:04:05.2000000Z","reason":"reset","culprit":null,"old":"project mercury\/beta","new":"project mercury"}"#,
        ],
    );

    let history = harness.app.mod_list_history().unwrap();
    let first = harness
        .app
        .restore_history_entry(&history[0].time, &history[0].old)
        .unwrap();
    let second = harness
        .app
        .restore_history_entry(&history[1].time, &history[1].old)
        .unwrap();

    assert_eq!(first, "Restored 2026-11-21 03-04-05 UTC");
    assert_eq!(second, "Restored 2026-11-21 03-04-05 UTC 2");
    assert_eq!(harness.app.humankind_profiles().unwrap().len(), 2);
}

#[test]
fn humankind_has_no_anno_mod_loader_log() {
    let harness = setup();
    let log = game_dir(&harness).join("log").join("mod-loader.log");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    fs::write(&log, "[2026-11-19 13:54:37.100] [info] Search mods...\n").unwrap();

    assert!(harness.app.loader_log(HUMANKIND).unwrap().is_empty());
}
