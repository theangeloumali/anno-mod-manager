mod common;

use std::fs;
use std::path::{Path, PathBuf};

use atelier_core::{GameId, Severity};
use atelier_install::{
    Error, InstallItemStatus, InstallPreview, InstallPreviewItem, Limits, PreviewOptions,
};
use common::*;

const HUMANKIND: GameId = GameId::Humankind;
const ANNO: GameId = GameId::Anno117;
const GUID: &str = "402b91b9e5724da3851e2b6b727b100a";

fn long_name(name: &str, version: &str) -> String {
    format!("{name}.{GUID}.{version}")
}

/// A module's files under `parent`: the module-name file, its manifest and an empty bundle.
fn module_files(parent: &str, name: &str, version: &str) -> Vec<(String, Vec<u8>)> {
    let long = long_name(name, version);
    let prefix = if parent.is_empty() {
        String::new()
    } else {
        format!("{parent}/")
    };
    vec![
        (format!("{prefix}{long}"), b"module".to_vec()),
        (format!("{prefix}{long}.manifest"), b"m".to_vec()),
        (
            format!("{prefix}{}.assetbundle", long.to_lowercase()),
            Vec::new(),
        ),
    ]
}

fn zip_files(path: &Path, files: &[(String, Vec<u8>)]) {
    let entries: Vec<Entry> = files
        .iter()
        .map(|(name, bytes)| Entry::File(name, bytes))
        .collect();
    write_zip(path, &entries);
}

fn drop_zip(sandbox: &Sandbox, name: &str, files: &[(String, Vec<u8>)]) -> PathBuf {
    let archive = sandbox.drop_path(name);
    zip_files(&archive, files);
    archive
}

fn preview(
    sandbox: &Sandbox,
    game: GameId,
    archive: &Path,
    options: &PreviewOptions,
) -> InstallPreview {
    let installed = sandbox.installed(game);
    sandbox
        .engine
        .preview_with_options(game, &[archive.to_path_buf()], &installed, options)
        .unwrap()
}

fn only_item(preview: &InstallPreview) -> &InstallPreviewItem {
    assert_eq!(preview.items.len(), 1, "items: {:#?}", preview.items);
    &preview.items[0]
}

fn has_check(item: &InstallPreviewItem, level: Severity, needle: &str) -> bool {
    item.checks
        .iter()
        .any(|check| check.level == level && check.message.contains(needle))
}

fn confirm(sandbox: &Sandbox, game: GameId, preview: &InstallPreview) {
    sandbox
        .engine
        .confirm(
            &preview.id,
            &sandbox.mods(),
            &sandbox.installed(game),
            false,
        )
        .unwrap();
}

/// A game folder, with BepInEx's core dll when `with_bepinex`.
fn game_dir(sandbox: &Sandbox, with_bepinex: bool) -> PathBuf {
    let dir = sandbox.root.path().join("Humankind");
    fs::create_dir_all(&dir).unwrap();
    if with_bepinex {
        write_tree(&dir, &[("BepInEx/core/BepInEx.dll", "")]);
    }
    dir
}

fn plugin_options(game_dir: Option<PathBuf>) -> PreviewOptions {
    PreviewOptions {
        game_dir,
        ..PreviewOptions::default()
    }
}

#[test]
fn a_module_previews_as_one_new_humankind_item() {
    let sandbox = Sandbox::new();
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("Sigma Pack", "Sigma Pack", "2.5"),
    );

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.mod_id, "sigma pack");
    assert_eq!(item.name, "Sigma Pack");
    assert_eq!(item.version, "2.5");
    assert_eq!(item.root_path, "Sigma Pack");
    assert_eq!(item.target_game, Some(HUMANKIND));
    assert_eq!(item.status, InstallItemStatus::New);
    let target = format!(
        "Installs to Community\\Atelier\\{}",
        long_name("Sigma Pack", "2.5")
    );
    assert!(has_check(item, Severity::Ok, &target), "{item:#?}");
    assert!(!sandbox.mods().exists(), "preview must not touch Community");
}

