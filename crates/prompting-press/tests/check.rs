//! Advisory lint contract (spec 008 reshape of spec 003, T015; FR-016/017/018/020).
//!
//! Post-reshape, the lint runs per-prompt via `Prompt::check()`. Construction enforces the
//! hard invariants (agreement, parse, reserved name). `Prompt::check()` surfaces only the
//! trust/guard advisory — `UntrustedWithoutGuard`.
//!
//! Vignettes:
//! - **V3.1** a well-formed prompt (declared vars, guarded untrusted field) → `check()` passes.
//! - **V3.2** agreement violations are **construction failures** post-reshape — tested in
//!   `tests/prompt_construct.rs` and in `prompt.rs` unit tests; not tested here as a
//!   `check()` advisory.
//! - **V3.3** a prompt declaring an `untrusted` var with NO `guard` key in `metadata`
//!   → one `UntrustedWithoutGuard` finding naming the prompt + field (SC-005).
//! - **V3.5** each variant's agreement is enforced at construction — a multi-variant prompt
//!   with an undeclared var in one variant fails `Prompt::new`, not `check()`.
//! - **EMPTY** a trivially-declared prompt with no untrusted fields → empty report (pass).

use prompting_press::check::FindingKind;
use prompting_press::{CheckReport, Prompt};

/// V3.1 — a well-formed prompt (all template roots declared; one untrusted var has a
/// `metadata.guard`) → `check()` passes clean (no findings).
#[test]
fn well_formed_prompt_passes() {
    // All roots declared; untrusted field has a guard in metadata → no advisory.
    let prompt = Prompt::from_json(
        r#"{
        "name": "summarize",
        "role": "system",
        "body": "Summarize: {{ doc }}",
        "metadata": { "guard": { "enabled": true } },
        "variables": {
            "doc": { "type": "string", "trusted": false }
        }
    }"#,
    )
    .expect("well-formed prompt must construct");

    let report = prompt.check();
    assert!(
        report.passed(),
        "well-formed guarded prompt must pass check, got findings: {:?}",
        report.findings
    );
}

/// V3.3 (SC-005) — a prompt declaring an `untrusted` variable with NO `guard` key produces
/// one `UntrustedWithoutGuard` finding per uncovered field.
#[test]
fn untrusted_without_guard_is_flagged() {
    let prompt = Prompt::from_json(
        r#"{
        "name": "unguarded",
        "role": "user",
        "body": "Process {{ payload }}",
        "variables": {
            "payload": { "type": "string", "trusted": false }
        }
    }"#,
    )
    .expect("valid shape must construct");

    let report = prompt.check();
    assert!(!report.passed(), "untrusted-without-guard must fail");

    let prov: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(&f.kind, FindingKind::UntrustedWithoutGuard { .. }))
        .collect();
    assert_eq!(
        prov.len(),
        1,
        "exactly one provenance finding expected, got: {:?}",
        report.findings
    );

    let finding = prov[0];
    assert_eq!(finding.prompt, "unguarded", "finding must name the prompt");
    // The trust/guard advisory is prompt-level (no variant).
    assert_eq!(
        finding.variant, None,
        "trust/guard finding is prompt-level (no variant)"
    );
    let FindingKind::UntrustedWithoutGuard { field } = &finding.kind;
    assert_eq!(field, "payload", "finding must name the uncovered field");
    assert!(
        finding.detail.contains("payload"),
        "detail must name the field: {:?}",
        finding.detail
    );
}

/// A `trusted: false` field triggers the guard obligation. A `guard` key in `metadata` satisfies it.
/// (Previously tested with `origin: "external"`; that enum collapsed to `trusted: false` in spec 015.)
#[test]
fn untrusted_false_field_obligation_and_metadata_guard_satisfaction() {
    // Untrusted field (trusted: false), NO guard anywhere → flagged.
    let no_guard = Prompt::from_json(
        r#"{
        "name": "ext",
        "role": "user",
        "body": "{{ feed }}",
        "variables": { "feed": { "type": "string", "trusted": false } }
    }"#,
    )
    .expect("valid shape must construct");
    let report = no_guard.check();
    let prov: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(&f.kind, FindingKind::UntrustedWithoutGuard { .. }))
        .collect();
    assert_eq!(
        prov.len(),
        1,
        "untrusted (trusted: false) field must carry the obligation"
    );
    let FindingKind::UntrustedWithoutGuard { field } = &prov[0].kind;
    assert_eq!(field, "feed");

    // Same prompt but with a `metadata.guard` key → satisfied (no provenance finding).
    let with_guard = Prompt::from_json(
        r#"{
        "name": "ext",
        "role": "user",
        "body": "{{ feed }}",
        "metadata": { "guard": "configured-elsewhere" },
        "variables": { "feed": { "type": "string", "trusted": false } }
    }"#,
    )
    .expect("valid shape must construct");
    assert!(
        with_guard.check().passed(),
        "a `metadata.guard` key must satisfy the provenance obligation"
    );
}

/// A prompt with no untrusted/external variables → `check()` returns an empty report (pass).
#[test]
fn trusted_only_prompt_passes_check() {
    let prompt = Prompt::from_json(
        r#"{
        "name": "greet",
        "role": "user",
        "body": "Hi {{ name }}, you have {{ count }} messages",
        "variables": {
            "name":  { "type": "string",  "trusted": true },
            "count": { "type": "integer", "trusted": true }
        }
    }"#,
    )
    .expect("valid prompt must construct");

    let report = prompt.check();
    assert!(
        report.passed(),
        "all-trusted prompt must pass check, findings: {:?}",
        report.findings
    );
    assert!(report.findings.is_empty());
}

/// The `CheckReport` type is correctly accessible at the crate root.
#[test]
fn check_report_type_is_accessible() {
    let prompt = Prompt::from_json(
        r#"{
        "name": "t",
        "role": "user",
        "body": "{{ x }}",
        "variables": { "x": { "type": "string", "trusted": true } }
    }"#,
    )
    .expect("valid");
    let _report: CheckReport = prompt.check();
}
