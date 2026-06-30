//! US1 hashing / provenance suite (spec 002, T014).
//!
//! Covers quickstart scenarios V1.2 (determinism, SC-001), V1.8 (`get_source` bytes
//! hash to the rendered `template_hash`), per-variant distinct `template_hash`
//! (US1 scenario 7), and the FR-014 invariant that `RenderResult` carries no
//! `vars_hash`. Template bodies are in JSON data fixtures, never inlined here.

mod common;

use common::{load_def_fixture, load_prompt_definition};
use prompting_press_core::{get_source, render, GuardConfig, RenderResult};
use sha2::{Digest, Sha256};

fn no_guard() -> GuardConfig {
    GuardConfig {
        enabled: false,
        ..Default::default()
    }
}

/// Independent lowercase-hex SHA-256 of a string's UTF-8 bytes, used to cross-check the
/// kernel's hashing without reaching into its private helper.
fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    // sha2 0.11: digest is a `hybrid_array::Array` (no `LowerHex`); hex-encode bytes.
    let mut hex = String::new();
    for byte in hasher.finalize() {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// V1.2 — re-rendering with identical inputs is byte-identical with equal hashes.
/// [FR-003, SC-001]
#[test]
fn v1_2_determinism_byte_identical_and_equal_hashes() {
    let def = load_def_fixture("hello");
    let mk = || minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" }));

    let first = render(&def, None, mk(), &no_guard()).expect("first render");
    let second = render(&def, None, mk(), &no_guard()).expect("second render");

    assert_eq!(first.text, second.text, "text is byte-identical");
    assert_eq!(
        first.template_hash, second.template_hash,
        "template_hash equal across renders"
    );
    assert_eq!(
        first.render_hash, second.render_hash,
        "render_hash equal across renders"
    );
}

/// V1.8 — `get_source(def, Some("concise"))` returns the unrendered arm source; its
/// SHA-256 hex equals the `template_hash` produced when rendering `concise`. Also pins
/// `render_hash == sha256(text)` and `template_hash == sha256(source)`. [FR-006, FR-012]
#[test]
fn v1_8_get_source_hash_matches_template_hash() {
    let def = load_prompt_definition("multi-variant");

    let source = get_source(&def, Some("concise")).expect("get_source concise");
    assert_eq!(source, "In one sentence, summarise: {{article}}");

    let values = minijinja::Value::from_serialize(serde_json::json!({ "article": "A long text." }));
    let result = render(&def, Some("concise"), values, &no_guard()).expect("render concise");

    assert_eq!(
        result.template_hash,
        sha256_hex(source),
        "template_hash == sha256(get_source)"
    );
    assert_eq!(
        result.render_hash,
        sha256_hex(&result.text),
        "render_hash == sha256(text)"
    );
}

/// US1 scenario 7 — two different variants of the same prompt each carry their own
/// `template_hash`, computed over that variant's own source. [FR-012 per-variant]
#[test]
fn per_variant_distinct_template_hash() {
    let def = load_prompt_definition("multi-variant");

    let default_src = get_source(&def, None).expect("default source");
    let concise_src = get_source(&def, Some("concise")).expect("concise source");
    let structured_src = get_source(&def, Some("structured")).expect("structured source");

    // The three arms differ in source, so their template_hashes must differ.
    let default_hash = sha256_hex(default_src);
    let concise_hash = sha256_hex(concise_src);
    let structured_hash = sha256_hex(structured_src);

    assert_ne!(default_hash, concise_hash);
    assert_ne!(default_hash, structured_hash);
    assert_ne!(concise_hash, structured_hash);

    // And a render of each stamps that arm's own hash.
    let values =
        minijinja::Value::from_serialize(serde_json::json!({ "article": "x", "max_words": 10 }));
    let concise = render(&def, Some("concise"), values.clone(), &no_guard()).expect("concise");
    let structured = render(&def, Some("structured"), values, &no_guard()).expect("structured");

    assert_eq!(concise.template_hash, concise_hash);
    assert_eq!(structured.template_hash, structured_hash);
    assert_ne!(concise.template_hash, structured.template_hash);
}

/// FR-014 — `RenderResult` carries NO `vars_hash` (nor any hash over structured input).
/// Compile-level assertion: exhaustively destructure every field; if a `vars_hash`
/// field were ever added, this stops compiling and the test fails loudly.
#[test]
fn fr_014_render_result_has_no_vars_hash() {
    let def = load_def_fixture("hello");
    let values = minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" }));
    let result = render(&def, None, values, &no_guard()).expect("render");

    let RenderResult {
        text: _,
        name: _,
        variant: _,
        template_hash: _,
        render_hash: _,
        guard: _,
    } = result;
    // No `vars_hash` binding above ⇒ none exists on the struct (FR-014).
}
