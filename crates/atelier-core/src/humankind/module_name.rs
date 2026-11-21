//! Module names: `<Name>.<guid32hex>.<major>.<minor>` on disk, the lowercased `<Name>` in Registry.xml.

use std::fmt;

const GUID_LENGTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleName {
    pub name: String,
    pub guid: String,
    pub major: i16,
    pub minor: i16,
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
            guid: guid.to_string(),
            major: parse_version_part(major)?,
            minor: parse_version_part(minor)?,
        })
    }

    /// `Runtime.CheckRuntimeModuleName` lowercases every letter before the module is registered.
    pub fn short_name(&self) -> String {
        self.name.to_lowercase()
    }

    pub fn version(&self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

impl fmt::Display for ModuleName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}.{}.{}",
            self.name, self.guid, self.major, self.minor
        )
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
