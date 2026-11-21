mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use atelier_core::humankind::bundle::{self, BundleError, ModuleType};
use atelier_core::humankind::module_name::{self, ModuleName};
use atelier_core::humankind::{detect, registry};
use atelier_core::scan::{self, SkipReason};
use atelier_core::{ActivationScheme, Error, GameId, Mod, ModLocation, ModOrigin, activation};
use common::{documents, folder_at, snapshots, test_mod};

/// Not a real Steam id; the fixture registry was copied from a fake-named folder.
const USER: &str = "0x0000000000000001";
const DEEPSEA_LONG: &str = "DeepSea Reloaded.a76c474eb14d46cab94bde2e802b5ba8.3.3";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/humankind")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// `Documents/Humankind` as the game lays it out, rebuilt from fixtures in a tempdir.
fn game_dir(root: &Path) -> PathBuf {
    let dir = root.join("Humankind");
    copy_tree(&fixtures().join("Community"), &dir.join("Community"));
    let user = dir.join("Users").join(USER);
    fs::create_dir_all(&user).unwrap();
    fs::copy(fixtures().join("Registry.xml"), user.join("Registry.xml")).unwrap();
    dir
}

fn registry_file(game: &Path) -> PathBuf {
    game.join("Users").join(USER).join("Registry.xml")
}

fn module(id: &str) -> Mod {
    Mod {
        game: GameId::Humankind,
        folder: format!("mod.io/1_1/{id}"),
        ..test_mod(id, "Extension", id)
    }
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn set_modified(path: &Path, seconds_ago: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(SystemTime::now() - Duration::from_secs(seconds_ago))
        .unwrap();
}

// module_name

#[test]
fn long_name_splits_into_name_guid_and_version() {
    let parsed = ModuleName::parse(DEEPSEA_LONG).unwrap();
    assert_eq!(parsed.name, "DeepSea Reloaded");
    assert_eq!(parsed.guid, "a76c474eb14d46cab94bde2e802b5ba8");
    assert_eq!((parsed.major, parsed.minor), (3, 3));
    assert_eq!(parsed.version(), "3.3");
    assert_eq!(parsed.to_string(), DEEPSEA_LONG);
}

#[test]
fn short_name_is_the_lowercased_name() {
    let parsed = ModuleName::parse(DEEPSEA_LONG).unwrap();
    assert_eq!(parsed.short_name(), "deepsea reloaded");
    let chinese = ModuleName::parse("神话-自然奇观.6dd979469e6f4c92a35ed83dfce869bd.1.1").unwrap();
    assert_eq!(chinese.short_name(), "神话-自然奇观");
}

#[test]
fn name_may_hold_dots_and_minor_may_exceed_a_byte() {
    let parsed =
        ModuleName::parse("Mod.With.Dots.CD3480E932114F8084DB755DDD65F2D8.1.2377").unwrap();
    assert_eq!(parsed.name, "Mod.With.Dots");
    assert_eq!(parsed.version(), "1.2377");
}

#[test]
fn non_module_names_are_rejected() {
    for name in [
        "DeepSea Reloaded.a76c474eb14d46cab94bde2e802b5ba8.3.3.manifest",
        "deepsea reloaded.a76c474eb14d46cab94bde2e802b5ba8.3.3.assetbundle",
        "DeepSea Reloaded.a76c474eb14d46cab94bde2e802b5b.3.3",
        "DeepSea Reloaded.a76c474eb14d46cab94bde2e802b5bz.3.3",
        "DeepSea Reloaded.a76c474eb14d46cab94bde2e802b5ba8.3.40000",
        "DeepSea Reloaded.a76c474eb14d46cab94bde2e802b5ba8.+3.3",
        ".a76c474eb14d46cab94bde2e802b5ba8.3.3",
        "project mercury",
    ] {
        assert_eq!(ModuleName::parse(name), None, "{name}");
    }
}

#[test]
fn registry_entries_map_to_short_names() {
    assert_eq!(module_name::short_name(DEEPSEA_LONG), "deepsea reloaded");
    assert_eq!(
        module_name::short_name("Project Mercury"),
        "project mercury"
    );
}

// registry

#[test]
fn registry_reads_the_ordered_module_list() {
    let tmp = tempfile::tempdir().unwrap();
    let path = registry_file(&game_dir(tmp.path()));
    assert_eq!(
        registry::read_modules(&path).unwrap(),
        names(&["project mercury"])
    );
}

#[test]
fn missing_registry_reads_as_the_base_game_only() {
    let tmp = tempfile::tempdir().unwrap();
    let modules = registry::read_modules(&tmp.path().join("Registry.xml")).unwrap();
    assert_eq!(modules, names(&[registry::BASE_MODULE]));
}

#[test]
fn registry_write_changes_only_the_module_value() {
    let tmp = tempfile::tempdir().unwrap();
    let path = registry_file(&game_dir(tmp.path()));
    let original = fs::read(&path).unwrap();
    let list = names(&[
        "project mercury",
        "deepsea reloaded",
        "comp super pack & vip",
    ]);
    registry::write_modules(&path, &list, &snapshots(tmp.path())).unwrap();

    let written = fs::read(&path).unwrap();
    let old_value = b"project mercury</RuntimeModuleConfiguration>";
    let start = find(&original, old_value);
    let new_value = b"project mercury/deepsea reloaded/comp super pack &amp; vip";
    assert_eq!(&written[..start], &original[..start]);
    assert_eq!(&written[start..start + new_value.len()], new_value);
    let tail = start + b"project mercury".len();
    assert_eq!(&written[start + new_value.len()..], &original[tail..]);
    assert_eq!(registry::read_modules(&path).unwrap(), list);
}

fn find(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
        .unwrap()
}

#[test]
fn rewriting_the_same_list_keeps_every_byte() {
    let tmp = tempfile::tempdir().unwrap();
    let path = registry_file(&game_dir(tmp.path()));
    let original = fs::read(&path).unwrap();
    registry::write_modules(&path, &names(&["project mercury"]), &snapshots(tmp.path())).unwrap();
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn registry_write_snapshots_the_old_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = registry_file(&game_dir(tmp.path()));
    registry::write_modules(&path, &names(&["deepsea reloaded"]), &snapshots(tmp.path())).unwrap();
    let copies: Vec<_> = fs::read_dir(tmp.path().join("snapshots"))
        .unwrap()
        .collect();
    assert_eq!(copies.len(), 1);
}

#[test]
fn base_module_is_forced_first_and_duplicates_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = registry_file(&game_dir(tmp.path()));
    let list = names(&["deepsea reloaded", "project mercury", "deepsea reloaded"]);
    registry::write_modules(&path, &list, &snapshots(tmp.path())).unwrap();
    let expected = names(&["project mercury", "deepsea reloaded"]);
    assert_eq!(registry::read_modules(&path).unwrap(), expected);
}

