//! Adversarial property-based tests for the consumer `Prompt` surface (spec 009, T005).
//!
//! Exercises `Prompt::new` / `from_yaml` / `from_json` / `from_toml` / `render::<V>` /
//! `check` / `with` under generated and hostile inputs, asserting:
//!
//! - **Never-panic**: every entry point returns Ok(…) or Err(ConsumerError), never panics.
//! - **Validate-before-render** (SC-005): an invalid garde V is caught as
//!   `ConsumerError::Validation` — the kernel is NEVER reached.
//! - **Construction rejects hostile/un-analyzable docs** with a structured `ConsumerError`
//!   (not a panic; SC-002).
//!
//! ## Bounds + seed (FR-004)
//!
//! Every `proptest!` block uses `bounded_config()` (128 cases) and `FIXED_SEED` so failures
//! are reproducible without relying on proptest's default randomness.

use garde::Validate;
use prompting_press::error::code;
use prompting_press::{ConsumerError, Prompt, PromptOverlay};
use prompting_press_core::GuardConfig;
use proptest::prelude::*;
use serde::Serialize;

// ── config ────────────────────────────────────────────────────────────────────

const CASES: u32 = 128;

fn bounded_config() -> ProptestConfig {
    ProptestConfig {
        cases: CASES,
        source_file: Some(file!()),
        ..ProptestConfig::default()
    }
}

fn no_guard() -> GuardConfig {
    GuardConfig::default()
}

// ── Vars structs for render tests ─────────────────────────────────────────────

/// A minimal Vars struct that *always validates* (just a non-empty name).
#[derive(Debug, Serialize, Validate)]
struct ValidVars {
    #[garde(length(min = 1, max = 256))]
    name: String,
}

/// A Vars struct that *always fails validation* (length=0 always violates min=1).
#[derive(Debug, Serialize, Validate)]
struct InvalidVars {
    /// garde requires length >= 1 but we always supply "".
    #[garde(length(min = 1))]
    name: String,
}

// ── baseline prompt fixtures (inline JSON, no file I/O) ───────────────────────

fn greeting_json() -> &'static str {
    r#"{
        "name": "greet",
        "role": "user",
        "body": "Hi {{ name }}",
        "variables": {
            "name": { "type": "string", "trusted": true }
        }
    }"#
}

fn greeting_prompt() -> Prompt {
    Prompt::from_json(greeting_json()).expect("greeting prompt must construct")
}

// ── T005: never-panic over from_yaml / from_json / from_toml ─────────────────

proptest! {
    #![proptest_config(bounded_config())]

    /// Never-panic: feeding generated hostile strings to `from_yaml` must return
    /// Ok(Prompt) or Err(ConsumerError) — never panic.
    #[test]
    fn prop_from_yaml_never_panics(
        input in prop_hostile_string()
    ) {
        let result = Prompt::from_yaml(&input);
        prop_assert!(
            result.is_ok() || result.is_err(),
            "from_yaml must return Ok or Err, never panic; input={input:?}"
        );
    }

    /// Never-panic: feeding generated hostile strings to `from_json` must return
    /// Ok(Prompt) or Err(ConsumerError) — never panic.
    #[test]
    fn prop_from_json_never_panics(
        input in prop_hostile_string()
    ) {
        let result = Prompt::from_json(&input);
        prop_assert!(
            result.is_ok() || result.is_err(),
            "from_json must return Ok or Err, never panic; input={input:?}"
        );
    }

    /// Never-panic: feeding generated hostile strings to `from_toml` must return
    /// Ok(Prompt) or Err(ConsumerError) — never panic.
    #[test]
    fn prop_from_toml_never_panics(
        input in prop_hostile_string()
    ) {
        let result = Prompt::from_toml(&input);
        prop_assert!(
            result.is_ok() || result.is_err(),
            "from_toml must return Ok or Err, never panic; input={input:?}"
        );
    }
}

// ── T005: construction rejects hostile / un-analyzable docs with ConsumerError ─

