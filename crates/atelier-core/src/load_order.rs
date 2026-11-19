//! Reproduces the 117 loader's order: a normal then a late (`LoadAfter: ["*"]`) phase;
//! in each, sub-mods then top-level mods sorted by `"[Category] Name"`, with
//! LoadAfter targets pulled in ahead of the mod that names them.

use std::collections::{HashMap, HashSet};

use crate::{LoadOrder, LoadOrderEntry, LoadReason, Mod, Phase};

const LATE_MARKER: &str = "*";

pub fn compute(mods: &[Mod]) -> LoadOrder {
    let mut seen = HashSet::new();
    let active: Vec<&Mod> = mods
        .iter()
        .filter(|m| m.active && seen.insert(m.id.as_str()))
        .collect();
    let mut order = LoadOrder::default();
    for phase in [Phase::Normal, Phase::Late] {
        let members: Vec<&Mod> = active
            .iter()
            .copied()
            .filter(|m| phase_of(m) == phase)
            .collect();
        let mut walker = Walker::new(&members);
        for target in sorted(&members) {
            walker.visit(target);
        }
        order.cycles.append(&mut walker.cycles);
        for (target, after) in walker.emitted {
            let reason = match (phase, after.is_empty()) {
                (Phase::Late, _) => LoadReason::Late,
                (Phase::Normal, false) => LoadReason::LoadAfter,
                (Phase::Normal, true) => LoadReason::Alphabetical,
            };
            order.entries.push(LoadOrderEntry {
                position: order.entries.len() + 1,
                mod_id: target.id.clone(),
                phase,
                reason,
                after,
            });
        }
    }
    order
}

fn phase_of(target: &Mod) -> Phase {
    if target
        .dependencies
        .load_after
        .iter()
        .any(|id| id == LATE_MARKER)
    {
        Phase::Late
    } else {
        Phase::Normal
    }
}

/// Plain byte comparison, so `[` and case sort the way the game's string compare does.
fn sorted<'a>(members: &[&'a Mod]) -> Vec<&'a Mod> {
    let mut ordered = members.to_vec();
    ordered.sort_by(|left, right| {
        let key = |m: &Mod| {
            (
                m.parent_id.is_none(),
                format!("[{}] {}", m.category, m.name),
            )
        };
        key(left)
            .cmp(&key(right))
            .then_with(|| left.id.cmp(&right.id))
    });
    ordered
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    InProgress,
    Done,
}

struct Walker<'a> {
    by_id: HashMap<&'a str, &'a Mod>,
    state: HashMap<&'a str, Visit>,
    stack: Vec<&'a str>,
    emitted: Vec<(&'a Mod, Vec<String>)>,
    cycles: Vec<Vec<String>>,
}

impl<'a> Walker<'a> {
    fn new(members: &[&'a Mod]) -> Self {
        Self {
            by_id: members.iter().map(|m| (m.id.as_str(), *m)).collect(),
            state: HashMap::new(),
            stack: Vec::new(),
            emitted: Vec::new(),
            cycles: Vec::new(),
        }
    }

    /// Depth-first: emit resolvable LoadAfter targets first; a target still in
    /// progress closes a cycle, which is recorded rather than looping.
    fn visit(&mut self, target: &'a Mod) {
        if self.state.contains_key(target.id.as_str()) {
            return;
        }
        self.state.insert(&target.id, Visit::InProgress);
        self.stack.push(&target.id);
        let mut after = Vec::new();
        for dependency in &target.dependencies.load_after {
            let Some(&dependency_mod) = self.by_id.get(dependency.as_str()) else {
                continue;
            };
            if dependency_mod.id == target.id {
                continue;
            }
            match self.state.get(dependency.as_str()) {
                Some(Visit::InProgress) => self.record_cycle(dependency),
                Some(Visit::Done) => after.push(dependency.clone()),
                None => {
                    self.visit(dependency_mod);
                    after.push(dependency.clone());
                }
            }
        }
        self.stack.pop();
        self.state.insert(&target.id, Visit::Done);
        self.emitted.push((target, after));
    }

    fn record_cycle(&mut self, back_to: &str) {
        if let Some(start) = self.stack.iter().position(|id| *id == back_to) {
            let cycle = self.stack[start..]
                .iter()
                .map(|id| id.to_string())
                .collect();
            self.cycles.push(cycle);
        }
    }
}
