mod common;

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use atelier_core::humankind::saves::{
    self, ModStatus, SAVE_FILES_DIR, SaveError, SaveMod, parse_descriptor,
};
use atelier_core::{GameId, Mod};
use common::test_mod;
use zip::write::SimpleFileOptions;

const GUID: &str = "0123456789abcdef0123456789abcdef";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/humankind/saves/cubans-turn-386.ctr")
}

/// The descriptor's strings: `u32 4 + 2n`, `u32 n`, then n UTF-16LE units.
fn encode(text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let count = u32::try_from(units.len()).unwrap();
    let mut bytes = Vec::new();
    bytes.extend((4 + 2 * count).to_le_bytes());
    bytes.extend(count.to_le_bytes());
    units
        .iter()
        .for_each(|unit| bytes.extend(unit.to_le_bytes()));
    bytes
}

/// A header the parser must step over, then the given strings back to back.
fn descriptor(strings: &[&str]) -> Vec<u8> {
    let mut bytes = vec![5, 0, 0, 0, 0x71, 0x3c, 0, 0, 1, 2, 3];
    strings.iter().for_each(|text| bytes.extend(encode(text)));
    bytes.extend([0, 0, 0, 0, 7]);
    bytes
}

fn write_ctr(path: &Path, entries: &[(&str, &[u8])]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut archive = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        archive
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap();
}

fn save_with_mods(path: &Path, mod_list: &str) {
    let bytes = descriptor(&["ver", "V1.0", "rmc", mod_list]);
    write_ctr(path, &[("SaveDescriptor.bin", &bytes)]);
}

fn names(mods: &[SaveMod]) -> Vec<&str> {
    mods.iter().map(|module| module.name.as_str()).collect()
}

fn installed(id: &str, version: &str) -> Mod {
    Mod {
        game: GameId::Humankind,
        version: version.into(),
        ..test_mod(id, "", id)
    }
}

#[test]
fn the_real_save_lists_its_fourteen_mods_in_order() {
    let save = saves::read_save(&fixture()).unwrap();
    assert_eq!(
        names(&save.descriptor.mods),
        [
            "emblematic garrisons",
            "expansionextras",
            "long range nuclear missiles",
            "eventful",
            "missileboy",
            "eternal eqs",
            "denser districts",
            "territorial growth",
            "era stars",
            "trade money",
            "eq's inheritance",
            "housingdistrict",
            "deepsea reloaded",
            "unique infrastructures",
        ]
    );
    assert_eq!(save.descriptor.mods[0].version, None);
    assert_eq!(save.descriptor.mods[3].version.as_deref(), Some("0.7"));
    assert_eq!(save.descriptor.mods[13].version.as_deref(), Some("1.265"));
}

#[test]
fn the_real_save_reports_build_world_size_and_empires() {
    let save = saves::read_save(&fixture()).unwrap();
    assert_eq!(
        save.descriptor.build.as_deref(),
        Some("V1.18.3510-S10, RELEASE (64-bit Standalone, build: 264005)")
    );
    assert_eq!(save.descriptor.world_size.as_deref(), Some("Huge"));
    assert_eq!(save.descriptor.empire_count, Some(10));
    let option = |key: &str| {
        save.descriptor
            .options
            .iter()
            .find(|option| option.key == key)
            .map(|option| option.value.as_str())
    };
    assert_eq!(option("GameSpeed"), Some("Slow"));
    assert_eq!(option("Climate"), Some("Hot&Dry"));
    assert_eq!(save.name, "cubans-turn-386");
}

#[test]
fn the_committed_fixture_holds_no_account_id_or_player_name() {
    let mut archive = zip::ZipArchive::new(fs::File::open(fixture()).unwrap()).unwrap();
    assert_eq!(archive.len(), 1);
    let mut bytes = Vec::new();
    archive
        .by_name("SaveDescriptor.bin")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let utf16 =
        |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    for personal in ["116560993", "Gelo"] {
        let needle = utf16(personal);
        assert!(!bytes.windows(needle.len()).any(|window| window == needle));
    }
}

#[test]
fn no_mod_list_or_only_the_base_game_means_zero_mods() {
    for strings in [
        vec!["ver", "V1.0", "rmc", "project mercury"],
        vec!["ver", "V1.0", "rmc", ""],
        vec!["ver", "V1.0"],
    ] {
        let parsed = parse_descriptor(&descriptor(&strings));
        assert!(parsed.mods.is_empty(), "{strings:?}");
        assert_eq!(parsed.build.as_deref(), Some("V1.0"));
    }
}

