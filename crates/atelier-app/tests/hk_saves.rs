mod common;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use atelier_app::ErrorCode;
use atelier_core::GameId;
use atelier_core::humankind::saves::ModStatus;
use common::Harness;
use zip::write::SimpleFileOptions;

const HUMANKIND: GameId = GameId::Humankind;
const GUID: &str = "0123456789abcdef0123456789abcdef";
const BASE: &str = "project mercury";
const BASE_ONLY: &str = "<RuntimeModuleConfiguration>project mercury</RuntimeModuleConfiguration>";

fn game_dir(harness: &Harness) -> PathBuf {
    harness.root.path().join("My Documents").join("Humankind")
}

fn long_name(name: &str, version: &str) -> String {
    format!("{name}.{GUID}.{version}")
}

/// A module in its own Community folder, as the game expects.
fn add_module(harness: &Harness, name: &str, version: &str) {
    let long = long_name(name, version);
    let dir = game_dir(harness).join("Community").join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(&long), "module").unwrap();
    fs::write(dir.join(format!("{long}.manifest")), "manifest").unwrap();
}

/// A `.ctr` holding a descriptor with the given `\`-separated mod list.
fn write_save(harness: &Harness, name: &str, mod_list: &str) -> PathBuf {
    let mut descriptor = vec![9, 9, 9, 9];
    for text in ["ver", "V1.0", "rmc", mod_list] {
        let units: Vec<u16> = text.encode_utf16().collect();
        let count = u32::try_from(units.len()).unwrap();
        descriptor.extend((4 + 2 * count).to_le_bytes());
        descriptor.extend(count.to_le_bytes());
        units
            .iter()
            .for_each(|unit| descriptor.extend(unit.to_le_bytes()));
    }
    let path = game_dir(harness).join("Save Files/Cloud").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut archive = zip::ZipWriter::new(fs::File::create(&path).unwrap());
    archive
        .start_file("SaveDescriptor.bin", SimpleFileOptions::default())
        .unwrap();
    archive.write_all(&descriptor).unwrap();
    archive.finish().unwrap();
    path
}

fn installed_version(harness: &Harness, id: &str) -> Option<String> {
    harness
        .app
        .list_mods(HUMANKIND)
        .unwrap()
        .into_iter()
        .find(|item| item.id == id)
        .map(|item| item.version)
}

/// Puts `name` at `version` in the backups, the way an uninstall does.
fn vault(harness: &Harness, name: &str, version: &str) {
    add_module(harness, name, version);
    harness
        .app
        .uninstall_mod(HUMANKIND, &name.to_lowercase())
        .unwrap();
}

fn played_with_eventful_and_deepsea(harness: &Harness) -> PathBuf {
    let list = format!(
        "project mercury\\{}\\{}",
        long_name("Eventful", "0.7"),
        long_name("Deepsea Reloaded", "1.9")
    );
    write_save(harness, "cubans.ctr", &list)
}

#[test]
fn listing_reads_every_save_and_skips_broken_ones() {
    let harness = Harness::new();
    write_save(&harness, "one.ctr", "project mercury\\eventful");
    let broken = game_dir(&harness).join("Save Files/broken.ctr");
    fs::write(&broken, "garbage").unwrap();

    let listing = harness.app.list_saves(HUMANKIND).unwrap();

    assert_eq!(listing.saves.len(), 1);
    assert_eq!(listing.saves[0].descriptor.mods[0].name, "eventful");
    assert_eq!(listing.skipped.len(), 1);
    assert_eq!(listing.skipped[0].path, broken);
}

#[test]
fn saves_are_a_humankind_feature() {
    let harness = Harness::new();
    let error = harness.app.list_saves(GameId::Anno117).unwrap_err();
    assert_eq!(error.code, ErrorCode::NotSupported);
}

