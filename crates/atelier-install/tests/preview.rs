mod common;

use std::fs;

use atelier_core::{GameId, Severity};
use atelier_install::{InstallItemStatus, InstallPreviewItem};
use common::*;

fn only_item(items: &[InstallPreviewItem]) -> &InstallPreviewItem {
    assert_eq!(items.len(), 1, "items: {items:#?}");
    &items[0]
}

fn has_check(item: &InstallPreviewItem, level: Severity, needle: &str) -> bool {
    item.checks
        .iter()
        .any(|check| check.level == level && check.message.contains(needle))
}

#[test]
fn a_zip_with_a_wrapper_folder_yields_one_new_mod_and_writes_nothing_to_mods() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Harbour.zip");
    let info = modinfo("Harbour_Expansion", "1.2.0", Some(8));
    write_zip(
        &archive,
        &[
            Entry::File("Wrapper/Harbour/modinfo.json", info.as_bytes()),
            Entry::File("Wrapper/Harbour/data/base/config/a.xml", b"<a/>"),
        ],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    assert_eq!(preview.source_name, "Harbour.zip");
    let item = only_item(&preview.items);
    assert_eq!(item.mod_id, "harbour-expansion");
    assert_eq!(item.root_path, "Wrapper/Harbour");
    assert_eq!(item.name, "Harbour_Expansion name");
    assert_eq!(item.version, "1.2.0");
    assert_eq!(item.target_game, Some(GameId::Anno117));
    assert_eq!(item.status, InstallItemStatus::New);
    assert!(!sandbox.mods().exists(), "preview must not touch mods");
    assert!(sandbox.staging().join(&preview.id).is_dir());
}

#[test]
fn three_roots_and_a_sub_mod_become_three_items() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Pack.zip");
    let (a, b, c, sub) = (
        modinfo("ModA", "1", Some(8)),
        modinfo("ModB", "1", Some(8)),
        modinfo("ModC", "1", Some(8)),
        modinfo("ModA_Visuals", "1", Some(8)),
    );
    write_zip(
        &archive,
        &[
            Entry::File("Pack/ModA/modinfo.json", a.as_bytes()),
            Entry::File("Pack/ModA/Visuals/modinfo.json", sub.as_bytes()),
            Entry::File("Pack/ModB/modinfo.json", b.as_bytes()),
            Entry::File("Pack/Deep/ModC/modinfo.jsonc", c.as_bytes()),
        ],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    let ids: Vec<&str> = preview.items.iter().map(|i| i.mod_id.as_str()).collect();
    assert_eq!(ids, ["moda", "modb", "modc"]);
    assert_eq!(preview.items[0].sub_mod_ids, ["moda-visuals"]);
    assert!(preview.items[1].sub_mod_ids.is_empty());
}

#[test]
fn a_zip_inside_a_zip_is_unpacked_one_level() {
    let sandbox = Sandbox::new();
    let info = modinfo("Inner", "2.0", Some(8));
    let inner = zip_bytes(&[Entry::File("Inner/modinfo.json", info.as_bytes())]);
    let archive = sandbox.drop_path("Outer.zip");
    write_zip(&archive, &[Entry::File("Inner.zip", &inner)]);

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    assert_eq!(only_item(&preview.items).mod_id, "inner");
}

#[test]
fn macosx_and_ds_store_entries_are_ignored() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Mac.zip");
    let info = modinfo("MacMod", "1", Some(8));
    write_zip(
        &archive,
        &[
            Entry::File("MacMod/modinfo.json", info.as_bytes()),
            Entry::File("MacMod/.DS_Store", b"junk"),
            Entry::File("__MACOSX/MacMod/modinfo.json", info.as_bytes()),
            Entry::File("__MACOSX/MacMod/._modinfo.json", b"junk"),
        ],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    only_item(&preview.items);
    let staged = files_under(&sandbox.staging().join(&preview.id));
    assert!(
        staged
            .iter()
            .all(|f| !f.contains("__MACOSX") && !f.ends_with(".DS_Store")),
        "{staged:?}"
    );
}

#[test]
fn a_mod_for_the_other_game_is_wrong_game_and_names_it() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Old.zip");
    let info = modinfo("OldMod", "1", Some(7));
    write_zip(
        &archive,
        &[Entry::File("OldMod/modinfo.json", info.as_bytes())],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    let item = only_item(&preview.items);
    assert_eq!(item.status, InstallItemStatus::WrongGame);
    assert_eq!(item.target_game, Some(GameId::Anno1800));
    assert!(has_check(item, Severity::Error, "Anno 1800"), "{item:#?}");
}

#[test]
fn without_an_anno_field_the_asset_layout_decides_the_game() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Layout.zip");
    let info = modinfo("Layout", "1", None);
    write_zip(
        &archive,
        &[
            Entry::File("Layout/modinfo.json", info.as_bytes()),
            Entry::File("Layout/data/config/export/main/asset/a.xml", b"<a/>"),
        ],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    let item = only_item(&preview.items);
    assert_eq!(item.target_game, Some(GameId::Anno1800));
    assert_eq!(item.status, InstallItemStatus::WrongGame);
}

#[test]
fn no_modinfo_is_invalid_for_117() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Bare.zip");
    write_zip(
        &archive,
        &[Entry::File("Bare/data/base/config/a.xml", b"<a/>")],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    let item = only_item(&preview.items);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(
        has_check(item, Severity::Error, "modinfo.json"),
        "{item:#?}"
    );
}

