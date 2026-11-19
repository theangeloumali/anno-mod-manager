mod common;

use std::fs;

use atelier_core::{
    Error, IssueKind, IssueLevel, LoadReason, ModLocation, OptionValue, Phase, issues, load_order,
    modinfo, options,
};
use common::*;

#[test]
fn load_order_runs_sub_mods_first_then_alphabetical_then_late() {
    let mut needs = test_mod("needs-zed", "Content", "Needs");
    needs.dependencies.load_after = ids(&["zed", "not-installed"]);
    let mut late = test_mod("late-one", "Gameplay", "Aardvark");
    late.dependencies.load_after = ids(&["*"]);
    let mut child = test_mod("child", "Raw", "Child");
    child.parent_id = Some("zed".into());
    let mut off = test_mod("off", "A", "A");
    off.active = false;
    let mods = vec![
        test_mod("zed", "Gameplay", "Zed"),
        test_mod("alpha", "Gameplay", "Alpha"),
        late,
        needs,
        child,
        off,
    ];

    let order = load_order::compute(&mods);
    let got: Vec<_> = order.entries.iter().map(|e| e.mod_id.as_str()).collect();
    assert_eq!(got, ["child", "zed", "needs-zed", "alpha", "late-one"]);
    assert!(order.cycles.is_empty());
    let needs_entry = &order.entries[2];
    assert_eq!(needs_entry.position, 3);
    assert_eq!(needs_entry.reason, LoadReason::LoadAfter);
    assert_eq!(needs_entry.after, ids(&["zed"]));
    assert_eq!(order.entries[1].reason, LoadReason::Alphabetical);
    assert_eq!(order.entries[4].phase, Phase::Late);
    assert_eq!(order.entries[4].reason, LoadReason::Late);
}

#[test]
fn load_after_cycle_is_reported_not_hidden() {
    let mut first = test_mod("a", "Cat", "A");
    first.dependencies.load_after = ids(&["b"]);
    let mut second = test_mod("b", "Cat", "B");
    second.dependencies.load_after = ids(&["a"]);
    let order = load_order::compute(&[first, second]);
    assert_eq!(order.entries.len(), 2);
    assert_eq!(order.cycles, vec![ids(&["a", "b"])]);
}

#[test]
fn issues_cover_dependencies_conflicts_duplicates_and_cycles() {
    let mut app = test_mod("app", "Cat", "App");
    app.dependencies.require = ids(&["lib", "sleepy"]);
    let mut sleepy = test_mod("sleepy", "Cat", "Sleepy");
    sleepy.active = false;
    let mut x = test_mod("x", "Cat", "X");
    x.dependencies.incompatible = ids(&["y"]);
    let mut y = test_mod("y", "Cat", "Y");
    y.dependencies.incompatible = ids(&["x"]);
    let mut newer = test_mod("newer", "Cat", "Newer");
    newer.dependencies.deprecate = ids(&["older"]);
    let mut idle = test_mod("idle", "Cat", "Idle");
    idle.active = false;
    idle.dependencies.require = ids(&["nothing"]);
    let mut twin_game = test_mod("twin", "Cat", "Twin");
    twin_game.location = ModLocation::Game;
    let mut loop_a = test_mod("loop-a", "Cat", "LA");
    loop_a.dependencies.load_after = ids(&["loop-b"]);
    let mut loop_b = test_mod("loop-b", "Cat", "LB");
    loop_b.dependencies.load_after = ids(&["loop-a"]);
    let mods = vec![
        app,
        sleepy,
        x,
        y,
        newer,
        test_mod("older", "Cat", "Older"),
        idle,
        test_mod("twin", "Cat", "Twin"),
        twin_game,
        loop_a,
        loop_b,
    ];

    let found = issues::check(&mods, &load_order::compute(&mods));
    let summary: Vec<_> = found
        .iter()
        .map(|i| {
            (
                i.kind,
                i.mod_id.as_str(),
                i.related_mod_id.as_deref(),
                i.level,
            )
        })
        .collect();
    assert_eq!(
        summary,
        [
            (
                IssueKind::MissingDependency,
                "app",
                Some("lib"),
                IssueLevel::Error
            ),
            (
                IssueKind::InactiveDependency,
                "app",
                Some("sleepy"),
                IssueLevel::Warning
            ),
            (IssueKind::Incompatible, "x", Some("y"), IssueLevel::Error),
            (
                IssueKind::Deprecated,
                "older",
                Some("newer"),
                IssueLevel::Warning
            ),
            (IssueKind::DuplicateId, "twin", None, IssueLevel::Warning),
            (
                IssueKind::LoadOrderCycle,
                "loop-a",
                Some("loop-b"),
                IssueLevel::Warning
            ),
        ]
    );
}

const OPTION_MODINFO: &[u8] = br#"{"ModID":"my-mod","Options":{
    "Speed":{"type":"slider","default":2,"min":1,"max":5},
    "Enabled":{"default":true}}}"#;

#[test]
fn options_overlay_saved_values_and_save_keeps_other_mods() {
    let tmp = tempfile::tempdir().unwrap();
    let defaults = modinfo::parse(OPTION_MODINFO).unwrap().options;
    fs::write(
        tmp.path().join("active-options.jsonc"),
        "// user tweaks\n{ \"my-mod\": { \"Speed\": 4, }, \"other\": { \"X\": true } }",
    )
    .unwrap();

    let loaded = options::load(tmp.path(), "my-mod", &defaults).unwrap();
    let value = |opts: &[atelier_core::ModOption], key: &str| {
        opts.iter().find(|o| o.key == key).unwrap().value.clone()
    };
    assert_eq!(value(&loaded, "Speed"), OptionValue::Number(4.0));
    assert_eq!(value(&loaded, "Enabled"), OptionValue::Bool(true));

    let changes = [("Enabled".to_string(), OptionValue::Bool(false))];
    options::save(
        &documents(tmp.path()),
        "my-mod",
        &changes,
        &snapshots(tmp.path()),
    )
    .unwrap();
    let reloaded = options::load(tmp.path(), "my-mod", &defaults).unwrap();
    assert_eq!(value(&reloaded, "Enabled"), OptionValue::Bool(false));
    assert_eq!(value(&reloaded, "Speed"), OptionValue::Number(4.0));
    let saved = fs::read_to_string(tmp.path().join("active-options.jsonc")).unwrap();
    assert!(saved.contains("\"other\""));
}

#[test]
fn options_default_when_nothing_is_saved_and_refuse_read_only_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let defaults = modinfo::parse(OPTION_MODINFO).unwrap().options;
    assert_eq!(
        options::load(tmp.path(), "my-mod", &defaults).unwrap(),
        defaults
    );
    let result = options::save(
        &folder_at(tmp.path(), ModLocation::Ubi),
        "my-mod",
        &[],
        &snapshots(tmp.path()),
    );
    assert!(matches!(result, Err(Error::ReadOnly(_))));
}