#[test]
fn registry_without_the_module_element_gains_one() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("Registry.xml");
    fs::write(
        &path,
        "\u{feff}<?xml version=\"1.0\"?>\r\n<Registry>\r\n  <A>1</A>\r\n</Registry>",
    )
    .unwrap();
    registry::write_modules(&path, &names(&["deepsea reloaded"]), &snapshots(tmp.path())).unwrap();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.ends_with(
        "  <A>1</A>\r\n  <RuntimeModule>\r\n    <RuntimeModuleConfiguration>project mercury/deepsea reloaded</RuntimeModuleConfiguration>\r\n  </RuntimeModule>\r\n</Registry>"
    ));
    assert!(text.starts_with('\u{feff}'));
}

#[test]
fn registry_without_an_end_tag_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("Registry.xml");
    fs::write(&path, "<Registry>").unwrap();
    let error = registry::write_modules(&path, &names(&["x"]), &snapshots(tmp.path())).unwrap_err();
    assert!(matches!(error, Error::InvalidRegistry(_)), "{error:?}");
    assert_eq!(fs::read_to_string(&path).unwrap(), "<Registry>");
}

// detect

#[test]
fn newest_user_registry_is_picked() {
    let tmp = tempfile::tempdir().unwrap();
    let game = game_dir(tmp.path());
    let other = game.join("Users").join("0x0000000000000002");
    fs::create_dir_all(&other).unwrap();
    fs::write(other.join("Registry.xml"), "<Registry></Registry>").unwrap();
    set_modified(&registry_file(&game), 3600);
    assert_eq!(
        detect::registry_path(&game).unwrap(),
        Some(other.join("Registry.xml"))
    );
    set_modified(&other.join("Registry.xml"), 7200);
    assert_eq!(
        detect::registry_path(&game).unwrap(),
        Some(registry_file(&game))
    );
}

