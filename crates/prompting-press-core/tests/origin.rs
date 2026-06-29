//! US3 origin + guard-expansion suite (spec 002, T027; per-variable tag renamed
//! `provenance`→`origin` in spec 008).
//!
//! Covers quickstart scenarios V3.1–V3.5: exposing origin tags
//! ([`origin_view`]) and the opt-in, additive guard expansion carried in
//! [`RenderResult::guard`]. Template bodies live in JSON data fixtures
//! (`tests/fixtures/defs/*.json`), never inlined here, per the
//! self-referential-grep mitigation (see `tests/fixtures/README.md`).

mod common;

use common::load_def_fixture;
use prompting_press_core::{origin_view, render, GuardConfig};

/// A disabled guard config — the baseline plain-render configuration.
fn no_guard() -> GuardConfig {
    GuardConfig {
        enabled: false,
        template: None,
    }
}

/// Values for the mixed-provenance fixture (all three declared vars supplied so the
/// strict-undefined render succeeds).
fn mixed_values() -> minijinja::Value {
    minijinja::Value::from_serialize(serde_json::json!({
        "q": "what is rust?",
        "ctx": "background material",
        "sys": "be concise",
    }))
}

/// V3.1 — `origin_view` buckets fields by their declared tag.
///
/// `{q: untrusted, ctx: external, sys: trusted}` → `untrusted = {q}`,
/// `external = {ctx}` (trusted `sys` is the complement and is not stored). [FR-021]
#[test]
fn v3_1_origin_view_buckets_by_tag() {
    let def = load_def_fixture("provenance-mixed");

    let view = origin_view(&def);

    assert!(
        view.untrusted.iter().eq(["q"].iter()),
        "untrusted must be exactly {{q}}, got {:?}",
        view.untrusted
    );
    assert!(
        view.external.iter().eq(["ctx"].iter()),
        "external must be exactly {{ctx}}, got {:?}",
        view.external
    );
}

/// V3.2 — guard opt-out: `guard == None` and `text` equals a plain render. [FR-022, SC-005]
#[test]
fn v3_2_guard_disabled_no_field_and_plain_text() {
    let def = load_def_fixture("provenance-mixed");

    // The reference plain render (no guard involvement whatsoever).
    let plain = render(&def, None, mixed_values(), &no_guard()).expect("plain render must succeed");

    // A second render, also disabled — must be byte-identical and carry no guard field.
    let disabled =
        render(&def, None, mixed_values(), &no_guard()).expect("disabled render must succeed");

    assert_eq!(disabled.guard, None, "disabled guard must produce no field");
    assert_eq!(
        disabled.text, plain.text,
        "disabled-guard text must equal the plain render"
    );
}

/// V3.3 — guard enabled, default template: `guard = Some(s)` naming both `q` and `ctx`,
/// and `text` is byte-identical to the plain render (guard is NOT in the body). [FR-022/23, SC-005]
#[test]
fn v3_3_guard_default_template_names_fields_body_unchanged() {
    let def = load_def_fixture("provenance-mixed");

    let plain = render(&def, None, mixed_values(), &no_guard()).expect("plain render must succeed");

    let guarded = render(
        &def,
        None,
        mixed_values(),
        &GuardConfig {
            enabled: true,
            template: None,
        },
    )
    .expect("guarded render must succeed");

    let guard_text = guarded
        .guard
        .as_deref()
        .expect("guard field must be present");
    assert!(
        guard_text.contains('q'),
        "default guard text must name `q`, got: {guard_text:?}"
    );
    assert!(
        guard_text.contains("ctx"),
        "default guard text must name `ctx`, got: {guard_text:?}"
    );

    // SC-005: the body is byte-identical whether or not the guard is enabled.
    assert_eq!(
        guarded.text, plain.text,
        "guard text must NOT be concatenated into the rendered body"
    );
}

/// V3.4 — guard enabled with an override template: the `{fields}` placeholder is
/// replaced with the comma-joined sorted union. `{q (untrusted), ctx (external)}`
/// → sorted union `ctx, q`. [FR-024]
#[test]
fn v3_4_guard_override_template_uses_sorted_union() {
    let def = load_def_fixture("provenance-mixed");

    let guarded = render(
        &def,
        None,
        mixed_values(),
        &GuardConfig {
            enabled: true,
            template: Some("ATTN fields: {fields}".to_string()),
        },
    )
    .expect("guarded render must succeed");

    assert_eq!(
        guarded.guard.as_deref(),
        Some("ATTN fields: ctx, q"),
        "override template must be used with the sorted union substituted"
    );
}

/// V3.5 — an untrusted value is rendered verbatim with guard enabled; the kernel never
/// sanitizes/strips/escapes-away values. [FR-025]
#[test]
fn v3_5_untrusted_value_passes_through_unchanged() {
    let def = load_def_fixture("provenance-untrusted-only");
    let payload = "<script>alert(1)</script>";
    let values = minijinja::Value::from_serialize(serde_json::json!({ "q": payload }));

    let guarded = render(
        &def,
        None,
        values,
        &GuardConfig {
            enabled: true,
            template: None,
        },
    )
    .expect("guarded render must succeed");

    assert_eq!(
        guarded.text, payload,
        "untrusted value must pass through render unchanged (no sanitization)"
    );
    // The guard names the field; it must not have touched the body's value.
    assert!(guarded.guard.is_some(), "guard field must be present");
}

/// TS-I7 — an all-`trusted` prompt rendered with the guard ENABLED produces no guard
/// field: the untrusted∪external union is empty, so there is nothing to name. The render
/// must succeed (no panic) and carry `guard == None`. [FR-022]
#[test]
fn ts_i7_all_trusted_enabled_guard_produces_no_field() {
    // `hello` declares a single `trusted` field (`name`).
    let def = load_def_fixture("hello");
    let values = minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" }));

    let result = render(
        &def,
        None,
        values,
        &GuardConfig {
            enabled: true,
            template: None,
        },
    )
    .expect("guard-enabled render over an all-trusted prompt must succeed");

    assert_eq!(
        result.guard, None,
        "an empty untrusted/external union must yield no guard field, even when enabled"
    );
    assert_eq!(result.text, "Hello Ada");
}
