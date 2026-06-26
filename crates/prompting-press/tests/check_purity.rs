//! US3 lint purity contract (spec 003, T016; FR-019 / V3.4).
//!
//! `check()` is **pure analysis**: it produces only a [`CheckReport`] and MUST NOT mutate
//! any prompt, definition, or input, render anything, or produce side effects. This test
//! snapshots the registry (by serializing every definition to canonical JSON) *before*
//! `check()` runs, then asserts the snapshot is byte-identical *after* — proving the lint
//! left the registry untouched.
//!
//! `check(&Registry)` takes a shared borrow, so mutation is already impossible through the
//! type system; this test is the behavioral backstop that the contract (not just the
//! signature) holds — and that nothing was rendered (the report carries only findings, never
//! rendered text).

use prompting_press::{check, PromptDefinition, Registry};

fn def(value: serde_json::Value) -> PromptDefinition {
    serde_json::from_value(value).expect("valid prompt-definition fixture")
}

/// Serialize every definition in the registry to a stable `(name, json)` vector — the
/// snapshot we compare before/after. `Registry::iter()` walks a `BTreeMap`, so order is
/// deterministic; `serde_json::to_string` of each def captures its full state.
fn snapshot(reg: &Registry) -> Vec<(String, String)> {
    reg.iter()
        .map(|(name, def)| {
            (
                name.to_string(),
                serde_json::to_string(def).expect("definition serializes"),
            )
        })
        .collect()
}

/// V3.4 (FR-019) — `check()` mutates nothing and renders nothing. We run it over a registry
/// containing BOTH a clean prompt and two failing prompts (an undeclared-variable case and
/// an untrusted-without-guard case) so the lint actively produces findings; the registry
/// must still be byte-identical afterward.
#[test]
fn check_is_pure_no_mutation() {
    let mut reg = Registry::new();
    reg.insert(def(serde_json::json!({
        "name": "clean",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": { "name": { "type": "string", "provenance": "trusted" } }
    })));
    reg.insert(def(serde_json::json!({
        "name": "undeclared",
        "role": "user",
        "body": "{{ name }} {{ ghost }}",
        "variables": { "name": { "type": "string", "provenance": "trusted" } }
    })));
    reg.insert(def(serde_json::json!({
        "name": "unguarded",
        "role": "user",
        "body": "{{ payload }}",
        "variables": { "payload": { "type": "string", "provenance": "untrusted" } }
    })));

    let before = snapshot(&reg);

    // Run the lint. It returns only a report; it must not touch the registry.
    let report = check(&reg);

    let after = snapshot(&reg);

    assert_eq!(
        before, after,
        "check() must not mutate any definition in the registry"
    );

    // The lint DID do work (findings present) — proving the no-mutation result is not the
    // trivial outcome of a no-op over a passing registry.
    assert!(
        !report.passed(),
        "this fixture has failing prompts; the report should carry findings"
    );

    // The report's surface is findings only: there is no rendered text on it (FR-019 — the
    // lint renders nothing). This is enforced structurally (CheckReport has only `findings`)
    // and asserted here as a behavioral guard.
    assert!(!report.findings.is_empty());
}