#[test]
fn a_dropped_module_installs_under_atelier_by_its_long_name() {
    let sandbox = Sandbox::new();
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("Wrapper", "Sigma Pack", "2.5"),
    );
    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    confirm(&sandbox, HUMANKIND, &preview);

    let long = long_name("Sigma Pack", "2.5");
    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].folder, format!("Atelier/{long}"));
    assert!(
        sandbox
            .mods()
            .join("Atelier")
            .join(&long)
            .join(&long)
            .is_file()
    );
}

#[test]
fn a_module_without_its_assetbundle_is_invalid() {
    let sandbox = Sandbox::new();
    let mut files = module_files("Sigma", "Sigma Pack", "2.5");
    files.retain(|(name, _)| !name.ends_with(".assetbundle"));
    let archive = drop_zip(&sandbox, "Sigma.zip", &files);

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(
        has_check(item, Severity::Error, ".assetbundle"),
        "{item:#?}"
    );
}

#[test]
fn a_module_nested_in_extra_folders_is_found() {
    let sandbox = Sandbox::new();
    let files = module_files("Download/ws64/Deep", "Sigma Pack", "2.5");
    let archive = drop_zip(&sandbox, "Sigma.zip", &files);

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.root_path, "Download/ws64/Deep");
    assert_eq!(item.status, InstallItemStatus::New);
}

#[test]
fn a_module_at_the_archive_root_is_wrapped_and_found() {
    let sandbox = Sandbox::new();
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("", "Sigma Pack", "2.5"),
    );

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.root_path, "Sigma");
    assert_eq!(item.status, InstallItemStatus::New);
}

#[test]
fn several_modules_in_one_archive_become_several_items() {
    let sandbox = Sandbox::new();
    let mut files = module_files("Pack/A", "Alpha", "1.0");
    files.extend(module_files("Pack/B", "Beta", "1.0"));
    let archive = drop_zip(&sandbox, "Pack.zip", &files);

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());
    confirm(&sandbox, HUMANKIND, &preview);

    let ids: Vec<&str> = preview
        .items
        .iter()
        .map(|item| item.mod_id.as_str())
        .collect();
    assert_eq!(ids, ["alpha", "beta"]);
    assert_eq!(sandbox.installed(HUMANKIND).len(), 2);
}

#[test]
fn an_anno_mod_and_a_module_in_one_archive_each_target_their_own_game() {
    let sandbox = Sandbox::new();
    let mut files = module_files("Both/Sigma", "Sigma Pack", "2.5");
    files.push((
        "Both/Roads/modinfo.json".into(),
        modinfo("roads", "1.0", Some(8)).into_bytes(),
    ));
    let archive = drop_zip(&sandbox, "Both.zip", &files);

    let on_humankind = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());
    let on_anno = preview(&sandbox, ANNO, &archive, &PreviewOptions::default());

    let status = |preview: &InstallPreview, id: &str| {
        let item = preview.items.iter().find(|item| item.mod_id == id).unwrap();
        (item.status, item.checks.clone())
    };
    assert_eq!(
        status(&on_humankind, "sigma pack").0,
        InstallItemStatus::New
    );
    let (roads_status, roads_checks) = status(&on_humankind, "roads");
    assert_eq!(roads_status, InstallItemStatus::WrongGame);
    assert!(roads_checks[0].message.contains("Humankind won't load it"));
    assert_eq!(status(&on_anno, "roads").0, InstallItemStatus::New);
    let (sigma_status, sigma_checks) = status(&on_anno, "sigma pack");
    assert_eq!(sigma_status, InstallItemStatus::WrongGame);
    assert!(sigma_checks[0].message.starts_with("Built for Humankind"));
}

#[test]
fn an_anno_mod_without_a_game_marker_is_still_wrong_for_humankind() {
    let sandbox = Sandbox::new();
    let info = modinfo("roads", "1.0", None);
    let files = vec![("Roads/modinfo.json".to_string(), info.into_bytes())];
    let archive = drop_zip(&sandbox, "Roads.zip", &files);

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::WrongGame);
    assert!(
        has_check(item, Severity::Error, "Built for Anno"),
        "{item:#?}"
    );
}