#[test]
fn inspecting_compares_each_mod_with_what_is_installed() {
    let harness = Harness::new();
    add_module(&harness, "Eventful", "0.8");
    add_module(&harness, "Deepsea Reloaded", "1.9");
    let list = format!(
        "{}\\{}\\{}",
        long_name("Eventful", "0.7"),
        long_name("Deepsea Reloaded", "1.9"),
        long_name("Gone", "1.0")
    );
    let path = write_save(&harness, "mixed.ctr", &list);

    let inspection = harness.app.inspect_save(HUMANKIND, &path).unwrap();

    let statuses: Vec<ModStatus> = inspection
        .checks
        .into_iter()
        .map(|check| check.status)
        .collect();
    assert_eq!(
        statuses,
        [
            ModStatus::OtherVersion {
                installed: "0.8".into()
            },
            ModStatus::Exact,
            ModStatus::Missing,
        ]
    );
    assert_eq!(inspection.save.name, "mixed");
}

#[test]
fn a_path_outside_the_save_folder_is_refused() {
    let harness = Harness::new();
    let save = write_save(&harness, "one.ctr", "eventful");
    let outside = harness.root.path().join("elsewhere.ctr");
    fs::copy(&save, &outside).unwrap();
    let climbing = game_dir(&harness).join("Save Files/../Save Files/../../elsewhere.ctr");

    for path in [
        outside,
        climbing,
        game_dir(&harness).join("Save Files/none.ctr"),
    ] {
        let error = harness.app.inspect_save(HUMANKIND, &path).unwrap_err();
        assert_eq!(error.code, ErrorCode::InvalidInput, "{}", path.display());
    }
}

#[test]
fn a_corrupt_save_is_refused_with_its_reason() {
    let harness = Harness::new();
    let path = game_dir(&harness).join("Save Files/broken.ctr");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "garbage").unwrap();

    let error = harness.app.inspect_save(HUMANKIND, &path).unwrap_err();

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("archive"), "{}", error.message);
}

#[test]
fn making_a_save_loadable_restores_played_versions_then_writes_its_order() {
    let harness = Harness::new();
    harness.write_humankind_registry(BASE);
    vault(&harness, "Eventful", "0.7");
    add_module(&harness, "Eventful", "0.8");
    add_module(&harness, "Deepsea Reloaded", "1.9");
    let path = played_with_eventful_and_deepsea(&harness);

    let report = harness.app.make_save_loadable(HUMANKIND, &path).unwrap();

    assert_eq!(report.restored.len(), 1);
    assert_eq!(report.restored[0].name, "eventful");
    assert!(report.missing.is_empty());
    assert!(report.registry_written);
    assert_eq!(
        installed_version(&harness, "eventful").as_deref(),
        Some("0.7")
    );
    let written = fs::read_to_string(harness.humankind_registry()).unwrap();
    assert!(
        written.contains(
            "<RuntimeModuleConfiguration>project mercury/eventful/deepsea reloaded</RuntimeModuleConfiguration>"
        ),
        "{written}"
    );
}

#[test]
fn a_version_the_vault_lacks_is_reported_and_the_registry_is_left_alone() {
    let harness = Harness::new();
    harness.write_humankind_registry(BASE);
    add_module(&harness, "Eventful", "0.8");
    let path = played_with_eventful_and_deepsea(&harness);

    let report = harness.app.make_save_loadable(HUMANKIND, &path).unwrap();

    assert!(report.restored.is_empty());
    assert!(!report.registry_written);
    let missing: Vec<(&str, &ModStatus)> = report
        .missing
        .iter()
        .map(|check| (check.module.name.as_str(), &check.status))
        .collect();
    assert_eq!(
        missing,
        [
            (
                "eventful",
                &ModStatus::OtherVersion {
                    installed: "0.8".into()
                }
            ),
            ("deepsea reloaded", &ModStatus::Missing),
        ]
    );
    assert!(
        fs::read_to_string(harness.humankind_registry())
            .unwrap()
            .contains(BASE_ONLY)
    );
}

#[test]
fn a_save_with_no_mods_loads_the_base_game_alone() {
    let harness = Harness::new();
    harness.write_humankind_registry(BASE);
    add_module(&harness, "Eventful", "0.8");
    let listed =
        "<RuntimeModuleConfiguration>project mercury/eventful</RuntimeModuleConfiguration>";
    fs::write(
        harness.humankind_registry(),
        format!("<Registry>{listed}</Registry>"),
    )
    .unwrap();
    let path = write_save(&harness, "vanilla.ctr", "project mercury");

    let report = harness.app.make_save_loadable(HUMANKIND, &path).unwrap();

    assert!(report.registry_written);
    assert!(
        fs::read_to_string(harness.humankind_registry())
            .unwrap()
            .contains(BASE_ONLY)
    );
}

