//! Save files: each `.ctr` is a zip whose `SaveDescriptor.bin` names the build, mods and options.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::module_name::{ModuleName, short_name};
use super::registry::BASE_MODULE;
use crate::error::io_at;
use crate::{Mod, Result};

pub const SAVE_FILES_DIR: &str = "Save Files";
const SAVE_EXTENSION: &str = "ctr";
const DESCRIPTOR_ENTRY: &str = "SaveDescriptor.bin";
/// A real descriptor is about 45 KB; the cap stops a hostile zip from filling memory.
const MAX_DESCRIPTOR_BYTES: u64 = 4 * 1024 * 1024;
/// Far above any real key or mod list; a longer run is not a descriptor string.
const MAX_STRING_UNITS: u32 = 64 * 1024;
const BUILD_KEY: &str = "ver";
const MODS_KEY: &str = "rmc";
const MOD_SEPARATOR: char = '\\';
const OPTION_PREFIX: &str = "GameOption_";
const WORLD_SIZE_OPTION: &str = "WorldSize";
const EMPIRE_COUNT_OPTION: &str = "SlotCount";

/// One module a save was played with; `version` is `None` when the save didn't pin one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMod {
    /// The short, lowercased name Registry.xml uses.
    pub name: String,
    pub version: Option<String>,
    /// Two mods can share a name; the guid tells them apart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameOption {
    /// Without the `GameOption_` prefix, e.g. `WorldSize`.
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    pub build: Option<String>,
    /// In load order, the base game left out.
    pub mods: Vec<SaveMod>,
    /// False when the descriptor names no mod list at all: zero mods then means "unknown",
    /// not "vanilla", and the save must never set the active list.
    pub mod_list_found: bool,
    pub options: Vec<GameOption>,
    pub world_size: Option<String>,
    pub empire_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Save {
    pub path: PathBuf,
    pub name: String,
    #[serde(flatten)]
    pub descriptor: Descriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedSave {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveList {
    pub saves: Vec<Save>,
    pub skipped: Vec<SkippedSave>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModStatus {
    Exact,
    OtherVersion { installed: String },
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModCheck {
    #[serde(flatten)]
    pub module: SaveMod,
    pub status: ModStatus,
}

/// Why a `.ctr` can't be read; saves are untrusted input, so each is a reason, never a panic.
#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("can't be read: {0}")]
    Io(#[from] io::Error),
    #[error("isn't a readable save archive ({0})")]
    NotZip(String),
    #[error("has no {DESCRIPTOR_ENTRY}")]
    NoDescriptor,
    #[error("has a {DESCRIPTOR_ENTRY} over {0} bytes")]
    TooLarge(u64),
}

/// Every `.ctr` under `<game_dir>/Save Files`, at any depth (the game keeps `Cloud/` and others).
pub fn list_saves(game_dir: &Path) -> Result<SaveList> {
    let mut list = SaveList::default();
    visit(&game_dir.join(SAVE_FILES_DIR), &mut list)?;
    Ok(list)
}

fn visit(dir: &Path, list: &mut SaveList) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(io_at(dir))? {
        let entry = entry.map_err(io_at(dir))?;
        // file_type() does not follow symlinks, so a linked folder is never walked.
        let kind = entry.file_type().map_err(io_at(&entry.path()))?;
        entries.push((entry.path(), kind));
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));
    for (path, kind) in entries {
        if kind.is_dir() {
            visit(&path, list)?;
        } else if kind.is_file() && is_save(&path) {
            match read_save(&path) {
                Ok(save) => list.saves.push(save),
                Err(error) => list.skipped.push(SkippedSave {
                    path,
                    reason: error.to_string(),
                }),
            }
        }
    }
    Ok(())
}

fn is_save(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(SAVE_EXTENSION))
}

pub fn read_save(path: &Path) -> std::result::Result<Save, SaveError> {
    let not_zip = |error: zip::result::ZipError| SaveError::NotZip(error.to_string());
    let mut archive = zip::ZipArchive::new(File::open(path)?).map_err(not_zip)?;
    let entry = match archive.by_name(DESCRIPTOR_ENTRY) {
        Ok(entry) => entry,
        Err(zip::result::ZipError::FileNotFound) => return Err(SaveError::NoDescriptor),
        Err(error) => return Err(not_zip(error)),
    };
    if entry.size() > MAX_DESCRIPTOR_BYTES {
        return Err(SaveError::TooLarge(MAX_DESCRIPTOR_BYTES));
    }
    // The declared size can lie, so the read itself is capped too.
    let mut bytes = Vec::new();
    entry
        .take(MAX_DESCRIPTOR_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| SaveError::NotZip(error.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_DESCRIPTOR_BYTES {
        return Err(SaveError::TooLarge(MAX_DESCRIPTOR_BYTES));
    }
    Ok(Save {
        path: path.to_path_buf(),
        name: path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
        descriptor: parse_descriptor(&bytes),
    })
}

/// Unknown bytes are skipped, so a damaged descriptor yields what it still holds.
pub fn parse_descriptor(bytes: &[u8]) -> Descriptor {
    let strings = strings(bytes);
    let options = options(&strings);
    let option = |key: &str| {
        options
            .iter()
            .find(|option| option.key == key)
            .map(|option| option.value.clone())
            .filter(|value| !value.is_empty())
    };
    let mod_list = value_of(&strings, MODS_KEY);
    Descriptor {
        build: value_of(&strings, BUILD_KEY).filter(|build| !build.is_empty()),
        mod_list_found: mod_list.is_some(),
        mods: mod_list.map(|list| mods(&list)).unwrap_or_default(),
        world_size: option(WORLD_SIZE_OPTION),
        empire_count: option(EMPIRE_COUNT_OPTION).and_then(|count| count.parse().ok()),
        options,
    }
}

/// Where the installed copies stand against the version the save was played with.
/// A copy whose folder names another guid is a different mod that happens to share the name.
fn same_guid(module: &SaveMod, item: &Mod) -> bool {
    let installed = item
        .folder
        .rsplit('/')
        .next()
        .and_then(ModuleName::parse)
        .map(|parsed| parsed.guid);
    match (&module.guid, installed) {
        (Some(wanted), Some(found)) => wanted.eq_ignore_ascii_case(&found),
        _ => true,
    }
}

pub fn check_mods(save_mods: &[SaveMod], installed: &[Mod]) -> Vec<ModCheck> {
    save_mods
        .iter()
        .map(|module| {
            let copies: Vec<&Mod> = installed
                .iter()
                .filter(|item| item.id == module.name && same_guid(module, item))
                .collect();
            let status = match (&module.version, copies.first()) {
                (_, None) => ModStatus::Missing,
                (None, Some(_)) => ModStatus::Exact,
                (Some(wanted), Some(first)) => {
                    if copies.iter().any(|item| &item.version == wanted) {
                        ModStatus::Exact
                    } else {
                        ModStatus::OtherVersion {
                            installed: first.version.clone(),
                        }
                    }
                }
            };
            ModCheck {
                module: module.clone(),
                status,
            }
        })
        .collect()
}

struct Found {
    text: String,
    start: usize,
    end: usize,
}

/// Every string in the stream. An empty one counts only right after another string, since
/// `04 00 00 00 00 00 00 00` is common in the binary fields between them.
fn strings(bytes: &[u8]) -> Vec<Found> {
    let mut found: Vec<Found> = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let follows = found.last().is_some_and(|last| last.end == at);
        match string_at(bytes, at).filter(|(text, _)| follows || !text.is_empty()) {
            Some((text, end)) => {
                found.push(Found {
                    text,
                    start: at,
                    end,
                });
                at = end;
            }
            None => at += 1,
        }
    }
    found
}

/// `u32 4 + 2n`, `u32 n`, then n UTF-16LE units with no control characters.
fn string_at(bytes: &[u8], at: usize) -> Option<(String, usize)> {
    let size = u32_at(bytes, at)?;
    let count = u32_at(bytes, at.checked_add(4)?)?;
    if u64::from(size) != 4 + 2 * u64::from(count) || count > MAX_STRING_UNITS {
        return None;
    }
    let start = at.checked_add(8)?;
    let end = start.checked_add(usize::try_from(count).ok()?.checked_mul(2)?)?;
    let units = bytes
        .get(start..end)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair));
    // Stops at the first bad unit, so overlapping fake headers can't make the scan quadratic.
    let mut text = String::new();
    for decoded in char::decode_utf16(units) {
        text.push(decoded.ok().filter(|character| !character.is_control())?);
    }
    Some((text, end))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    let word: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(word))
}