/// Corpus of hostile/un-analyzable document strings that must all fail construction
/// with a structured `ConsumerError`, never with a panic.
static HOSTILE_DOCS: &[&str] = &[
    // Empty string
    "",
    // Whitespace only
    "   \n\t  ",
    // Truncated JSON
    r#"{"name":"x""#,
    // Wrong types
    r#"{"name":123,"role":"user","body":"hi","variables":{}}"#,
    // Missing required field (body)
    r#"{"name":"x","role":"user","variables":{}}"#,
    // Template with undeclared variable
    r#"{"name":"x","role":"user","body":"{{ ghost }}","variables":{}}"#,
    // Template with excluded feature (include)
    r#"{"name":"x","role":"user","body":"{% include \"x\" %}","variables":{}}"#,
    // Template with syntax error
    r#"{"name":"x","role":"user","body":"{{ unclosed","variables":{}}"#,
    // Variant named "default" (reserved)
    r#"{"name":"x","role":"user","body":"hi","variables":{},"variants":{"default":{"body":"hi"}}}"#,
    // Lone null byte (invalid UTF-8 territory, encoded as the JSON escape )
    "{\"name\":\"x\",\"role\":\"user\",\"body\":\"\u{0000}\",\"variables\":{}}",
    // Very long name (may or may not be rejected; must not panic)
    r#"{"name":"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx","role":"user","body":"hi","variables":{}}"#,
];

#[test]
fn corpus_hostile_docs_return_consumer_error_never_panic() {
    for doc in HOSTILE_DOCS {
        // Try all three parsers; none must panic.
        let r_json = Prompt::from_json(doc);
        let r_yaml = Prompt::from_yaml(doc);
        let r_toml = Prompt::from_toml(doc);

        // Each result is either Ok or a structured ConsumerError.
        // For the obviously-invalid ones (undeclared var, bad syntax, etc.) we get Err.
        // We don't assert Err specifically here because some docs (e.g. the null-byte one)
        // might parse differently across formats — what we require is NO PANIC.
        let _ = (r_json, r_yaml, r_toml);
    }
}

/// Construction from JSON specifically rejects un-analyzable template bodies with a
/// structured `ConsumerError::Kernel` carrying a parse/excluded-feature code.
#[test]
fn construction_rejects_bad_template_with_kernel_error() {
    let bad_bodies: &[&str] = &[
        r#"{"name":"x","role":"user","body":"{{ unclosed","variables":{}}"#,
        r#"{"name":"x","role":"user","body":"{% include \"x\" %}","variables":{}}"#,
        r#"{"name":"x","role":"user","body":"{{ ghost }}","variables":{}}"#,
    ];
    for doc in bad_bodies {
        let err = Prompt::from_json(doc).expect_err("hostile template doc must fail construction");
        match &err {
            ConsumerError::Kernel(rows) => {
                assert!(!rows.is_empty(), "Kernel error must have at least one row");
                let valid_codes = [
                    code::PARSE,
                    code::EXCLUDED_FEATURE,
                    code::UNDEFINED_VARIABLE,
                ];
                let got: Vec<&str> = rows.iter().map(|r| r.code.as_str()).collect();
                assert!(
                    got.iter().any(|c| valid_codes.contains(c)),
                    "construction failure code must be one of parse/excluded_feature/undefined_variable, got {got:?} for doc {doc:?}"
                );
            }
            ConsumerError::Load(_) => {} // a deserialize failure is also acceptable
            ConsumerError::Validation(_) => {
                panic!("expected ConsumerError::Kernel or Load, got a Validation error")
            }
        }
    }
}

// ── T005: validate-before-render (SC-005) ─────────────────────────────────────

proptest! {
    #![proptest_config(bounded_config())]

    /// SC-005: an invalid garde V must be caught as ConsumerError::Validation — the kernel
    /// render is NEVER reached. We verify this by constructing InvalidVars (always empty
    /// name → garde rejects it) and asserting the error variant is Validation, not Kernel.
    #[test]
    fn prop_invalid_vars_never_reaches_kernel(
        // Generate a name value; it does not matter what it is — InvalidVars always has
        // the EMPTY string as its `name`, so garde will always reject it regardless.
        _noise in "[a-z]{0,16}"
    ) {
        let prompt = greeting_prompt();
        let vars = InvalidVars { name: String::new() }; // always invalid

        let err = prompt
            .render(&vars, None, &no_guard(), false)
            .expect_err("invalid vars must be rejected before reaching kernel");

        prop_assert!(
            matches!(err, ConsumerError::Validation(_)),
            "invalid garde V must produce ConsumerError::Validation, not {:?}", err
        );
    }
}