#[test]
fn a_modio_download_installs_under_its_modio_folder() {
    let sandbox = Sandbox::new();
    let archive = drop_zip(
        &sandbox,
        "1_2.zip",
        &module_files("Sigma Pack", "Sigma Pack", "2.5"),
    );
    let options = PreviewOptions {
        modio_folder: Some("4801_77001".into()),
        ..PreviewOptions::default()
    };

    let preview = preview(&sandbox, HUMANKIND, &archive, &options);
    let long = long_name("Sigma Pack", "2.5");
    let target = format!("Installs to Community\\Atelier\\4801_77001\\{long}");
    assert!(has_check(only_item(&preview), Severity::Ok, &target));
    confirm(&sandbox, HUMANKIND, &preview);

    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed[0].folder, format!("Atelier/4801_77001/{long}"));
}

#[test]
fn a_newer_modfile_replaces_the_old_one_and_backs_it_up() {
    let sandbox = Sandbox::new();
    for (version, modfile) in [("2.5", "4801_1"), ("2.6", "4801_2")] {
        let archive = drop_zip(
            &sandbox,
            "Sigma.zip",
            &module_files("S", "Sigma Pack", version),
        );
        let options = PreviewOptions {
            modio_folder: Some(modfile.into()),
            ..PreviewOptions::default()
        };
        let preview = preview(&sandbox, HUMANKIND, &archive, &options);
        confirm(&sandbox, HUMANKIND, &preview);
    }

    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed.len(), 1);
    assert_eq!(installed[0].version, "2.6");
    assert!(
        !sandbox.mods().join("Atelier/4801_1").exists(),
        "emptied modfile folder stays"
    );
    let backups = sandbox
        .engine
        .backup_versions(HUMANKIND, "sigma pack")
        .unwrap();
    assert_eq!(backups[0].version, "2.5");
}

#[test]
fn an_update_shows_the_module_wording() {
    let sandbox = Sandbox::new();
    let first = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("S", "Sigma Pack", "2.5"),
    );
    let preview_one = preview(&sandbox, HUMANKIND, &first, &PreviewOptions::default());
    confirm(&sandbox, HUMANKIND, &preview_one);
    let second = drop_zip(
        &sandbox,
        "Sigma2.zip",
        &module_files("S", "Sigma Pack", "2.6"),
    );

    let preview_two = preview(&sandbox, HUMANKIND, &second, &PreviewOptions::default());

    let item = only_item(&preview_two);
    assert_eq!(item.status, InstallItemStatus::Update);
    assert!(
        has_check(item, Severity::Ok, "Same module, newer version"),
        "{item:#?}"
    );
}

#[test]
fn a_copy_outside_atelier_is_a_duplicate_and_nothing_installs() {
    let sandbox = Sandbox::new();
    let long = long_name("Sigma Pack", "2.5");
    let existing = sandbox.mods().join("mod.io/4801_77001").join(&long);
    write_tree(&existing, &[(&long, "module")]);
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("S", "Sigma Pack", "2.6"),
    );

    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    let place = format!("Community\\mod.io\\4801_77001\\{long}");
    assert!(has_check(item, Severity::Error, &place), "{item:#?}");
    confirm(&sandbox, HUMANKIND, &preview);
    assert!(!sandbox.mods().join("Atelier").exists());
}

#[test]
fn a_plugin_needs_the_game_folder() {
    let sandbox = Sandbox::new();
    let files = vec![("BepInEx/plugins/Cool.dll".to_string(), b"MZ".to_vec())];
    let archive = drop_zip(&sandbox, "CoolPlugin.zip", &files);

    let preview = preview(&sandbox, HUMANKIND, &archive, &plugin_options(None));

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(
        has_check(item, Severity::Error, "install folder"),
        "{item:#?}"
    );
}

#[test]
fn a_plugin_without_bepinex_installed_asks_for_bepinex_first() {
    let sandbox = Sandbox::new();
    let game = game_dir(&sandbox, false);
    let files = vec![("Cool.dll".to_string(), b"MZ".to_vec())];
    let archive = drop_zip(&sandbox, "CoolPlugin.zip", &files);

    let preview = preview(
        &sandbox,
        HUMANKIND,
        &archive,
        &plugin_options(Some(game.clone())),
    );

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(
        has_check(item, Severity::Error, "Install BepInEx first"),
        "{item:#?}"
    );
    assert!(!game.join("BepInEx").exists());
}

