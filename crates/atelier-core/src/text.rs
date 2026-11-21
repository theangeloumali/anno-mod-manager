//! Byte decoding and JSONC clean-up shared by modinfo and the options file.

use std::fs;
use std::io::{ErrorKind, Read};
use std::path::Path;

use encoding_rs::{BIG5, Encoding, GB18030, WINDOWS_1252};

use crate::error::io_at;
use crate::{Error, Result};

/// Far above any Registry.xml or history file the game and plugin write.
const MAX_READ_BYTES: u64 = 8 * 1024 * 1024;

/// The file's bytes, `Ok(None)` when it does not exist, `TooLarge` past the cap.
pub(crate) fn read_capped(path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_at(path)(error)),
    };
    let mut bytes = Vec::new();
    // Reads one byte past the cap, so a file that grows after a size check is still caught.
    file.take(MAX_READ_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(io_at(path))?;
    if bytes.len() as u64 > MAX_READ_BYTES {
        return Err(Error::TooLarge(path.to_path_buf()));
    }
    Ok(Some(bytes))
}

/// Reads and decodes a text file; `Ok(None)` when it does not exist.
pub(crate) fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(decode(&bytes))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_at(path)(error)),
    }
}

/// Same fallback chain as the reference app: UTF-8 (BOM-aware), GB18030, Big5, then cp1252.
pub(crate) fn decode(bytes: &[u8]) -> String {
    if let Some((encoding, bom_length)) = Encoding::for_bom(bytes) {
        let (text, _) = encoding.decode_without_bom_handling(&bytes[bom_length..]);
        return text.into_owned();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_owned();
    }
    [GB18030, BIG5]
        .into_iter()
        .find_map(|encoding| {
            encoding
                .decode_without_bom_handling_and_without_replacement(bytes)
                .map(|text| text.into_owned())
        })
        .unwrap_or_else(|| {
            WINDOWS_1252
                .decode_without_bom_handling(bytes)
                .0
                .into_owned()
        })
}

/// Removes `//` and `/* */` comments and trailing commas, leaving string contents alone.
pub(crate) fn strip_jsonc(source: &str) -> String {
    let without_comments = strip_comments(source);
    strip_trailing_commas(&without_comments)
}

fn strip_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    while let Some(current) = chars.next() {
        if in_string {
            output.push(current);
            match current {
                '\\' => output.extend(chars.next()),
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match (current, chars.peek()) {
            ('"', _) => {
                in_string = true;
                output.push(current);
            }
            ('/', Some('/')) => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        output.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut previous = '\0';
                for skipped in chars.by_ref() {
                    if previous == '*' && skipped == '/' {
                        break;
                    }
                    previous = skipped;
                }
                output.push(' ');
            }
            _ => output.push(current),
        }
    }
    output
}

fn strip_trailing_commas(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut output = String::with_capacity(source.len());
    let mut in_string = false;
    let mut index = 0;
    while index < chars.len() {
        let current = chars[index];
        if in_string {
            output.push(current);
            if current == '\\' {
                if let Some(&escaped) = chars.get(index + 1) {
                    output.push(escaped);
                    index += 1;
                }
            } else if current == '"' {
                in_string = false;
            }
        } else if current == '"' {
            in_string = true;
            output.push(current);
        } else if current != ',' || !closes_next(&chars[index + 1..]) {
            output.push(current);
        }
        index += 1;
    }
    output
}

fn closes_next(rest: &[char]) -> bool {
    matches!(
        rest.iter().find(|c| !c.is_whitespace()),
        Some('}') | Some(']')
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_comment_markers_inside_strings() {
        let cleaned = strip_jsonc(
            r#"{"a": "x // y /* z */", /* gone */ "b": [1, 2,], // end
        }"#,
        );
        let value: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(value["a"], "x // y /* z */");
        assert_eq!(value["b"], serde_json::json!([1, 2]));
    }

    #[test]
    fn keeps_escaped_quotes_inside_strings() {
        let cleaned = strip_jsonc(r#"{"a": "say \"hi\", // ok",}"#);
        let value: serde_json::Value = serde_json::from_str(&cleaned).unwrap();
        assert_eq!(value["a"], "say \"hi\", // ok");
    }

    #[test]
    fn falls_back_to_cp1252() {
        assert_eq!(decode(&[0x43, 0x61, 0x66, 0xE9, 0x20]), "Café ");
    }
}