#[test]
fn without_a_registry_nothing_is_restored() {
    let harness = Harness::new();
    vault(&harness, "Eventful", "0.7");
    add_module(&harness, "Eventful", "0.8");
    let path = played_with_eventful_and_deepsea(&harness);

    let error = harness
        .app
        .make_save_loadable(HUMANKIND, &path)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::NotFound);
    assert_eq!(
        installed_version(&harness, "eventful").as_deref(),
        Some("0.8")
    );
}

#[test]
fn a_running_game_blocks_making_a_save_loadable() {
    let harness = Harness::new();
    harness.write_humankind_registry(BASE);
    let path = played_with_eventful_and_deepsea(&harness);
    harness.set_running(true);

    let error = harness
        .app
        .make_save_loadable(HUMANKIND, &path)
        .unwrap_err();

    assert_eq!(error.code, ErrorCode::GameRunning);
}

fn backup_dirs(harness: &Harness) -> Vec<String> {
    let dir = harness.root.path().join("data/backups/humankind/eventful");
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn old_backup(harness: &Harness, slot: &str) -> PathBuf {
    let dir = harness
        .root
        .path()
        .join("data/backups/humankind/eventful")
        .join(slot);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn pruning_keeps_every_version_a_save_was_played_with() {
    let harness = Harness::new();
    for slot in ["0.7-1000", "0.8-2000", "0.9-3000"] {
        old_backup(&harness, slot);
    }
    played_with_eventful_and_deepsea(&harness);

    harness.app.save_settings(harness.app.settings()).unwrap();

    assert_eq!(backup_dirs(&harness), ["0.7-1000", "0.9-3000"]);
}

#[test]
fn pruning_still_expires_versions_no_save_needs() {
    let harness = Harness::new();
    for slot in ["0.7-1000", "0.8-2000"] {
        old_backup(&harness, slot);
    }

    harness.app.save_settings(harness.app.settings()).unwrap();

    assert_eq!(backup_dirs(&harness), ["0.8-2000"]);
}

/// An expired Anno backup beside its newer copy; pruning removes the expired one.
fn expired_anno_backup(harness: &Harness) -> PathBuf {
    let dir = harness.root.path().join("data/backups/anno117/old-mod");
    fs::create_dir_all(dir.join("2.0-3000")).unwrap();
    let expired = dir.join("1.0-1000");
    fs::create_dir_all(&expired).unwrap();
    expired
}

#[test]
fn a_save_that_cannot_be_read_keeps_every_humankind_backup() {
    let harness = Harness::new();
    for slot in ["0.7-1000", "0.8-2000"] {
        old_backup(&harness, slot);
    }
    let broken = game_dir(&harness).join("Save Files/broken.ctr");
    fs::create_dir_all(broken.parent().unwrap()).unwrap();
    fs::write(&broken, "garbage").unwrap();
    let anno = expired_anno_backup(&harness);

    harness.app.save_settings(harness.app.settings()).unwrap();

    assert_eq!(backup_dirs(&harness), ["0.7-1000", "0.8-2000"]);
    assert!(!anno.exists(), "Anno's backups are still pruned");
}

#[cfg(unix)]
#[test]
fn saves_that_cannot_be_listed_keep_every_humankind_backup() {
    use std::os::unix::fs::PermissionsExt;
    let harness = Harness::new();
    for slot in ["0.7-1000", "0.8-2000"] {
        old_backup(&harness, slot);
    }
    let save_files = game_dir(&harness).join("Save Files");
    fs::create_dir_all(&save_files).unwrap();
    fs::set_permissions(&save_files, fs::Permissions::from_mode(0o000)).unwrap();
    let anno = expired_anno_backup(&harness);

    let saved = harness.app.save_settings(harness.app.settings());
    fs::set_permissions(&save_files, fs::Permissions::from_mode(0o755)).unwrap();

    saved.unwrap();
    assert_eq!(backup_dirs(&harness), ["0.7-1000", "0.8-2000"]);
    assert!(!anno.exists(), "Anno's backups are still pruned");
}
