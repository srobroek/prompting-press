//! US3 agreement + provenance lint contract (spec 003, T015; FR-016/017/018/020).
//!
//! Exercises [`prompting_press::check`] over a [`Registry`] of constructed
//! [`PromptDefinition`]s. The lint is the library's headline differentiator — it catches,
//! *before* render, the class of bug where a template references a variable the prompt never
//! declared (the agreement half, SC-004) and the class where a prompt declares an
//! untrusted/external input but configures no guard for it (the reframed provenance half,
//! SC-005).
//!
//! Vignettes:
//! - **V3.1** a registry of well-formed prompts (every template root declared; the one
//!   untrusted var carries a `meta.guard`) → `check()` passes (empty findings).
//! - **V3.2** a prompt whose `body` references a var NOT in `variables` → one
//!   `UndeclaredVariable` finding naming the prompt + variant (SC-004).
//! - **V3.3** a prompt declaring an `untrusted` var with NO `guard` key in `meta`/`metadata`
//!   → one `UntrustedWithoutGuard` finding naming the prompt + field (SC-005).
//! - **V3.5** a multi-variant prompt where ONE named variant's body references an undeclared
//!   var → that variant flagged (confirms each variant is analyzed independently).
//! - **EMPTY** an empty registry → empty `CheckReport` (pass — F7).
//!
//! Registries are built by `insert`ing constructed `PromptDefinition`s (deserialized from
//! `serde_json::json!`), so `meta` / `variables` / `variants` are set exactly as each
//! vignette needs; provenance tags live in `variables.*.provenance`.

use prompting_press::check::FindingKind;
use prompting_press::{check, PromptDefinition, Registry};

/// Deserialize a `PromptDefinition` from a JSON value (the constructed-object path — no
/// loader, no I/O). Panics on a malformed fixture (a test-author error, not a lint concern).
fn def(value: serde_json::Value) -> PromptDefinition {
    serde_json::from_value(value).expect("valid prompt-definition fixture")
}

/// V3.1 — a registry whose prompts each reference only declared variables, and whose one
/// untrusted-declaring prompt carries a `meta.guard`, passes clean (no findings).
#[test]
fn well_formed_registry_passes() {
    let mut reg = Registry::new();

    // All roots declared; no untrusted/external fields → no provenance obligation.
    reg.insert(def(serde_json::json!({
        "name": "greet",
        "role": "user",
        "body": "Hi {{ name }}, you have {{ count }} messages",
        "variables": {
            "name":  { "type": "string",  "provenance": "trusted" },
            "count": { "type": "integer", "provenance": "trusted" }
        }
    })));

    // Declares an `untrusted` field BUT configures a guard via `meta.guard` → satisfied.
    reg.insert(def(serde_json::json!({
        "name": "summarize",
        "role": "system",
        "body": "Summarize: {{ doc }}",
        "meta": { "guard": { "enabled": true } },
        "variables": {
            "doc": { "type": "string", "provenance": "untrusted" }
        }
    })));

    let report = check(&reg);
    assert!(
        report.passed(),
        "well-formed registry must pass, got findings: {:?}",
        report.findings
    );
    assert!(report.findings.is_empty());
}

/// V3.2 (SC-004) — a prompt whose `body` references a variable absent from `variables`
/// produces exactly one `UndeclaredVariable` finding, naming the prompt + variant + variable.
#[test]
fn undeclared_variable_is_flagged() {
    let mut reg = Registry::new();
    // `body` references `name` AND `secret`; only `name` is declared.
    reg.insert(def(serde_json::json!({
        "name": "leaky",
        "role": "user",
        "body": "Hello {{ name }} -- {{ secret }}",
        "variables": {
            "name": { "type": "string", "provenance": "trusted" }
        }
    })));

    let report = check(&reg);
    assert!(!report.passed(), "an undeclared var must fail the check");

    let undeclared: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(&f.kind, FindingKind::UndeclaredVariable { .. }))
        .collect();
    assert_eq!(
        undeclared.len(),
        1,
        "exactly one undeclared-variable finding expected, got: {:?}",
        report.findings
    );

    let finding = undeclared[0];
    assert_eq!(finding.prompt, "leaky", "finding must name the prompt");
    assert_eq!(
        finding.variant.as_deref(),
        Some("default"),
        "finding must name the variant (the default arm)"
    );
    match &finding.kind {
        FindingKind::UndeclaredVariable { name } => {
            assert_eq!(name, "secret", "finding must name the undeclared variable");
        }
        other => panic!("expected UndeclaredVariable, got {other:?}"),
    }
    // `detail` is actionable (FR-020): it mentions the offending variable.
    assert!(
        finding.detail.contains("secret"),
        "detail must be actionable: {:?}",
        finding.detail
    );
}

