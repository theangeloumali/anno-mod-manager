// Shared by several test binaries; each uses a subset.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use atelier_core::activation::Snapshots;
use atelier_core::scan::ModsFolder;
use atelier_core::{GameId, GameSetup, Mod, ModDependencies, ModLocation, ModOrigin};

pub fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

pub fn write_mod(root: &Path, folder: &str, modinfo: &str) {
    let dir = root.join(folder);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("modinfo.json"), modinfo).unwrap();
}

pub fn info117(id: &str, extra: &str) -> String {
    format!(
        r#"{{"Anno":8,"ModID":"{id}","Version":"1.0.0","Category":{{"English":"Cat"}},"ModName":{{"English":"{id}"}}{extra}}}"#
    )
}

pub fn info1800(id: &str) -> String {
    format!(r#"{{"ModID":"{id}","Version":"1.0.0","ModName":{{"English":"{id}"}}}}"#)
}

pub fn folder_at(path: &Path, location: ModLocation) -> ModsFolder {
    ModsFolder {
        path: path.to_path_buf(),
        location,
    }
}

pub fn documents(path: &Path) -> ModsFolder {
    folder_at(path, ModLocation::Documents)
}

pub fn snapshots(root: &Path) -> Snapshots {
    Snapshots::new(root.join("snapshots"), 5)
}

pub fn test_mod(id: &str, category: &str, name: &str) -> Mod {
    Mod {
        id: id.into(),
        game: GameId::Anno117,
        folder: format!("[{category}] {name}"),
        name: name.into(),
        category: category.into(),
        version: "1.0.0".into(),
        author: None,
        description: None,
        difficulty: None,
        location: ModLocation::Documents,
        origin: ModOrigin::Local,
        modio_id: None,
        active: true,
        size_bytes: 0,
        setup: GameSetup::default(),
        dependencies: ModDependencies::default(),
        known_issues: Vec::new(),
        has_options: false,
        parent_id: None,
        sub_mod_ids: Vec::new(),
        update: None,
    }
}

pub fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}