#[test]
fn a_modio_download_never_installs_a_plugin() {
    let sandbox = Sandbox::new();
    let game = game_dir(&sandbox, true);
    let files = vec![("BepInEx/plugins/Cool.dll".to_string(), b"MZ".to_vec())];
    let archive = drop_zip(&sandbox, "CoolPlugin.zip", &files);
    let options = PreviewOptions {
        game_dir: Some(game.clone()),
        modio_folder: Some("123_456".into()),
        ..PreviewOptions::default()
    };

    let preview = preview(&sandbox, HUMANKIND, &archive, &options);

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(
        has_check(item, Severity::Error, "drop the file"),
        "{item:#?}"
    );
    assert!(!game.join("BepInEx/plugins/CoolPlugin").exists());
}

#[test]
fn a_plugin_tree_installs_into_bepinex_plugins_named_after_the_archive() {
    let sandbox = Sandbox::new();
    let game = game_dir(&sandbox, true);
    let files = vec![
        ("BepInEx/plugins/Cool.dll".to_string(), b"MZ".to_vec()),
        ("BepInEx/config/cool.cfg".to_string(), b"x=1".to_vec()),
    ];
    let archive = drop_zip(&sandbox, "CoolPlugin.zip", &files);

    let preview = preview(
        &sandbox,
        HUMANKIND,
        &archive,
        &plugin_options(Some(game.clone())),
    );
    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::New);
    assert_eq!(item.target_game, Some(HUMANKIND));
    assert!(
        has_check(item, Severity::Ok, "BepInEx\\plugins\\CoolPlugin"),
        "{item:#?}"
    );
    assert!(has_check(item, Severity::Warning, "config"), "{item:#?}");
    confirm(&sandbox, HUMANKIND, &preview);

    assert!(game.join("BepInEx/plugins/CoolPlugin/Cool.dll").is_file());
    assert!(!sandbox.mods().join("Atelier").exists());
}

#[test]
fn loose_dlls_are_a_plugin_and_a_reinstall_backs_up_the_old_one() {
    let sandbox = Sandbox::new();
    let game = game_dir(&sandbox, true);
    let options = plugin_options(Some(game.clone()));
    for bytes in [b"v1", b"v2"] {
        let files = vec![("Cool.dll".to_string(), bytes.to_vec())];
        let archive = drop_zip(&sandbox, "CoolPlugin.zip", &files);
        let preview = preview(&sandbox, HUMANKIND, &archive, &options);
        confirm(&sandbox, HUMANKIND, &preview);
    }

    let dll = game.join("BepInEx/plugins/CoolPlugin/Cool.dll");
    assert_eq!(fs::read(dll).unwrap(), b"v2");
    assert_eq!(
        sandbox
            .engine
            .backup_versions(HUMANKIND, "coolplugin")
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn loose_dlls_on_anno_are_not_a_plugin() {
    let sandbox = Sandbox::new();
    let files = vec![("Pack/Cool.dll".to_string(), b"MZ".to_vec())];
    let archive = drop_zip(&sandbox, "Cool.zip", &files);

    let preview = preview(&sandbox, ANNO, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(has_check(
        item,
        Severity::Error,
        "No modinfo.json or data folder"
    ));
}

#[test]
fn a_module_archive_with_an_unsafe_entry_is_refused() {
    let sandbox = Sandbox::new();
    let mut files = module_files("S", "Sigma Pack", "2.5");
    files.push(("../escape.txt".into(), b"x".to_vec()));
    let archive = drop_zip(&sandbox, "Sigma.zip", &files);

    let result =
        sandbox
            .engine
            .preview_with_options(HUMANKIND, &[archive], &[], &PreviewOptions::default());

    assert!(
        matches!(result, Err(Error::UnsafePath { .. })),
        "{result:?}"
    );
}

#[test]
fn a_module_archive_over_the_size_limit_is_refused() {
    let sandbox = Sandbox::new();
    let mut files = module_files("S", "Sigma Pack", "2.5");
    files.push(("S/big.bin".into(), vec![0; 4096]));
    let archive = drop_zip(&sandbox, "Sigma.zip", &files);
    let options = PreviewOptions {
        limits: Some(Limits {
            max_total_bytes: 1024,
            max_entries: 100,
        }),
        ..PreviewOptions::default()
    };

    let result = sandbox
        .engine
        .preview_with_options(HUMANKIND, &[archive], &[], &options);

    assert!(matches!(result, Err(Error::TooLarge { .. })), "{result:?}");
}

#[test]
fn restoring_a_removed_module_puts_it_back_under_atelier() {
    let sandbox = Sandbox::new();
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("S", "Sigma Pack", "2.5"),
    );
    let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());
    confirm(&sandbox, HUMANKIND, &preview);
    let long = long_name("Sigma Pack", "2.5");
    let dir = sandbox.mods().join("Atelier").join(&long);
    sandbox.engine.remove_to_backup(HUMANKIND, &dir).unwrap();

    sandbox
        .engine
        .rollback(HUMANKIND, "sigma pack", &sandbox.mods())
        .unwrap();

    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed.len(), 1);
    assert!(
        installed[0].folder.starts_with("Atelier/"),
        "{}",
        installed[0].folder
    );
}

