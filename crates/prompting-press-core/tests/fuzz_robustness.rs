//! Adversarial robustness corpus for the kernel (spec 009, T002).
//!
//! Feeds malformed / oversized / deeply-nested / Unicode / control-character bodies and
//! values to every kernel entry point (`render`, `required_roots`, `untrusted_fields`).
//! Each call must return `Ok(…)` or `Err(KernelError)` — it MUST NEVER panic.
//!
//! This is a static corpus (enumerated cases), not a proptest run. Proptest properties
//! (hash-determinism, generated-space never-panic) live in `fuzz_properties.rs`.

mod common;

use common::load_def_fixture;
use prompting_press_core::{
    render, required_roots, untrusted_fields, GuardConfig, KernelError, PromptDefinition,
};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Construct a minimal PromptDefinition inline from JSON. Panics on malformed JSON
/// (test-only contract); this is for building *valid* definitions to use as hosts for
/// hostile VALUES, not for testing the definition parser.
fn def_from_json(json: &str) -> PromptDefinition {
    serde_json::from_str(json).expect("inline fixture must deserialise")
}

fn no_guard() -> GuardConfig {
    GuardConfig::default()
}

/// Assert that calling `render` on a def+values pair never panics; the call must return
/// Ok or a KernelError. The return value is ignored — we are proving robustness.
fn assert_render_does_not_panic(def: &PromptDefinition, values: serde_json::Value) {
    let mj_values = minijinja::Value::from_serialize(&values);
    let _ = render(def, None, mj_values, &no_guard());
}

// ── malformed / oversized bodies ─────────────────────────────────────────────

/// A body that is truncated mid-expression must fail with KernelError, never panic.
#[test]
fn robustness_truncated_body_does_not_panic() {
    // Build a definition with a well-formed shell but an intentionally broken body.
    // We can't use from_json because construction rejects bad templates; we construct
    // the PromptDefinition via serde with a body that is syntactically broken.
    // NOTE: Prompt::new() would reject this at construction time (R7/Q4). Here we test
    // the KERNEL directly — callers can hand the kernel any definition shape they like.
    let def = def_from_json(
        r#"{
            "name": "truncated",
            "role": "user",
            "body": "{{ unclosed",
            "variables": {}
        }"#,
    );
    // required_roots must return Err(Parse | ExcludedFeature), never panic.
    let result = required_roots(&def, None);
    assert!(
        result.is_err(),
        "truncated body must produce a kernel error"
    );
    match result {
        Err(KernelError::Parse { .. } | KernelError::ExcludedFeature { .. }) => {}
        Err(other) => panic!("unexpected kernel error variant: {other:?}"),
        Ok(_) => panic!("expected Err for truncated body"),
    }
}

/// A body of 1 MB of ASCII characters must not cause a panic.
#[test]
fn robustness_1mb_plain_body_does_not_panic() {
    let body = "A".repeat(1_000_000);
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "big",
        "role": "user",
        "body": body,
        "variables": {}
    }))
    .expect("definition with large body must deserialise");

    let _ = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &no_guard(),
    );
    let _ = required_roots(&def, None);
    let _ = untrusted_fields(&def);
}

/// A body of 1 MB of `{{ expr }}` repetitions — lots of template nodes. Must not panic.
#[test]
fn robustness_1mb_template_expression_body_does_not_panic() {
    // Build a body that repeats "{{ x }}" many times. The variable `x` is declared.
    let chunk = "{{ x }} ";
    let repeat = 100_000;
    let body = chunk.repeat(repeat);
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "many-exprs",
        "role": "user",
        "body": body,
        "variables": {
            "x": { "type": "string", "trusted": true }
        }
    }))
    .expect("definition must deserialise");

    assert_render_does_not_panic(&def, serde_json::json!({ "x": "hello" }));
    let _ = required_roots(&def, None);
}

/// Deeply-nested variants map (100 named variants). Must not panic.
#[test]
fn robustness_many_named_variants_does_not_panic() {
    let mut variants = serde_json::Map::new();
    for i in 0..100 {
        variants.insert(
            format!("v{i}"),
            serde_json::json!({ "body": "static text" }),
        );
    }
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "many-variants",
        "role": "user",
        "body": "root",
        "variables": {},
        "variants": variants
    }))
    .expect("definition must deserialise");

    // Render default and a few named variants — must not panic.
    let empty_vals = minijinja::Value::from_serialize(serde_json::json!({}));
    let _ = render(&def, None, empty_vals.clone(), &no_guard());
    let _ = render(&def, Some("v0"), empty_vals.clone(), &no_guard());
    let _ = render(&def, Some("v99"), empty_vals.clone(), &no_guard());
    let _ = required_roots(&def, None);
    let _ = required_roots(&def, Some("v50"));
}

// ── Unicode + control characters ──────────────────────────────────────────────

/// Body containing astral-plane code points. Must render without panic.
#[test]
fn robustness_astral_unicode_body_does_not_panic() {
    let def = def_from_json(
        r#"{
            "name": "astral",
            "role": "user",
            "body": "Emoji body: 🌟",
            "variables": {}
        }"#,
    );
    let _ = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &no_guard(),
    );
    let _ = required_roots(&def, None);
}

