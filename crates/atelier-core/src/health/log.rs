//! Parses `mod-loader.log`: `[time] [level] [mod-id: ]message[ (file:line)]`,
//! where lines not starting with `[` continue the previous message.

use crate::{LogEntry, LogLevel};

const ENABLED_PREFIX: &str = "Enabled mod: ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnabledMod {
    pub id: String,
    pub version: String,
    pub path: String,
}

pub fn parse(text: &str) -> Vec<LogEntry> {
    let mut entries: Vec<LogEntry> = Vec::new();
    for line in text.lines() {
        match parse_line(line) {
            Some(entry) => entries.push(entry),
            None => {
                if let Some(last) = entries.last_mut()
                    && !line.trim().is_empty()
                {
                    last.message.push('\n');
                    last.message.push_str(line);
                }
            }
        }
    }
    entries
}

/// The loader prints one `Enabled mod: <path>  (<id>@<version>)` line per mod, in load order.
pub fn enabled_mods(entries: &[LogEntry]) -> Vec<EnabledMod> {
    entries
        .iter()
        .filter_map(|entry| {
            let rest = entry.message.strip_prefix(ENABLED_PREFIX)?;
            // Paths contain parentheses too, so split on the last ` (`.
            let split = rest.rfind(" (")?;
            let tag = rest[split + 2..].strip_suffix(')')?;
            let (id, version) = tag.rsplit_once('@')?;
            Some(EnabledMod {
                id: id.to_string(),
                version: version.to_string(),
                path: rest[..split].trim().to_string(),
            })
        })
        .collect()
}

fn parse_line(line: &str) -> Option<LogEntry> {
    let (time, rest) = bracketed(line)?;
    let (level, rest) = bracketed(rest.trim_start())?;
    let body = rest.strip_prefix(' ').unwrap_or(rest);
    let (body, source) = split_source(body);
    let (mod_id, message) = split_mod_id(body);
    Some(LogEntry {
        time: time.to_string(),
        level: parse_level(level),
        mod_id,
        message: message.to_string(),
        source,
    })
}

fn bracketed(text: &str) -> Option<(&str, &str)> {
    let inner = text.strip_prefix('[')?;
    let end = inner.find(']')?;
    Some((&inner[..end], &inner[end + 1..]))
}

fn parse_level(raw: &str) -> LogLevel {
    match raw.to_ascii_lowercase().as_str() {
        "warning" | "warn" => LogLevel::Warning,
        "error" | "err" => LogLevel::Error,
        _ => LogLevel::Info,
    }
}

/// A trailing `(path:line)` names the XML that triggered the message.
fn split_source(body: &str) -> (&str, Option<String>) {
    let trimmed = body.trim_end();
    let Some(inner) = trimmed.strip_suffix(')') else {
        return (body, None);
    };
    let Some(open) = inner.rfind(" (") else {
        return (body, None);
    };
    let source = &inner[open + 2..];
    let is_location = source.rsplit_once(':').is_some_and(|(file, line)| {
        !file.is_empty() && !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit())
    });
    if is_location {
        (&inner[..open], Some(source.to_string()))
    } else {
        (body, None)
    }
}

fn split_mod_id(body: &str) -> (Option<String>, &str) {
    let Some((prefix, message)) = body.split_once(": ") else {
        return (None, body);
    };
    let mut chars = prefix.chars();
    let starts_well = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_ok =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    if starts_well && rest_ok {
        (Some(prefix.to_string()), message)
    } else {
        (None, body)
    }
}