/// V3.3 (SC-005) — a prompt declaring an `untrusted` variable with NO `guard` key in either
/// `meta` or `metadata` produces one `UntrustedWithoutGuard` finding naming the prompt +
/// field. (The agreement half is clean here: the template root is declared.)
#[test]
fn untrusted_without_guard_is_flagged() {
    let mut reg = Registry::new();
    reg.insert(def(serde_json::json!({
        "name": "unguarded",
        "role": "user",
        "body": "Process {{ payload }}",
        // No `meta.guard` and no `metadata.guard` → the untrusted field is uncovered.
        "variables": {
            "payload": { "type": "string", "provenance": "untrusted" }
        }
    })));

    let report = check(&reg);
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
    // The provenance lint is a prompt-level obligation, not per-variant.
    assert_eq!(
        finding.variant, None,
        "the provenance finding is prompt-level (no variant)"
    );
    match &finding.kind {
        FindingKind::UntrustedWithoutGuard { field } => {
            assert_eq!(field, "payload", "finding must name the uncovered field");
        }
        other => panic!("expected UntrustedWithoutGuard, got {other:?}"),
    }
    assert!(
        finding.detail.contains("payload"),
        "detail must name the field: {:?}",
        finding.detail
    );
}

/// An `external`-tagged field counts toward the provenance obligation exactly like
/// `untrusted` (the lint flags the union untrusted ∪ external — FR-018). A `guard` key in
/// `metadata` (not just `meta`) satisfies it.
#[test]
fn external_field_obligation_and_metadata_guard_satisfaction() {
    // External field, NO guard anywhere → flagged.
    let mut flagged = Registry::new();
    flagged.insert(def(serde_json::json!({
        "name": "ext",
        "role": "user",
        "body": "{{ feed }}",
        "variables": { "feed": { "type": "string", "provenance": "external" } }
    })));
    let report = check(&flagged);
    let prov: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(&f.kind, FindingKind::UntrustedWithoutGuard { .. }))
        .collect();
    assert_eq!(prov.len(), 1, "external field must carry the obligation");
    match &prov[0].kind {
        FindingKind::UntrustedWithoutGuard { field } => assert_eq!(field, "feed"),
        other => panic!("expected UntrustedWithoutGuard, got {other:?}"),
    }

    // Same prompt but with a `metadata.guard` key → satisfied (no provenance finding).
    let mut satisfied = Registry::new();
    satisfied.insert(def(serde_json::json!({
        "name": "ext",
        "role": "user",
        "body": "{{ feed }}",
        "metadata": { "guard": "configured-elsewhere" },
        "variables": { "feed": { "type": "string", "provenance": "external" } }
    })));
    assert!(
        check(&satisfied).passed(),
        "a `metadata.guard` key must satisfy the provenance obligation"
    );
}

/// V3.5 — a multi-variant prompt where ONE named variant references an undeclared var: that
/// variant (and only that variant) is flagged, confirming each variant is analyzed against
/// the shared declared `variables`.
#[test]
fn each_variant_is_analyzed_independently() {
    let mut reg = Registry::new();
    reg.insert(def(serde_json::json!({
        "name": "multi",
        "role": "user",
        // default + `terse` reference only declared `topic`; `verbose` references `extra`.
        "body": "Tell me about {{ topic }}",
        "variants": {
            "terse":   { "body": "{{ topic }}?" },
            "verbose": { "body": "Elaborate on {{ topic }} and {{ extra }}" }
        },
        "variables": {
            "topic": { "type": "string", "provenance": "trusted" }
        }
    })));

    let report = check(&reg);
    assert!(!report.passed(), "the verbose variant must be flagged");

    let undeclared: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(&f.kind, FindingKind::UndeclaredVariable { .. }))
        .collect();
    assert_eq!(
        undeclared.len(),
        1,
        "only the verbose variant should fail, got: {:?}",
        report.findings
    );

    let finding = undeclared[0];
    assert_eq!(finding.prompt, "multi");
    assert_eq!(
        finding.variant.as_deref(),
        Some("verbose"),
        "the finding must name the offending variant"
    );
    match &finding.kind {
        FindingKind::UndeclaredVariable { name } => assert_eq!(name, "extra"),
        other => panic!("expected UndeclaredVariable, got {other:?}"),
    }
}

/// EMPTY registry → empty `CheckReport` (pass — F7), never a panic.
#[test]
fn empty_registry_passes() {
    let reg = Registry::new();
    let report = check(&reg);
    assert!(report.passed(), "empty registry must pass");
    assert!(report.findings.is_empty());
}
