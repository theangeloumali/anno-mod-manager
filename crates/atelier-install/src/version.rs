//! Semver-ish version ordering, falling back to plain text when a version isn't numeric.

use std::cmp::Ordering;

/// `1.10.0 > 1.9.5`, `2.0 == 2.0.0`, `1.0-beta < 1.0`; non-numeric versions compare as text.
pub(crate) fn compare_versions(left: &str, right: &str) -> Ordering {
    match (parse(left), parse(right)) {
        (Some(left), Some(right)) => compare_parsed(&left, &right),
        _ => left.trim().cmp(right.trim()),
    }
}

struct ParsedVersion<'a> {
    numbers: Vec<u64>,
    pre_release: Option<&'a str>,
}

fn parse(raw: &str) -> Option<ParsedVersion<'_>> {
    let trimmed = raw.trim();
    let trimmed = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);
    // Build metadata after `+` never affects precedence.
    let without_build = trimmed.split('+').next().unwrap_or(trimmed);
    let (core, pre_release) = match without_build.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (without_build, None),
    };
    let numbers = core
        .split('.')
        .map(|part| part.parse().ok())
        .collect::<Option<Vec<u64>>>()?;
    Some(ParsedVersion {
        numbers,
        pre_release,
    })
}

fn compare_parsed(left: &ParsedVersion, right: &ParsedVersion) -> Ordering {
    let width = left.numbers.len().max(right.numbers.len());
    let segment = |numbers: &[u64], index: usize| numbers.get(index).copied().unwrap_or(0);
    (0..width)
        .map(|index| segment(&left.numbers, index).cmp(&segment(&right.numbers, index)))
        .find(|ordering| ordering.is_ne())
        .unwrap_or_else(|| match (left.pre_release, right.pre_release) {
            (None, None) => Ordering::Equal,
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (Some(left), Some(right)) => left.cmp(right),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_segments_compare_as_numbers() {
        assert_eq!(compare_versions("1.10.0", "1.9.5"), Ordering::Greater);
        assert_eq!(compare_versions("2.0", "2.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("v1.2", "1.3"), Ordering::Less);
    }

    #[test]
    fn a_pre_release_ranks_below_its_release() {
        assert_eq!(compare_versions("1.0-beta", "1.0"), Ordering::Less);
        assert_eq!(compare_versions("1.0+build7", "1.0"), Ordering::Equal);
    }

    #[test]
    fn non_numeric_versions_fall_back_to_text() {
        assert_eq!(compare_versions("beta", "alpha"), Ordering::Greater);
    }
}