/// The string straight after the first `key`; a value always follows its key with no gap.
fn value_of(strings: &[Found], key: &str) -> Option<String> {
    strings
        .windows(2)
        .find(|pair| pair[0].text == key && pair[1].start == pair[0].end)
        .map(|pair| pair[1].text.clone())
}

fn options(strings: &[Found]) -> Vec<GameOption> {
    let mut options: Vec<GameOption> = Vec::new();
    let mut index = 0;
    while let Some(current) = strings.get(index) {
        let value = strings
            .get(index + 1)
            .filter(|next| next.start == current.end);
        match (current.text.strip_prefix(OPTION_PREFIX), value) {
            (Some(key), Some(value)) => {
                if !options.iter().any(|option| option.key == key) {
                    options.push(GameOption {
                        key: key.to_string(),
                        value: value.text.clone(),
                    });
                }
                index += 2;
            }
            _ => index += 1,
        }
    }
    options
}

/// `\`-separated long or short names; the base game and repeats are dropped, first place kept.
fn mods(list: &str) -> Vec<SaveMod> {
    let mut mods: Vec<SaveMod> = Vec::new();
    for entry in list.split(MOD_SEPARATOR).filter(|entry| !entry.is_empty()) {
        let parsed = ModuleName::parse(entry);
        let module = SaveMod {
            name: short_name(entry),
            version: parsed.as_ref().map(ModuleName::version),
            guid: parsed.map(|parsed| parsed.guid),
        };
        let listed = mods.iter().any(|other| other.name == module.name);
        if module.name != BASE_MODULE && !listed {
            mods.push(module);
        }
    }
    mods
}