#[test]
fn an_empty_mod_list_does_not_swallow_the_next_string() {
    let parsed = parse_descriptor(&descriptor(&["rmc", "", "GameOption_WorldSize", "Tiny"]));
    assert!(parsed.mods.is_empty());
    assert_eq!(parsed.world_size.as_deref(), Some("Tiny"));
}

#[test]
fn a_mod_name_with_dots_keeps_them() {
    let list = format!("project mercury\\v2.0 fix.pack.{GUID}.1.2");
    let parsed = parse_descriptor(&descriptor(&["rmc", &list]));
    assert_eq!(names(&parsed.mods), ["v2.0 fix.pack"]);
    assert_eq!(parsed.mods[0].version.as_deref(), Some("1.2"));
}

#[test]
fn a_mod_listed_twice_appears_once_at_its_first_place() {
    let list = format!("alpha\\beta.{GUID}.1.0\\Alpha.{GUID}.2.0\\\\beta");
    let parsed = parse_descriptor(&descriptor(&["rmc", &list]));
    assert_eq!(names(&parsed.mods), ["alpha", "beta"]);
    assert_eq!(parsed.mods[0].version, None);
    assert_eq!(parsed.mods[1].version.as_deref(), Some("1.0"));
}

#[test]
fn an_option_without_a_value_reads_as_empty() {
    let parsed = parse_descriptor(&descriptor(&[
        "GameOption_Seed",
        "",
        "GameOption_SlotCount",
        "many",
    ]));
    assert_eq!(parsed.options[0].key, "Seed");
    assert_eq!(parsed.options[0].value, "");
    assert_eq!(parsed.empire_count, None);
}

#[test]
fn every_truncation_of_the_real_descriptor_parses_without_panicking() {
    let mut archive = zip::ZipArchive::new(fs::File::open(fixture()).unwrap()).unwrap();
    let mut bytes = Vec::new();
    archive
        .by_index(0)
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    for end in (0..bytes.len()).step_by(97) {
        parse_descriptor(&bytes[..end]);
    }
    assert!(parse_descriptor(&[0xff; 64]).mods.is_empty());
}

/// A header every 8 bytes, each claiming a string that runs to the end of the buffer.
fn header_bomb(len: usize) -> Vec<u8> {
    let surrogate = |value: u32| {
        let halves = [value & 0xFFFF, value >> 16];
        halves.iter().any(|half| (0xD800..=0xDFFF).contains(half))
    };
    let mut bytes = [0x41u8, 0x00].repeat(len / 2);
    let mut at = 0;
    while at + 8 <= len {
        let mut count = u32::try_from((len - at - 8) / 2).unwrap();
        while count > 0 && (surrogate(count) || surrogate(4 + 2 * count)) {
            count -= 1;
        }
        bytes[at..at + 4].copy_from_slice(&(4 + 2 * count).to_le_bytes());
        bytes[at + 4..at + 8].copy_from_slice(&count.to_le_bytes());
        at += 8;
    }
    bytes
}

#[test]
fn a_hostile_descriptor_of_overlapping_headers_parses_quickly() {
    let bytes = header_bomb(512 * 1024);
    let started = std::time::Instant::now();
    parse_descriptor(&bytes);
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
}

#[test]
fn a_string_over_the_length_cap_is_not_read() {
    let long = "a".repeat(70_000);
    let parsed = parse_descriptor(&descriptor(&["rmc", &long]));
    assert!(parsed.mods.is_empty());
}

#[test]
fn a_file_that_is_not_a_zip_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("broken.ctr");
    fs::write(&path, b"not a zip at all").unwrap();
    assert!(matches!(saves::read_save(&path), Err(SaveError::NotZip(_))));
}

#[test]
fn a_save_without_a_descriptor_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("partial.ctr");
    write_ctr(&path, &[("Meta.xml", b"<Meta/>")]);
    assert!(matches!(
        saves::read_save(&path),
        Err(SaveError::NoDescriptor)
    ));
}

#[test]
fn an_oversized_descriptor_is_refused_before_it_is_read() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("huge.ctr");
    write_ctr(&path, &[("SaveDescriptor.bin", &vec![0; 5 * 1024 * 1024])]);
    assert!(matches!(
        saves::read_save(&path),
        Err(SaveError::TooLarge(_))
    ));
}