#[test]
fn no_users_folder_means_no_registry() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(detect::registry_path(tmp.path()).unwrap(), None);
}

// bundle

#[test]
fn bundle_reader_finds_the_runtime_module() {
    let path = fixtures()
        .join("Workshop/2968197432")
        .join(format!("{}.assetbundle", DEEPSEA_LONG.to_lowercase()));
    let info = bundle::read_module_info(&path).unwrap();
    assert_eq!(info.module_type, ModuleType::Extension);
    assert_eq!(info.author, "FreeThinker");
    assert!(
        info.description
            .starts_with("New Districts and Infrastructure on Coastal Waters")
    );
}

#[test]
fn manifest_bundle_holds_no_runtime_module() {
    let path = fixtures().join("Workshop/2968197432").join(DEEPSEA_LONG);
    let error = bundle::read_module_info(&path).unwrap_err();
    assert!(matches!(error, BundleError::NoRuntimeModule), "{error:?}");
}

#[test]
fn broken_bundles_are_errors_not_panics() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fs::read(
        fixtures()
            .join("Workshop/2968197432")
            .join(format!("{}.assetbundle", DEEPSEA_LONG.to_lowercase())),
    )
    .unwrap();
    for length in [0, 7, 40, 100, 300, 1000, source.len() - 1] {
        let path = tmp.path().join(format!("cut-{length}"));
        fs::write(&path, &source[..length]).unwrap();
        assert!(bundle::read_module_info(&path).is_err(), "{length}");
    }
    let path = tmp.path().join("noise");
    fs::write(&path, b"not a bundle at all").unwrap();
    assert!(matches!(
        bundle::read_module_info(&path),
        Err(BundleError::NotUnityFs)
    ));
}

#[test]
fn module_type_follows_the_game_enum() {
    assert_eq!(ModuleType::from_raw(0), Some(ModuleType::Standalone));
    assert_eq!(ModuleType::from_raw(1), Some(ModuleType::Conversion));
    assert_eq!(ModuleType::from_raw(2), Some(ModuleType::Extension));
    assert_eq!(ModuleType::from_raw(3), None);
    assert_eq!(ModuleType::Conversion.as_str(), "Conversion");
}

// scan

fn scan_fixtures(root: &Path) -> Vec<Mod> {
    let game = game_dir(root);
    let workshop = root.join("Workshop");
    copy_tree(&fixtures().join("Workshop"), &workshop);
    let folders = [
        folder_at(&root.join("Public/Modification"), ModLocation::Game),
        folder_at(&workshop, ModLocation::Workshop),
        documents(&game.join("Community")),
    ];
    scan::scan(GameId::Humankind, &folders).unwrap().mods
}

fn by_id<'a>(mods: &'a [Mod], id: &str) -> &'a Mod {
    mods.iter().find(|candidate| candidate.id == id).unwrap()
}

#[test]
fn scan_lists_one_mod_per_module() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = scan_fixtures(tmp.path());
    assert_eq!(mods.len(), 6);
    let deepsea = by_id(&mods, "deepsea reloaded");
    assert_eq!(deepsea.game, GameId::Humankind);
    assert_eq!(deepsea.name, "DeepSea Reloaded");
    assert_eq!(deepsea.version, "3.3");
    assert_eq!(deepsea.folder, "2968197432");
    assert_eq!(deepsea.location, ModLocation::Workshop);
    assert_eq!(deepsea.origin, ModOrigin::Local);
    assert_eq!(deepsea.category, "Extension");
    assert_eq!(deepsea.author.as_deref(), Some("FreeThinker"));
    assert!(deepsea.size_bytes > 0);
    assert!(deepsea.active);
}

#[test]
fn scan_marks_mod_io_installs() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = scan_fixtures(tmp.path());
    let sigma = by_id(&mods, "sigma pack");
    assert_eq!(
        sigma.folder,
        "mod.io/3506667_4753257/Sigma Pack.402b91b9e5724da3851e2b6b727b100a.2.5"
    );
    assert_eq!(sigma.location, ModLocation::Documents);
    assert_eq!(
        (sigma.origin, sigma.modio_id),
        (ModOrigin::Modio, Some(3506667))
    );
    let nested = by_id(&mods, "no city cap");
    assert_eq!(
        nested.folder,
        "mod.io/3421821_4404837/ws64/No City Cap.cda96359ca2548959fe428f029e7fe75.1.13"
    );
    assert_eq!(nested.modio_id, Some(3421821));
}

