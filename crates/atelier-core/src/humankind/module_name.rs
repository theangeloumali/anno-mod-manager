//! Module names: `<Name>.<guid32hex>.<major>.<minor>` on disk, the lowercased `<Name>` in Registry.xml.

use std::fmt;

const GUID_LENGTH: usize = 32;
const BUNDLE_EXTENSION: &str = ".assetbundle";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleName {
    pub name: String,
    /// `None` for an older module shipped under its plain name.
    stamp: Option<Stamp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    guid: String,
    major: i16,
    minor: i16,
}

impl ModuleName {
    /// Mirrors the game's `RuntimeUtils.TrySplitRuntimeModuleName`; versions are `short`s there.
    pub fn parse(long_name: &str) -> Option<Self> {
        let (rest, minor) = long_name.rsplit_once('.')?;
        let (rest, major) = rest.rsplit_once('.')?;
        let (name, guid) = rest.rsplit_once('.')?;
        let is_guid =
            guid.len() == GUID_LENGTH && guid.bytes().all(|byte| byte.is_ascii_hexdigit());
        if name.is_empty() || !is_guid {
            return None;
        }
        Some(Self {
            name: name.to_string(),
            stamp: Some(Stamp {
                guid: guid.to_string(),
                major: parse_version_part(major)?,
                minor: parse_version_part(minor)?,
            }),
        })
    }

    /// A module file among `siblings`: a long name, or any name whose bundle sits beside it.
    /// The game finds modules by bundle and loads one whose name won't split
    /// (`RuntimeManager.EnumerateRuntimeModules`).
    pub fn detect(file: &str, siblings: &[String]) -> Option<Self> {
        Self::parse(file).or_else(|| {
            let plain = Self {
                name: file.to_string(),
                stamp: None,
            };
            plain.bundle_in(siblings).is_some().then_some(plain)
        })
    }

    pub fn guid(&self) -> Option<&str> {
        self.stamp.as_ref().map(|stamp| stamp.guid.as_str())
    }

    /// The bundle's file name, which the game writes lowercased.
    pub fn bundle_name(&self) -> String {
        format!("{self}{BUNDLE_EXTENSION}").to_lowercase()
    }

    /// This module's bundle among `siblings`, matched without case.
    pub fn bundle_in<'s>(&self, siblings: &'s [String]) -> Option<&'s String> {
        let wanted = self.bundle_name();
        siblings.iter().find(|name| name.to_lowercase() == wanted)
    }

    /// `Runtime.CheckRuntimeModuleName` lowercases every letter before the module is registered.
    pub fn short_name(&self) -> String {
        self.name.to_lowercase()
    }

    /// Empty for a plain-named module, which carries no version.
    pub fn version(&self) -> String {
        self.stamp
            .as_ref()
            .map(|stamp| format!("{}.{}", stamp.major, stamp.minor))
            .unwrap_or_default()
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.stamp {
            Some(Stamp { guid, major, minor }) => {
                write!(formatter, "{}.{guid}.{major}.{minor}", self.name)
            }
            None => write!(formatter, "{}", self.name),
        }
    }
}

/// A registry entry may be a short name or a full long name; both resolve to the short name.
pub fn short_name(entry: &str) -> String {
    ModuleName::parse(entry)
        .map(|parsed| parsed.short_name())
        .unwrap_or_else(|| entry.to_lowercase())
}

fn parse_version_part(part: &str) -> Option<i16> {
    let digits_only = !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    digits_only.then(|| part.parse().ok()).flatten()
}