#[test]
fn restoring_over_a_newer_module_replaces_it_where_it_lives() {
    let sandbox = Sandbox::new();
    for version in ["2.5", "2.6"] {
        let archive = drop_zip(
            &sandbox,
            "Sigma.zip",
            &module_files("S", "Sigma Pack", version),
        );
        let preview = preview(&sandbox, HUMANKIND, &archive, &PreviewOptions::default());
        confirm(&sandbox, HUMANKIND, &preview);
    }

    sandbox
        .engine
        .rollback(HUMANKIND, "sigma pack", &sandbox.mods())
        .unwrap();

    let installed = sandbox.installed(HUMANKIND);
    assert_eq!(installed.len(), 1, "{installed:#?}");
    assert_eq!(installed[0].version, "2.5");
}

#[test]
fn a_lone_module_on_anno_is_built_for_humankind() {
    let sandbox = Sandbox::new();
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("", "Sigma Pack", "2.5"),
    );

    let preview = preview(&sandbox, ANNO, &archive, &PreviewOptions::default());

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::WrongGame);
    assert_eq!(item.target_game, Some(HUMANKIND));
    assert!(
        has_check(item, Severity::Error, "Built for Humankind"),
        "{item:#?}"
    );
}

#[test]
fn a_workshop_copy_is_a_duplicate_and_nothing_installs() {
    let sandbox = Sandbox::new();
    let long = long_name("Sigma Pack", "2.5");
    let workshop = sandbox.root.path().join("workshop/content/1124300/3001");
    write_tree(&workshop.join(&long), &[(&long, "module")]);
    let folder = atelier_core::scan::ModsFolder {
        path: workshop,
        location: atelier_core::ModLocation::Workshop,
    };
    let installed = atelier_core::scan::scan(HUMANKIND, &[folder]).unwrap().mods;
    let archive = drop_zip(
        &sandbox,
        "Sigma.zip",
        &module_files("S", "Sigma Pack", "2.6"),
    );

    let preview = sandbox
        .engine
        .preview_with_options(
            HUMANKIND,
            &[archive],
            &installed,
            &PreviewOptions::default(),
        )
        .unwrap();

    let item = only_item(&preview);
    assert_eq!(item.status, InstallItemStatus::Invalid);
    assert!(
        has_check(item, Severity::Error, "Steam Workshop folder"),
        "{item:#?}"
    );
}

#[test]
fn a_dll_beside_a_module_is_part_of_the_module_not_a_plugin() {
    let sandbox = Sandbox::new();
    let mut files = module_files("Pack", "Alpha", "1.0");
    files.push(("Pack/Helper.dll".to_string(), b"MZ".to_vec()));
    let archive = drop_zip(&sandbox, "Pack.zip", &files);
    let game = game_dir(&sandbox, true);

    let preview = preview(&sandbox, HUMANKIND, &archive, &plugin_options(Some(game)));

    assert_eq!(only_item(&preview).mod_id, "alpha");
}