#[test]
fn unreadable_bundle_leaves_the_type_blank() {
    let tmp = tempfile::tempdir().unwrap();
    let mods = scan_fixtures(tmp.path());
    let sigma = by_id(&mods, "sigma pack");
    assert_eq!(sigma.category, "");
    assert_eq!(sigma.author, None);
    assert_eq!(by_id(&mods, "神话-自然奇观").version, "1.1");
}

#[test]
fn module_at_the_folder_root_is_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join(DEEPSEA_LONG), b"").unwrap();
    let result = scan::scan(GameId::Humankind, &[documents(tmp.path())]).unwrap();
    assert!(result.mods.is_empty());
    assert!(matches!(result.skipped[0].reason, SkipReason::Invalid(_)));
}

// activation

#[test]
fn registry_list_decides_which_modules_are_on() {
    let tmp = tempfile::tempdir().unwrap();
    let game = game_dir(tmp.path());
    let path = registry_file(&game);
    let list = names(&["project mercury", DEEPSEA_LONG]);
    registry::write_modules(&path, &list, &snapshots(tmp.path())).unwrap();
    let state = activation::read(GameId::Humankind, &game.join("Community")).unwrap();
    assert_eq!(state.scheme, ActivationScheme::RegistryList);
    assert!(state.is_active("deepsea reloaded", "2968197432"));
    assert!(!state.is_active("sigma pack", "mod.io/3506667_4753257/x"));
    let expected = names(&["project mercury", "deepsea reloaded"]);
    assert_eq!(state.registry_modules(), Some(&expected[..]));
}

#[test]
fn no_registry_reads_as_every_mod_off() {
    let tmp = tempfile::tempdir().unwrap();
    let state = activation::read(GameId::Humankind, &tmp.path().join("Community")).unwrap();
    assert!(!state.is_active("deepsea reloaded", "2968197432"));
    assert_eq!(
        state.registry_modules(),
        Some(&names(&["project mercury"])[..])
    );
}

#[test]
fn switching_on_appends_and_off_removes() {
    let tmp = tempfile::tempdir().unwrap();
    let game = game_dir(tmp.path());
    let community = documents(&game.join("Community"));
    let store = snapshots(tmp.path());
    for id in ["sigma pack", "deepsea reloaded", "sigma pack"] {
        activation::set_active(GameId::Humankind, &community, &module(id), true, &store).unwrap();
    }
    let path = registry_file(&game);
    let expected = names(&["project mercury", "sigma pack", "deepsea reloaded"]);
    assert_eq!(registry::read_modules(&path).unwrap(), expected);

    activation::set_active(
        GameId::Humankind,
        &community,
        &module("sigma pack"),
        false,
        &store,
    )
    .unwrap();
    let expected = names(&["project mercury", "deepsea reloaded"]);
    assert_eq!(registry::read_modules(&path).unwrap(), expected);
}

#[test]
fn switching_off_drops_a_long_name_entry_too() {
    let tmp = tempfile::tempdir().unwrap();
    let game = game_dir(tmp.path());
    let path = registry_file(&game);
    let store = snapshots(tmp.path());
    registry::write_modules(&path, &names(&[DEEPSEA_LONG]), &store).unwrap();
    let community = documents(&game.join("Community"));
    activation::set_active(
        GameId::Humankind,
        &community,
        &module("deepsea reloaded"),
        false,
        &store,
    )
    .unwrap();
    assert_eq!(
        registry::read_modules(&path).unwrap(),
        names(&["project mercury"])
    );
}

#[test]
fn base_game_module_cannot_be_switched_off() {
    let tmp = tempfile::tempdir().unwrap();
    let game = game_dir(tmp.path());
    let community = documents(&game.join("Community"));
    let base = module("project mercury");
    let error = activation::set_active(
        GameId::Humankind,
        &community,
        &base,
        false,
        &snapshots(tmp.path()),
    )
    .unwrap_err();
    assert!(matches!(error, Error::BaseModule(_)), "{error:?}");
}

#[test]
fn switching_without_a_registry_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let community = documents(&tmp.path().join("Humankind/Community"));
    let error = activation::set_active(
        GameId::Humankind,
        &community,
        &module("sigma pack"),
        true,
        &snapshots(tmp.path()),
    )
    .unwrap_err();
    assert!(matches!(error, Error::RegistryNotFound(_)), "{error:?}");
}