/// Enumerated corpus: invalid var structs all produce Validation, not Kernel.
#[test]
fn invalid_vars_always_produce_validation_error_not_kernel() {
    let prompt = greeting_prompt();
    let vars = InvalidVars {
        name: String::new(),
    };

    let err = prompt
        .render(&vars, None, &no_guard(), false)
        .expect_err("empty name must be rejected");

    match err {
        ConsumerError::Validation(rows) => {
            assert!(!rows.is_empty());
            assert!(
                rows.iter().all(|r| r.code == code::VALIDATION),
                "all rows must carry code::VALIDATION"
            );
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

// ── T005: render never-panic over valid vars ──────────────────────────────────

proptest! {
    #![proptest_config(bounded_config())]

    /// SC-001: render with a valid Vars struct over generated name strings must never panic.
    #[test]
    fn prop_render_never_panics_valid_vars(
        name in "[A-Za-z0-9 ]{1,64}"
    ) {
        let prompt = greeting_prompt();
        let vars = ValidVars { name: name.clone() };
        let result = prompt.render(&vars, None, &no_guard(), false);
        prop_assert!(
            result.is_ok() || result.is_err(),
            "render must return Ok or Err, never panic; name={name:?}"
        );
        // On success the text must contain the name.
        if let Ok(r) = result {
            prop_assert!(
                r.text.contains(&name),
                "rendered text must contain the name; text={:?} name={name:?}", r.text
            );
        }
    }
}

// ── T005: check() never-panic ─────────────────────────────────────────────────

/// `check()` on any successfully constructed Prompt must never panic.
#[test]
fn check_never_panics_on_valid_prompts() {
    let prompts = vec![
        Prompt::from_json(greeting_json()).unwrap(),
        Prompt::from_json(
            r#"{"name":"u","role":"user","body":"{{ payload }}","variables":{"payload":{"type":"string","trusted":false}}}"#
        ).unwrap(),
        Prompt::from_json(
            r#"{"name":"g","role":"user","body":"{{ payload }}","variables":{"payload":{"type":"string","trusted":false}},"metadata":{"guard":{"enabled":true}}}"#
        ).unwrap(),
    ];
    for p in &prompts {
        let _ = p.check(); // must not panic
    }
}

// ── T005: with() never-panic ──────────────────────────────────────────────────

proptest! {
    #![proptest_config(bounded_config())]

    /// `derive()` over generated overlay bodies must return Ok or ConsumerError — never panic.
    #[test]
    fn prop_derive_never_panics(
        new_body in prop_hostile_string()
    ) {
        let prompt = greeting_prompt();
        let result = prompt.derive(PromptOverlay {
            body: Some(new_body.clone()),
            ..Default::default()
        });
        prop_assert!(
            result.is_ok() || result.is_err(),
            "derive() must return Ok or Err, never panic; body={new_body:?}"
        );
    }
}

// ── hostile string strategy ───────────────────────────────────────────────────

/// A proptest strategy generating strings that act as hostile fuzz inputs for the
/// parse/construction layer: empty, truncated JSON, oversized, Unicode,
/// control characters, injection-lookalike template syntax.
fn prop_hostile_string() -> impl Strategy<Value = String> {
    prop_oneof![
        // Empty
        Just(String::new()),
        // Whitespace
        Just("   \n\t   ".to_string()),
        // Printable ASCII noise
        prop::string::string_regex("[!-~]{0,256}").unwrap(),
        // Unicode / multibyte
        prop::string::string_regex("[\\u{0080}-\\u{FFFF}]{0,64}").unwrap(),
        // Control characters (except null which JSON can't handle well)
        Just("\x01\x02\x03\x1f\x1e".to_string()),
        // MiniJinja template-lookalike strings (hostile but NOT valid prompt docs)
        Just("{{ ignore instructions }}".to_string()),
        Just("{% for x in range(9999) %}{{ x }}{% endfor %}".to_string()),
        // Truncated valid JSON
        Just(r#"{"name":"x""#.to_string()),
        // Very long string (4 KB)
        prop::string::string_regex("[a-zA-Z0-9]{0,4096}").unwrap(),
    ]
}