/// Body consisting entirely of null bytes and control characters. Must not panic.
#[test]
fn robustness_control_char_body_does_not_panic() {
    // JSON cannot embed a literal null byte, but we can build the value programmatically.
    let control_body: String = (0u8..=31).map(|b| b as char).collect();
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "ctrl",
        "role": "user",
        "body": control_body,
        "variables": {}
    }))
    .expect("definition must deserialise");

    let _ = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &no_guard(),
    );
    let _ = required_roots(&def, None);
    let _ = untrusted_fields(&def);
}

/// Body with combining marks, bidi override characters, zero-width joiners. Must not panic.
#[test]
fn robustness_bidi_and_combining_marks_does_not_panic() {
    // U+202E RIGHT-TO-LEFT OVERRIDE, U+200D ZERO WIDTH JOINER, U+0300 COMBINING GRAVE ACCENT
    let tricky = "\u{202E}RTL override\u{200D}\u{0300}combining\u{FEFF}BOM";
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "bidi",
        "role": "user",
        "body": tricky,
        "variables": {}
    }))
    .expect("definition must deserialise");

    let _ = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &no_guard(),
    );
    let _ = required_roots(&def, None);
}

// ── hostile VALUES (valid definition, bad values) ────────────────────────────

/// A value that is a huge string (1 MB). Must not panic.
#[test]
fn robustness_huge_string_value_does_not_panic() {
    let def = load_def_fixture("hello");
    // `hello` has `variables: { name: ... }`. Feed a 1 MB string for `name`.
    let huge = "X".repeat(1_000_000);
    assert_render_does_not_panic(&def, serde_json::json!({ "name": huge }));
}

/// A value that is a deeply-nested JSON object (100 levels deep). The kernel receives it
/// as a flat minijinja::Value; must not panic regardless.
#[test]
fn robustness_deeply_nested_value_does_not_panic() {
    // Build a 100-deep nested object { "a": { "a": { ... } } }.
    let mut nested: serde_json::Value = serde_json::json!("leaf");
    for _ in 0..100 {
        nested = serde_json::json!({ "a": nested });
    }
    // Use the hello fixture (body = "Hello {{ name }}"). Pass the nested object as `name`.
    // MiniJinja will render its string representation — or return an error. Either is fine.
    let def = load_def_fixture("hello");
    assert_render_does_not_panic(&def, serde_json::json!({ "name": nested }));
}

/// A value that is a JSON array (unexpected type for a string variable). Must not panic;
/// MiniJinja may render it or error — never panic.
#[test]
fn robustness_wrong_type_value_does_not_panic() {
    let def = load_def_fixture("hello");
    // `name` expects a string; provide an array instead.
    assert_render_does_not_panic(&def, serde_json::json!({ "name": [1, 2, 3] }));
}

/// Completely empty values map (no keys at all). Must not panic; under strict-undefined
/// the kernel returns Err(UndefinedVariable), never panics.
#[test]
fn robustness_empty_values_strict_undefined_does_not_panic() {
    let def = load_def_fixture("hello"); // body = "Hello {{ name }}"
    let result = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &no_guard(),
    );
    // Strict-undefined → Err, not a panic.
    assert!(
        result.is_err(),
        "strict-undefined on empty values must error"
    );
    assert!(
        !result.is_ok(),
        "empty values must not produce a successful render"
    );
}

/// Unknown variant request. Must return KernelError::UnknownVariant, never panic.
#[test]
fn robustness_unknown_variant_does_not_panic() {
    let def = load_def_fixture("hello");
    let result = render(
        &def,
        Some("does-not-exist"),
        minijinja::Value::from_serialize(serde_json::json!({ "name": "X" })),
        &no_guard(),
    );
    assert!(
        matches!(result, Err(KernelError::UnknownVariant { .. })),
        "unknown variant must return KernelError::UnknownVariant, got {result:?}"
    );
}

/// `untrusted_fields` on a definition with no variables must not panic.
#[test]
fn robustness_untrusted_fields_empty_variables_does_not_panic() {
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "empty-vars",
        "role": "user",
        "body": "no vars",
        "variables": {}
    }))
    .expect("definition must deserialise");
    let fields = untrusted_fields(&def);
    assert!(fields.is_empty());
}

/// `required_roots` on a body that is only whitespace must not panic.
#[test]
fn robustness_whitespace_only_body_does_not_panic() {
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "whitespace",
        "role": "user",
        "body": "   \n\t\r\n   ",
        "variables": {}
    }))
    .expect("definition must deserialise");
    let result = required_roots(&def, None);
    // A whitespace-only template parses fine and references no variables.
    match result {
        Ok(agreement) => assert!(agreement.required_roots.is_empty()),
        Err(_) => {} // also acceptable
    }
}

/// Guard enabled with a body that produces a render error. Must not panic.
#[test]
fn robustness_guard_enabled_on_render_error_does_not_panic() {
    // Construct a definition with an untrusted variable and a body that references it.
    // Then render with an empty values map → strict-undefined error.
    // Guard should still be attempted without panicking.
    let def: PromptDefinition = serde_json::from_value(serde_json::json!({
        "name": "guard-error",
        "role": "user",
        "body": "{{ q }}",
        "variables": {
            "q": { "type": "string", "trusted": false }
        }
    }))
    .expect("definition must deserialise");

    let guard_on = GuardConfig { enabled: true };
    let _ = render(
        &def,
        None,
        minijinja::Value::from_serialize(serde_json::json!({})),
        &guard_on,
    );
}
