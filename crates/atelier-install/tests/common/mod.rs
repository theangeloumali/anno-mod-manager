#![allow(dead_code)] // each test binary uses a different subset

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use atelier_core::scan::{self, ModsFolder};
use atelier_core::{GameId, Mod, ModLocation};
use atelier_install::{EngineConfig, InstallEngine, Limits};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

pub enum Entry<'a> {
    File(&'a str, &'a [u8]),
    Dir(&'a str),
    Symlink(&'a str, &'a str),
}

pub struct Sandbox {
    pub root: TempDir,
    pub engine: InstallEngine,
}

impl Sandbox {
    pub fn new() -> Self {
        Self::with(Some(30), Limits::default())
    }

    pub fn with(retention_days: Option<u32>, limits: Limits) -> Self {
        let root = tempfile::tempdir().unwrap();
        let config = EngineConfig {
            staging_dir: root.path().join("staging"),
            backup_dir: root.path().join("backups"),
            retention_days,
        };
        let engine = InstallEngine::with_limits(config, limits);
        Self { root, engine }
    }

    pub fn mods(&self) -> PathBuf {
        self.root.path().join("mods")
    }

    pub fn staging(&self) -> PathBuf {
        self.root.path().join("staging")
    }

    pub fn backups(&self) -> PathBuf {
        self.root.path().join("backups")
    }

    /// A path for a dropped source, outside every engine folder.
    pub fn drop_path(&self, name: &str) -> PathBuf {
        let dir = self.root.path().join("downloads");
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    pub fn installed(&self, game: GameId) -> Vec<Mod> {
        let folder = ModsFolder {
            path: self.mods(),
            location: ModLocation::Documents,
        };
        scan::scan(game, &[folder]).unwrap().mods
    }
}

pub fn modinfo(id: &str, version: &str, anno: Option<u8>) -> String {
    let anno_field = anno.map(|n| format!("\"Anno\": {n},")).unwrap_or_default();
    format!(
        "{{ {anno_field} \"ModID\": \"{id}\", \"Version\": \"{version}\", \
         \"ModName\": {{ \"English\": \"{id} name\" }}, \"Category\": \"Test\" }}"
    )
}

pub fn write_zip(path: &Path, entries: &[Entry]) {
    let mut writer = zip::ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default();
    for entry in entries {
        match entry {
            Entry::File(name, bytes) => {
                writer.start_file(*name, options).unwrap();
                writer.write_all(bytes).unwrap();
            }
            Entry::Dir(name) => writer.add_directory(*name, options).unwrap(),
            Entry::Symlink(name, target) => writer.add_symlink(*name, *target, options).unwrap(),
        }
    }
    writer.finish().unwrap();
}

pub fn zip_bytes(entries: &[Entry]) -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("inner.zip");
    write_zip(&path, entries);
    fs::read(path).unwrap()
}

pub fn write_tree(dir: &Path, files: &[(&str, &str)]) {
    for (relative, text) in files {
        let path = dir.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

pub fn read_version(mod_dir: &Path) -> String {
    atelier_core::modinfo::read_dir(mod_dir)
        .unwrap()
        .expect("modinfo")
        .version
}

/// Every file under `dir`, relative and `/`-separated.
pub fn files_under(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    collect(dir, dir, &mut found);
    found.sort();
    found
}

fn collect(base: &Path, dir: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(base, &path, found);
        } else {
            let relative = path.strip_prefix(base).unwrap();
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}
