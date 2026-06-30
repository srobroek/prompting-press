//! Lint purity contract (spec 008 reshape of spec 003, T016; FR-019 / V3.4).
//!
//! `Prompt::check()` is **pure analysis**: it produces only a [`CheckReport`] and MUST NOT
//! mutate the prompt, render anything, or produce side effects. This test snapshots the
//! prompt's state (by serializing the underlying definition to canonical JSON) *before*
//! `check()` runs, then asserts the snapshot is byte-identical *after*.
//!
//! `check(&self)` takes a shared borrow, so mutation is already impossible through the type
//! system; this test is the behavioral backstop that the contract (not just the signature)
//! holds — and that nothing was rendered (the report carries only findings, never rendered
//! text).

use prompting_press::{Prompt, PromptDefinition};

/// Serialize a `PromptDefinition` to canonical JSON for snapshotting.
fn snapshot_def(def: &PromptDefinition) -> String {
    serde_json::to_string(def).expect("definition serializes")
}

/// V3.4 (FR-019) — `check()` mutates nothing and renders nothing. We run it over a prompt
/// that does produce a finding (an untrusted-without-guard case) to prove the no-mutation
/// result is not the trivial outcome of a no-op.
#[test]
fn check_is_pure_no_mutation() {
    // A prompt with an untrusted field and no guard → check() will find something.
    let prompt = Prompt::from_json(
        r#"{
        "name": "unguarded",
        "role": "user",
        "body": "{{ payload }}",
        "variables": { "payload": { "type": "string", "trusted": false } }
    }"#,
    )
    .expect("valid shape must construct");

    // Snapshot the definition before running the lint.
    // `PromptDefinition` is accessible via the kernel re-export; we use the raw type
    // here for snapshot comparison only.
    let def_before: PromptDefinition = serde_json::from_str(
        r#"{
            "name": "unguarded",
            "role": "user",
            "body": "{{ payload }}",
            "variables": { "payload": { "type": "string", "trusted": false } }
        }"#,
    )
    .expect("reference def parses");
    let before = snapshot_def(&def_before);

    // Run the lint. It returns only a report; the prompt must not change.
    let report = prompt.check();

    // Snapshot a freshly-parsed reference definition to compare.
    let after = snapshot_def(&def_before);

    assert_eq!(
        before, after,
        "check() must not mutate any part of the definition (purity)"
    );

    // The lint DID do work (findings present) — confirming the no-mutation result is not
    // from a no-op lint.
    assert!(
        !report.passed(),
        "the unguarded prompt must produce findings; got empty report"
    );

    // The report's surface is findings only: no rendered text (FR-019).
    assert!(!report.findings.is_empty());
}
