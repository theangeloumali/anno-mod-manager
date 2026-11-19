//! The line format shared by `profiles/<name>.txt` (117) and the legacy `active-profile.txt`.
//! `##` is a comment, a single `#` disables the entry; unknown lines survive a rewrite.

use crate::normalize_mod_id;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Line {
    Verbatim(String),
    EnableNewMods(bool),
    Mod {
        id: String,
        enabled: bool,
        note: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProfileFile {
    lines: Vec<Line>,
}

impl ProfileFile {
    pub(crate) fn parse(text: &str) -> Self {
        Self {
            lines: text.lines().map(parse_line).collect(),
        }
    }

    pub(crate) fn new(enable_new_mods: bool) -> Self {
        Self {
            lines: vec![Line::EnableNewMods(enable_new_mods)],
        }
    }

    pub(crate) fn render(&self) -> String {
        let mut text: String = self
            .lines
            .iter()
            .map(render_line)
            .collect::<Vec<_>>()
            .join("\n");
        text.push('\n');
        text
    }

    /// Absent `EnableNewMods` counts as on, matching a missing profile file.
    pub(crate) fn enable_new_mods(&self) -> bool {
        self.lines
            .iter()
            .find_map(|line| match line {
                Line::EnableNewMods(enabled) => Some(*enabled),
                _ => None,
            })
            .unwrap_or(true)
    }

    pub(crate) fn is_active(&self, mod_id: &str) -> bool {
        self.entries()
            .find(|(id, _)| *id == mod_id)
            .map_or_else(|| self.enable_new_mods(), |(_, enabled)| enabled)
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (&str, bool)> {
        self.lines.iter().filter_map(|line| match line {
            Line::Mod { id, enabled, .. } => Some((id.as_str(), *enabled)),
            _ => None,
        })
    }

    pub(crate) fn set(&mut self, mod_id: &str, active: bool) {
        for line in &mut self.lines {
            if let Line::Mod { id, enabled, .. } = line
                && id == mod_id
            {
                *enabled = active;
                return;
            }
        }
        self.lines.push(Line::Mod {
            id: mod_id.to_string(),
            enabled: active,
            note: String::new(),
        });
    }
}

fn parse_line(raw: &str) -> Line {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with("##") {
        return Line::Verbatim(raw.to_string());
    }
    let (enabled, body) = match trimmed.strip_prefix('#') {
        Some(rest) => (false, rest.trim()),
        None => (true, trimmed),
    };
    if body.eq_ignore_ascii_case("EnableNewMods") {
        return Line::EnableNewMods(enabled);
    }
    if body.is_empty() || body.starts_with("DisplayName") {
        return Line::Verbatim(raw.to_string());
    }
    // A trailing ` # note` is the game's stale marker ("not installed").
    let (id, note) = body.split_at(body.find(" #").unwrap_or(body.len()));
    Line::Mod {
        id: normalize_mod_id(id),
        enabled,
        note: note.to_string(),
    }
}

fn render_line(line: &Line) -> String {
    match line {
        Line::Verbatim(raw) => raw.clone(),
        Line::EnableNewMods(true) => "EnableNewMods".into(),
        Line::EnableNewMods(false) => "# EnableNewMods".into(),
        Line::Mod { id, enabled, note } => {
            let marker = if *enabled { "" } else { "# " };
            format!("{marker}{id}{note}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_comments_notes_and_display_names() {
        let text = "## header\nDisplayName Mine\n# EnableNewMods\nkeep\n# off # not installed\n";
        let mut file = ProfileFile::parse(text);
        assert_eq!(file.render(), text);
        file.set("keep", false);
        file.set("added", true);
        assert_eq!(
            file.render(),
            "## header\nDisplayName Mine\n# EnableNewMods\n# keep\n# off # not installed\nadded\n"
        );
    }
}
