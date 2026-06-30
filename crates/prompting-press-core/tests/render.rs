//! US1 render suite (spec 002, T013): rendering a prompt to text with provenance.
//!
//! Covers quickstart scenarios V1.1, V1.3, V1.5, V1.6. Template bodies live in JSON
//! data fixtures (`tests/fixtures/defs/*.json` and `schemas/.../valid/*.json`), never
//! inlined here, per the self-referential-grep mitigation (see `tests/fixtures/README.md`).

mod common;

use common::{load_def_fixture, load_prompt_definition};
use prompting_press_core::{get_source, render, GuardConfig};

/// A disabled guard config — US1 never opts into guard expansion (US3 owns that).
fn no_guard() -> GuardConfig {
    GuardConfig { enabled: false }
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

/// TS-C2 — the reserved `Some("default")` resolves to the root body, exactly like `None`
/// (it is NOT an unknown variant). Both `render` and `get_source` honour the reserved name.
/// [FR-011]
#[test]
fn reserved_default_variant_resolves_to_root_body() {
    let def = load_def_fixture("hello");

    // render(Some("default")) == render(None): same root body, variant "default".
    let explicit = render(
        &def,
        Some("default"),
        minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" })),
        &GuardConfig::default(),
    )
    .expect("Some(\"default\") must resolve to the root body, not error");
    assert_eq!(explicit.text, "Hello Ada");
    assert_eq!(explicit.variant, "default");

    let implicit = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" })),
        &GuardConfig::default(),
    )
    .expect("None must resolve to the root body");
    assert_eq!(
        explicit, implicit,
        "Some(\"default\") and None must produce the identical RenderResult"
    );

    // get_source(Some("default")) returns the root body string.
    let src =
        get_source(&def, Some("default")).expect("get_source(Some(\"default\")) must succeed");
    assert_eq!(
        src, "Hello {{ name }}",
        "reserved default must yield the root body"
    );
    assert_eq!(
        src,
        get_source(&def, None).expect("get_source(None) must succeed"),
        "get_source(Some(\"default\")) must equal get_source(None)"
    );
}

/// TS-I1 — an empty `body` renders to the empty string with valid 64-char hex hashes.
/// [spec Edge Cases]
#[test]
fn empty_body_renders_empty_with_valid_hashes() {
    let def = load_def_fixture("empty-body");

    let result = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &GuardConfig::default(),
    )
    .expect("an empty body must render successfully");

    assert_eq!(result.text, "", "empty body renders to the empty string");
    assert_eq!(result.variant, "default");
    assert_eq!(
        result.template_hash.len(),
        64,
        "template_hash is 64 hex chars"
    );
    assert_eq!(result.render_hash.len(), 64, "render_hash is 64 hex chars");
    assert!(
        result
            .template_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "template_hash is lowercase hex"
    );
    assert!(
        result
            .render_hash
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "render_hash is lowercase hex"
    );
}

/// TS-I2 — a multibyte/non-ASCII template renders correctly; hashing is over the UTF-8
/// bytes (deterministic 64-char hex). [spec Edge Cases]
#[test]
fn unicode_multibyte_body_renders_correctly() {
    let def = load_def_fixture("unicode");

    let result = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({ "name": "世界" })),
        &GuardConfig::default(),
    )
    .expect("a unicode body must render successfully");

    assert_eq!(result.text, "こんにちは 世界 🌟");
    assert_eq!(result.variant, "default");
    assert_eq!(
        result.render_hash.len(),
        64,
        "render_hash over UTF-8 bytes is 64 hex chars"
    );
}
