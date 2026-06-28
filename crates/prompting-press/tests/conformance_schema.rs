//! Conformance corpus — Rust schema round-trip runner (spec 006, T011; US2).
//!
//! Feeds every `conformance/schema/manifest.json` document through the consumer's own loader
//! (`Registry::load_json` / `load_yaml`) matching its `form`, and asserts the expected verdict: an
//! `accept` doc loads cleanly; a `reject` doc returns a `ConsumerError` (no panic, no partial load). This
//! is the Rust leg's GENUINELY-INDEPENDENT parity check (unlike marshaling, the loader is not the golden
//! source — critique E2). Cross-binding agreement is asserted by all three runners reaching the same
//! verdict for the same manifest (FR-009/010/011).

mod common;

use common::{load_schema_manifest, resolve_in_repo};
use prompting_press::Registry;

#[test]
fn schema_round_trip_matches_verdict() {
    let manifest = load_schema_manifest();
    assert!(!manifest.fixtures.is_empty(), "schema manifest is empty");

    let mut failures = Vec::new();

    for entry in &manifest.fixtures {
        let path = resolve_in_repo(&entry.path); // SEC-001: within-repo resolution
        let doc = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read schema fixture {}: {e}", path.display()));

        let mut reg = Registry::new();
        let outcome = match entry.form.as_str() {
            "json" => reg.load_json(&doc),
            "yaml" => reg.load_yaml(&doc),
            other => panic!("{}: unknown form {other:?}", entry.path),
        };

        match (entry.verdict.as_str(), outcome) {
            ("accept", Ok(_)) | ("reject", Err(_)) => { /* expected */ }
            ("accept", Err(e)) => failures.push(format!(
                "[rust] {} expected ACCEPT but was REJECTED: {e:?}",
                entry.path
            )),
            ("reject", Ok(_)) => failures.push(format!(
                "[rust] {} expected REJECT but was ACCEPTED",
                entry.path
            )),
            (v, _) => panic!("{}: unknown verdict {v:?}", entry.path),
        }
    }

    assert!(
        failures.is_empty(),
        "schema round-trip verdict divergences (binding+fixture):\n{}",
        failures.join("\n")
    );
}