#[test]
fn listing_walks_every_save_folder_and_skips_unreadable_saves_with_a_reason() {
    let root = tempfile::tempdir().unwrap();
    let game = root.path().join("Humankind");
    let save_files = game.join(SAVE_FILES_DIR);
    save_with_mods(&save_files.join("Cloud/one.ctr"), "alpha");
    save_with_mods(&save_files.join("Local/Two.CTR"), "beta");
    fs::write(save_files.join("Cloud/notes.txt"), "not a save").unwrap();
    fs::write(save_files.join("Cloud/broken.ctr"), "garbage").unwrap();
    write_ctr(&save_files.join("Cloud/empty.ctr"), &[("AI.bin", b"x")]);

    let listing = saves::list_saves(&game).unwrap();

    let found: Vec<&str> = listing
        .saves
        .iter()
        .map(|save| save.name.as_str())
        .collect();
    assert_eq!(found, ["one", "Two"]);
    assert_eq!(listing.skipped.len(), 2);
    assert!(listing.skipped.iter().all(|skip| !skip.reason.is_empty()));
    assert!(
        listing
            .skipped
            .iter()
            .any(|skip| skip.reason.contains("SaveDescriptor.bin"))
    );
}

#[test]
fn listing_without_a_save_folder_is_empty() {
    let root = tempfile::tempdir().unwrap();
    let listing = saves::list_saves(root.path()).unwrap();
    assert!(listing.saves.is_empty() && listing.skipped.is_empty());
}

#[test]
fn each_save_mod_is_exact_another_version_or_missing() {
    let list = format!("eventful.{GUID}.0.7\\deepsea reloaded.{GUID}.1.9\\gone.{GUID}.1.0\\loose");
    let parsed = parse_descriptor(&descriptor(&["rmc", &list]));
    let mods = [
        installed("eventful", "0.7"),
        installed("deepsea reloaded", "3.3"),
        installed("loose", "4.0"),
    ];

    let statuses: Vec<ModStatus> = saves::check_mods(&parsed.mods, &mods)
        .into_iter()
        .map(|check| check.status)
        .collect();

    assert_eq!(
        statuses,
        [
            ModStatus::Exact,
            ModStatus::OtherVersion {
                installed: "3.3".into()
            },
            ModStatus::Missing,
            ModStatus::Exact,
        ]
    );
}

#[test]
fn a_same_named_mod_with_another_guid_is_not_the_saves_mod() {
    let wanted = "Sigma Pack.402b91b9e5724da3851e2b6b727b100a.2.5";
    let parsed = parse_descriptor(&descriptor(&["rmc", wanted]));
    // Folder names say nothing about the guid; the module file's name does.
    let impostor = Mod {
        folder: "Atelier/Sigma".into(),
        module_guid: Some("00000000000000000000000000000001".into()),
        ..installed("sigma pack", "2.5")
    };
    let same = Mod {
        folder: "Atelier/Sigma".into(),
        module_guid: Some("402b91b9e5724da3851e2b6b727b100a".into()),
        ..installed("sigma pack", "2.5")
    };

    assert_eq!(
        saves::check_mods(&parsed.mods, &[impostor])[0].status,
        ModStatus::Missing
    );
    assert_eq!(
        saves::check_mods(&parsed.mods, &[same])[0].status,
        ModStatus::Exact
    );
}

#[test]
fn an_unpinned_mod_that_is_not_installed_is_missing() {
    let parsed = parse_descriptor(&descriptor(&["rmc", "loose"]));
    let checks = saves::check_mods(&parsed.mods, &[]);
    assert_eq!(checks[0].status, ModStatus::Missing);
}

#[test]
fn the_ui_contract_is_camel_case() {
    let save = saves::read_save(&fixture()).unwrap();
    let json = serde_json::to_value(&save).unwrap();
    assert_eq!(json["worldSize"], "Huge");
    assert_eq!(json["empireCount"], 10);
    assert_eq!(json["mods"][3]["name"], "eventful");
    let check = saves::check_mods(
        &save.descriptor.mods[12..13],
        &[installed("deepsea reloaded", "3.3")],
    );
    let json = serde_json::to_value(&check[0]).unwrap();
    assert_eq!(json["status"]["kind"], "otherVersion");
    assert_eq!(json["status"]["installed"], "3.3");
    assert_eq!(json["version"], "1.9");
}
