//! US1 render suite (spec 002, T013): rendering a prompt to text with provenance.
//!
//! Covers quickstart scenarios V1.1, V1.3, V1.5, V1.6. Template bodies live in JSON
//! data fixtures (`tests/fixtures/defs/*.json` and `schemas/.../valid/*.json`), never
//! inlined here, per the self-referential-grep mitigation (see `tests/fixtures/README.md`).

mod common;

use common::{load_def_fixture, load_prompt_definition};
use prompting_press_core::{render, GuardConfig};

/// A disabled guard config — US1 never opts into guard expansion (US3 owns that).
fn no_guard() -> GuardConfig {
    GuardConfig {
        enabled: false,
        template: None,
    }
}

/// V1.1 — render a single-body prompt with no variant named.
///
/// `"Hello {{ name }}"` + `{name: "Ada"}` → `"Hello Ada"`, variant `default`,
/// both hashes present. [FR-001, FR-007, FR-012/13]
#[test]
fn v1_1_single_body_default_render() {
    let def = load_def_fixture("hello");
    let values = minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" }));

    let result = render(&def, None, values, &no_guard()).expect("render must succeed");

    assert_eq!(result.text, "Hello Ada");
    assert_eq!(result.name, "hello");
    assert_eq!(result.variant, "default");
    assert!(!result.template_hash.is_empty(), "template_hash present");
    assert!(!result.render_hash.is_empty(), "render_hash present");
    assert_eq!(result.guard, None, "US1 never opts into guard expansion");
}

/// V1.3 — multi-variant prompt: render the named variant `concise`.
///
/// Renders the `concise` arm and stamps `variant = "concise"` with its own hash.
/// [FR-008, FR-012 (per-variant)]
#[test]
fn v1_3_named_variant_render() {
    let def = load_prompt_definition("multi-variant");
    // `concise` body is `"In one sentence, summarise: {{article}}"`.
    let values = minijinja::Value::from_serialize(serde_json::json!({ "article": "A long text." }));

    let result =
        render(&def, Some("concise"), values, &no_guard()).expect("render concise must succeed");

    assert_eq!(result.variant, "concise");
    assert_eq!(result.text, "In one sentence, summarise: A long text.");
    assert_eq!(result.name, "content-summariser");
    assert!(!result.template_hash.is_empty());
}

/// V1.5 — multi-variant prompt rendered with no variant named.
///
/// Resolves the root `body` as the reserved `default` arm (the root body is ALWAYS the
/// default; the kernel MUST NOT silently pick a named arm). [FR-007/011]
#[test]
fn v1_5_multi_variant_none_resolves_root_body_as_default() {
    let def = load_prompt_definition("multi-variant");
    // The root body is `"Summarise the following article in {{max_words}} words or
    // fewer:\n\n{{article}}"`.
    let values = minijinja::Value::from_serialize(
        serde_json::json!({ "max_words": 50, "article": "A long text." }),
    );

    let result = render(&def, None, values, &no_guard()).expect("default render must succeed");

    assert_eq!(result.variant, "default");
    assert_eq!(
        result.text,
        "Summarise the following article in 50 words or fewer:\n\nA long text."
    );
    // Distinct from a named arm: the default arm is the root body, never `concise`.
    assert_ne!(result.text, "In one sentence, summarise: A long text.");
}

/// V1.6 — a conditional + loop template renders correctly. [FR-001]
#[test]
fn v1_6_conditional_and_loop_render() {
    let def = load_def_fixture("conditional-loop");
    let values = minijinja::Value::from_serialize(
        serde_json::json!({ "items": ["alpha", "beta", "gamma"] }),
    );

    let result = render(&def, None, values, &no_guard()).expect("render must succeed");

    assert_eq!(result.text, "Items:\n- alpha\n- beta\n- gamma");
    assert_eq!(result.variant, "default");
}
