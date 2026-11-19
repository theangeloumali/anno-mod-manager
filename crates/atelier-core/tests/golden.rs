//! Golden tests over the real Anno 117 mod-loader.log.
mod common;

use std::fs;

use atelier_core::health::log;
use atelier_core::{GameId, LogLevel, load_order, scan};
use common::*;

fn real_log() -> Vec<atelier_core::LogEntry> {
    let text = fs::read_to_string(fixture("real/anno117-mod-loader.log")).unwrap();
    log::parse(&text)
}

#[test]
fn parses_every_line_of_the_real_log() {
    let entries = real_log();
    let count = |level: LogLevel| entries.iter().filter(|e| e.level == level).count();
    assert_eq!(entries.len(), 368);
    assert_eq!(count(LogLevel::Warning), 34);
    assert_eq!(count(LogLevel::Error), 0);
    let sourced: Vec<_> = entries.iter().filter(|e| e.source.is_some()).collect();
    assert_eq!(sourced.len(), 34);
    assert!(sourced.iter().all(|e| e.mod_id.is_some()));
    assert_eq!(
        sourced[0].mod_id.as_deref(),
        Some("aqueduct-roman-celtic-distribution-public-boost-dwojt")
    );
    assert_eq!(
        sourced[0].source.as_deref(),
        Some("data/base/config/export/assets.xml:8")
    );
}

#[test]
fn load_order_matches_the_real_log() {
    let enabled = log::enabled_mods(&real_log());
    assert_eq!(enabled.len(), 97);

    let corpus: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fixture("core/anno117-golden-mods.json")).unwrap(),
    )
    .unwrap();
    let tmp = tempfile::tempdir().unwrap();
    for entry in corpus["mods"].as_array().unwrap() {
        write_mod(
            tmp.path(),
            entry["folder"].as_str().unwrap(),
            &entry["modinfo"].to_string(),
        );
    }
    let scanned = scan::scan(GameId::Anno117, &[documents(tmp.path())]).unwrap();
    assert_eq!(scanned.mods.len(), 97);
    assert!(scanned.skipped.is_empty());

    let order = load_order::compute(&scanned.mods);
    let computed: Vec<_> = order.entries.iter().map(|e| e.mod_id.as_str()).collect();
    let expected: Vec<_> = enabled.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(computed, expected);
    assert!(order.cycles.is_empty());
}