#[test]
fn no_modinfo_with_a_data_folder_is_an_inferred_root_for_1800() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Legacy.zip");
    write_zip(
        &archive,
        &[Entry::File(
            "Legacy Mod/data/config/export/main/asset/a.xml",
            b"<a/>",
        )],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno1800, &[archive], &[])
        .unwrap();

    let item = only_item(&preview.items);
    assert_eq!(item.status, InstallItemStatus::New);
    assert_eq!(item.mod_id, "legacy-mod");
    assert_eq!(item.root_path, "Legacy Mod");
    assert!(has_check(item, Severity::Warning, "modinfo"), "{item:#?}");
}

#[test]
fn a_plain_folder_and_a_dropped_modinfo_are_both_sources() {
    let sandbox = Sandbox::new();
    let folder = sandbox.drop_path("FolderMod");
    write_tree(
        &folder,
        &[("modinfo.json", &modinfo("FolderMod", "1", Some(8)))],
    );
    let other = sandbox.drop_path("OtherMod");
    write_tree(
        &other,
        &[("modinfo.json", &modinfo("OtherMod", "1", Some(8)))],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[folder, other.join("modinfo.json")], &[])
        .unwrap();

    let ids: Vec<&str> = preview.items.iter().map(|i| i.mod_id.as_str()).collect();
    assert_eq!(ids, ["foldermod", "othermod"]);
    assert_eq!(preview.source_name, "FolderMod, modinfo.json");
}

#[test]
fn a_7z_archive_is_staged() {
    let sandbox = Sandbox::new();
    let tree = sandbox.drop_path("seven-src");
    write_tree(
        &tree.join("Seven"),
        &[("modinfo.json", &modinfo("Seven", "3", Some(8)))],
    );
    let archive = sandbox.drop_path("Seven.7z");
    sevenz_rust2::compress_to_path(&tree, &archive).unwrap();

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    assert_eq!(only_item(&preview.items).mod_id, "seven");
}

#[test]
fn versions_compare_by_mod_id_into_update_downgrade_and_same() {
    let sandbox = Sandbox::new();
    write_tree(
        &sandbox.mods(),
        &[
            ("Up/modinfo.json", &modinfo("Up", "1.2.0", Some(8))),
            ("Down/modinfo.json", &modinfo("Down", "1.10.0", Some(8))),
            ("Same/modinfo.json", &modinfo("Same", "2.0", Some(8))),
        ],
    );
    let installed = sandbox.installed(GameId::Anno117);
    let archive = sandbox.drop_path("Versions.zip");
    let (up, down, same) = (
        modinfo("Up", "1.10.0", Some(8)),
        modinfo("Down", "1.9.5", Some(8)),
        modinfo("Same", "2.0", Some(8)),
    );
    write_zip(
        &archive,
        &[
            Entry::File("Renamed Up/modinfo.json", up.as_bytes()),
            Entry::File("Down/modinfo.json", down.as_bytes()),
            Entry::File("Same/modinfo.json", same.as_bytes()),
        ],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &installed)
        .unwrap();

    let status_of = |id: &str| {
        let item = preview.items.iter().find(|i| i.mod_id == id).unwrap();
        (item.status, item.previous_version.clone())
    };
    assert_eq!(
        status_of("up"),
        (InstallItemStatus::Update, Some("1.2.0".into()))
    );
    assert_eq!(
        status_of("down"),
        (InstallItemStatus::Downgrade, Some("1.10.0".into()))
    );
    assert_eq!(
        status_of("same"),
        (InstallItemStatus::Same, Some("2.0".into()))
    );
}

#[test]
fn a_missing_required_dependency_is_a_warning() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Needy.zip");
    let info = r#"{ "Anno": 8, "ModID": "Needy", "Version": "1",
        "Dependencies": { "Require": ["Missing_Lib"] } }"#;
    write_zip(
        &archive,
        &[Entry::File("Needy/modinfo.json", info.as_bytes())],
    );

    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    let item = only_item(&preview.items);
    assert_eq!(item.status, InstallItemStatus::New);
    assert!(
        has_check(item, Severity::Warning, "missing-lib"),
        "{item:#?}"
    );
}

#[test]
fn the_preview_serialises_in_the_ui_shape() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Old.zip");
    let info = modinfo("OldMod", "1", Some(7));
    write_zip(
        &archive,
        &[Entry::File("OldMod/modinfo.json", info.as_bytes())],
    );
    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    let json = serde_json::to_value(&preview).unwrap();
    let item = &json["items"][0];

    assert_eq!(json["sourceName"], "Old.zip");
    assert_eq!(item["rootPath"], "OldMod");
    assert_eq!(item["status"], "wrong-game");
    assert_eq!(item["targetGame"], "anno1800");
    assert_eq!(item["previousVersion"], serde_json::Value::Null);
    assert_eq!(item["checks"][0]["level"], "error");
    assert!(item["subModIds"].is_array());
}

#[test]
fn cancel_deletes_the_staging() {
    let sandbox = Sandbox::new();
    let archive = sandbox.drop_path("Gone.zip");
    let info = modinfo("Gone", "1", Some(8));
    write_zip(
        &archive,
        &[Entry::File("Gone/modinfo.json", info.as_bytes())],
    );
    let preview = sandbox
        .engine
        .preview(GameId::Anno117, &[archive], &[])
        .unwrap();

    sandbox.engine.cancel(&preview.id).unwrap();

    assert!(!sandbox.staging().join(&preview.id).exists());
    assert!(fs::read_dir(sandbox.staging()).unwrap().next().is_none());
}
