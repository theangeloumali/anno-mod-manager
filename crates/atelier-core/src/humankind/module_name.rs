//! Module names: `<Name>.<guid32hex>.<major>.<minor>` on disk, the lowercased `<Name>` in Registry.xml.

use std::fmt;

const GUID_LENGTH: usize = 32;
pub const BUNDLE_EXTENSION: &str = ".assetbundle";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleName {
    pub name: String,
    /// `None` for an older module shipped under its plain name.
    pub guid: Option<String>,
    version: Option<(i16, i16)>,
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
            guid: Some(guid.to_string()),
            version: Some((parse_version_part(major)?, parse_version_part(minor)?)),
        })
    }

    /// A module file among `siblings`: a long name, or any name whose lowercased bundle sits
    /// beside it. The game finds modules by bundle and loads one whose name won't split
    /// (`RuntimeManager.EnumerateRuntimeModules`).
    pub fn detect(file: &str, siblings: &[String]) -> Option<Self> {
        if let Some(module) = Self::parse(file) {
            return Some(module);
        }
        let bundle = format!("{file}{BUNDLE_EXTENSION}").to_lowercase();
        let has_bundle = siblings.iter().any(|name| name.to_lowercase() == bundle);
        (has_bundle && !file.is_empty()).then(|| Self {
            name: file.to_string(),
            guid: None,
            version: None,
        })
    }

    /// The bundle's file name, which the game writes lowercased.
    pub fn bundle_name(&self) -> String {
        format!("{self}{BUNDLE_EXTENSION}").to_lowercase()
    }

    /// `Runtime.CheckRuntimeModuleName` lowercases every letter before the module is registered.
    pub fn short_name(&self) -> String {
        self.name.to_lowercase()
    }

    /// Empty for a plain-named module, which carries no version.
    pub fn version(&self) -> String {
        self.version
            .map(|(major, minor)| format!("{major}.{minor}"))
            .unwrap_or_default()
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.guid, self.version) {
            (Some(guid), Some((major, minor))) => {
                write!(formatter, "{}.{guid}.{major}.{minor}", self.name)
            }
            _ => write!(formatter, "{}", self.name),
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
