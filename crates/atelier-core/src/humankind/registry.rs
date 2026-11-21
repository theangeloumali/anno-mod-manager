//! The ordered module list in Registry.xml, `<RuntimeModuleConfiguration>a/b/c</…>`.

use std::io::ErrorKind;
use std::ops::Range;
use std::path::Path;

use super::load_order;
use crate::activation::Snapshots;
use crate::error::io_at;
use crate::snapshots::write_with_snapshot;
use crate::text::{decode, read_capped};
use crate::{Error, Result};

/// The base game; the game loads it first and falls back to it alone when the list is empty.
pub const BASE_MODULE: &str = "project mercury";
const ELEMENT_START: &str = "<RuntimeModuleConfiguration";
const CLOSE_TAG: &str = "</RuntimeModuleConfiguration>";
const END_TAG: &str = "</Registry>";
const SEPARATOR: &str = "/";

/// Where the value sits: between the tags, or a self-closing `<RuntimeModuleConfiguration />`.
enum Value {
    Between(Range<usize>),
    SelfClosing(Range<usize>),
}

/// A missing file or empty value reads as the base game alone, as the game treats it.
pub fn read_modules(path: &Path) -> Result<Vec<String>> {
    let text = read_capped(path)?
        .map(|bytes| decode(&bytes))
        .unwrap_or_default();
    let modules: Vec<String> = match find_value(&text) {
        Some(Value::Between(range)) => text[range]
            .split(SEPARATOR)
            .filter(|entry| !entry.is_empty())
            .map(unescape)
            .collect(),
        Some(Value::SelfClosing(_)) | None => Vec::new(),
    };
    Ok(if modules.is_empty() {
        vec![BASE_MODULE.to_string()]
    } else {
        modules
    })
}

/// Rewrites only the value, keeping the BOM, line endings and every other byte.
pub fn write_modules(path: &Path, modules: &[String], snapshots: &Snapshots) -> Result<()> {
    let bytes = read_capped(path)?.ok_or_else(|| io_at(path)(ErrorKind::NotFound.into()))?;
    let invalid = || Error::InvalidRegistry(path.to_path_buf());
    let text = String::from_utf8(bytes).map_err(|_| invalid())?;
    let value = escape(&load_order::normalize(modules).join(SEPARATOR));
    let element = format!("{ELEMENT_START}>{value}{CLOSE_TAG}");
    let updated = match find_value(&text) {
        Some(Value::Between(range)) => splice(&text, range, &value),
        Some(Value::SelfClosing(range)) => splice(&text, range, &element),
        None => insert_element(&text, &element).ok_or_else(invalid)?,
    };
    write_with_snapshot(path, &updated, snapshots)
}

fn find_value(text: &str) -> Option<Value> {
    let start = text.find(ELEMENT_START)?;
    let tag_end = start + text[start..].find('>')?;
    if text[..tag_end].ends_with('/') {
        return Some(Value::SelfClosing(start..tag_end + 1));
    }
    let value_start = tag_end + 1;
    let value_end = value_start + text[value_start..].find(CLOSE_TAG)?;
    Some(Value::Between(value_start..value_end))
}

fn splice(text: &str, range: Range<usize>, replacement: &str) -> String {
    format!(
        "{}{replacement}{}",
        &text[..range.start],
        &text[range.end..]
    )
}

/// Adds the game's own layout just before `</Registry>`; `None` when that tag is missing.
fn insert_element(text: &str, element: &str) -> Option<String> {
    let end = text.rfind(END_TAG)?;
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let block =
        format!("  <RuntimeModule>{newline}    {element}{newline}  </RuntimeModule>{newline}");
    Some(splice(text, end..end, &block))
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// `&amp;` goes last so `&amp;lt;` stays the literal text `&lt;`.
fn unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}
