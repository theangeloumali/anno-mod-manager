//! Dependency, conflict, duplicate and cycle problems among the active mods.

use std::collections::{HashMap, HashSet};

use crate::{Issue, IssueKind, IssueLevel, LoadOrder, Mod};

pub fn check(mods: &[Mod], order: &LoadOrder) -> Vec<Issue> {
    let mut installed: HashMap<&str, bool> = HashMap::new();
    for target in mods {
        *installed.entry(target.id.as_str()).or_default() |= target.active;
    }
    let mut found = Vec::new();
    let mut reported_pairs = HashSet::new();
    let mut checked = HashSet::new();
    for target in mods
        .iter()
        .filter(|m| m.active && checked.insert(m.id.as_str()))
    {
        dependency_issues(target, &installed, &mut found);
        for other in &target.dependencies.incompatible {
            let pair = ordered_pair(&target.id, other);
            if installed.get(other.as_str()) == Some(&true) && reported_pairs.insert(pair) {
                found.push(issue(
                    target,
                    IssueLevel::Error,
                    IssueKind::Incompatible,
                    Some(other),
                    format!("{} cannot run alongside {other}", target.name),
                ));
            }
        }
        for deprecated in &target.dependencies.deprecate {
            if let Some(old) = mods.iter().find(|m| m.active && m.id == *deprecated) {
                found.push(issue(
                    old,
                    IssueLevel::Warning,
                    IssueKind::Deprecated,
                    Some(&target.id),
                    format!("{} is replaced by {}; disable it", old.name, target.name),
                ));
            }
        }
    }
    duplicate_issues(mods, &mut found);
    for cycle in &order.cycles {
        let first = cycle.first();
        if let Some(target) = mods.iter().find(|m| Some(&m.id) == first) {
            found.push(issue(
                target,
                IssueLevel::Warning,
                IssueKind::LoadOrderCycle,
                cycle.get(1),
                format!("LoadAfter cycle: {}", cycle.join(" → ")),
            ));
        }
    }
    found
}

fn dependency_issues(target: &Mod, installed: &HashMap<&str, bool>, found: &mut Vec<Issue>) {
    for required in &target.dependencies.require {
        let (level, kind, message) = match installed.get(required.as_str()) {
            None => (
                IssueLevel::Error,
                IssueKind::MissingDependency,
                format!("{} needs {required}, which is not installed", target.name),
            ),
            Some(false) => (
                IssueLevel::Warning,
                IssueKind::InactiveDependency,
                format!("{} needs {required}, which is disabled", target.name),
            ),
            Some(true) => continue,
        };
        found.push(issue(target, level, kind, Some(required), message));
    }
}

/// One warning per id found in more than one folder; the loader keeps only one copy.
fn duplicate_issues(mods: &[Mod], found: &mut Vec<Issue>) {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for target in mods {
        *counts.entry(target.id.as_str()).or_default() += 1;
    }
    let mut reported = HashSet::new();
    for target in mods {
        if counts[target.id.as_str()] > 1 && reported.insert(target.id.as_str()) {
            found.push(issue(
                target,
                IssueLevel::Warning,
                IssueKind::DuplicateId,
                None,
                format!("{} is installed more than once", target.id),
            ));
        }
    }
}

fn ordered_pair(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_string(), right.to_string())
    } else {
        (right.to_string(), left.to_string())
    }
}

fn issue(
    target: &Mod,
    level: IssueLevel,
    kind: IssueKind,
    related: Option<&String>,
    message: String,
) -> Issue {
    Issue {
        mod_id: target.id.clone(),
        level,
        kind,
        related_mod_id: related.cloned(),
        message,
    }
}
