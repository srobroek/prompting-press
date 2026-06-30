//! US1 happy-path render contract (spec 008 reshape of spec 003, T007).
//!
//! Post-reshape, render is a method on `Prompt` instead of a free fn over a `Registry`.
//! The behavioral contract is unchanged:
//!
//! - **V1.1** valid vars → `RenderResult` with non-empty `text` and 64-hex `template_hash`
//!   / `render_hash` (the kernel's provenance, surfaced unchanged).
//! - **V1.5** same prompt + same vars rendered twice → byte-identical `text` and hashes.
//! - **F5 guard plumbing** `GuardConfig::enabled` over a prompt declaring an untrusted field
//!   → `guard.is_some()`; default (disabled) → `guard.is_none()`.

use garde::Validate;
use prompting_press::error::code;
use prompting_press::{ConsumerError, Prompt};
use prompting_press_core::GuardConfig;
use serde::Serialize;

/// At most 100 (custom validator).
fn at_most_100(value: &u32, _ctx: &()) -> garde::Result {
    if *value <= 100 {
        Ok(())
    } else {
        Err(garde::Error::new("n must be at most 100"))
    }
}

/// Typed Vars deriving both `serde::Serialize` and `garde::Validate`.
#[derive(Debug, Serialize, Validate)]
struct Vars {
    #[garde(length(min = 1, max = 20))]
    name: String,
    #[garde(custom(at_most_100))]
    n: u32,
}

/// A prompt whose body references `name` (untrusted) and `n` (trusted). Untrusted tag is
/// needed so the guard-plumb test has a field for the kernel to name.
fn greeting_prompt() -> Prompt {
    Prompt::from_json(
        r#"{
        "name": "greeting",
        "role": "user",
        "body": "Hi {{ name }}, n={{ n }}",
        "variables": {
            "name": { "type": "string",  "trusted": false },
            "n":    { "type": "integer", "trusted": true }
        }
    }"#,
    )
    .expect("valid greeting prompt")
}

/// V1.1 — valid vars produce a `RenderResult` with non-empty text and 64-hex hashes.
#[test]
fn valid_vars_render_with_provenance() {
    let prompt = greeting_prompt();
    let vars = Vars {
        name: "Ada".to_string(),
        n: 7,
    };

    let result = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect("valid vars must render");

    assert_eq!(result.name, "greeting");
    assert_eq!(result.variant, "default");
    assert_eq!(
        result.text, "Hi Ada, n=7",
        "rendered text must interpolate both vars"
    );
    assert!(!result.text.is_empty());

    assert!(
        is_sha256_hex(&result.template_hash),
        "template_hash must be 64-hex"
    );
    assert!(
        is_sha256_hex(&result.render_hash),
        "render_hash must be 64-hex"
    );
}

/// V1.5 — same prompt + same vars rendered twice is byte-identical (kernel determinism).
#[test]
fn render_is_deterministic() {
    let prompt = greeting_prompt();
    let vars = Vars {
        name: "Grace".to_string(),
        n: 42,
    };

    let first = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect("render 1");
    let second = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect("render 2");

    assert_eq!(first.text, second.text, "text must be byte-identical");
    assert_eq!(first.template_hash, second.template_hash);
    assert_eq!(first.render_hash, second.render_hash);
}

/// F5 — the consumer PLUMBS `GuardConfig` through to the kernel and surfaces the `guard`
/// field. Enabled → `Some` (and untrusted values are wrapped in the rendered text per spec 015);
/// default (disabled) → `None` and plain text. Assert plumbing and delimiting behavior.
#[test]
fn guard_config_is_plumbed_through() {
    let prompt = greeting_prompt();
    let vars = Vars {
        name: "Ada".to_string(),
        n: 1,
    };

    let disabled = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect("render disabled guard");
    assert!(
        disabled.guard.is_none(),
        "disabled GuardConfig must surface guard = None"
    );

    let enabled_cfg = GuardConfig { enabled: true };
    let enabled = prompt
        .render(&vars, None, &enabled_cfg, false)
        .expect("render enabled guard");
    assert!(
        enabled.guard.is_some(),
        "enabled GuardConfig must surface guard = Some"
    );

    // Spec 015: when the guard is enabled, untrusted values are wrapped in
    // <untrusted>…</untrusted> in the rendered body. The `name` field is declared
    // trusted: false, so with the guard enabled its value is delimited.
    assert!(
        enabled.text.contains("<untrusted>"),
        "enabled guard must wrap untrusted values in the rendered text"
    );
    // The trusted field `n` must not be wrapped.
    assert!(
        !enabled.text.contains("untrusted>1"),
        "trusted field must not be wrapped by the guard"
    );
    // The disabled render has no delimiters.
    assert!(
        !disabled.text.contains("<untrusted>"),
        "disabled guard must produce plain rendered text"
    );
}

/// A multi-variant prompt: root body + named `concise` variant, each referencing `name`.
fn variants_prompt() -> Prompt {
    Prompt::from_json(
        r#"{
        "name": "greet",
        "role": "user",
        "body": "Hello there, {{ name }}!",
        "variants": {
            "concise": { "body": "Hi {{ name }}" }
        },
        "variables": {
            "name": { "type": "string", "trusted": true }
        }
    }"#,
    )
    .expect("valid multi-variant prompt")
}

/// A single-field Vars struct for the multi-variant prompt.
#[derive(Debug, Serialize, Validate)]
struct NameVars {
    #[garde(length(min = 1, max = 50))]
    name: String,
}

/// TS-1(a) — `get_source(None)` returns the root body's unrendered template source.
#[test]
fn get_source_returns_root_body() {
    let prompt = variants_prompt();
    let src = prompt.get_source(None).expect("root source must resolve");
    assert_eq!(
        src, "Hello there, {{ name }}!",
        "must return the root body source"
    );
}

/// TS-1(a, variant) — `get_source` with a declared variant returns that arm's source.
#[test]
fn get_source_returns_named_variant_body() {
    let prompt = variants_prompt();
    let src = prompt
        .get_source(Some("concise"))
        .expect("variant source must resolve");
    assert_eq!(
        src, "Hi {{ name }}",
        "must return the named variant's body source"
    );
}

/// TS-1(c) — `get_source` for an unknown variant resolves to a normalized `Kernel` error
/// carrying `code::UNKNOWN_VARIANT`.
#[test]
fn get_source_unknown_variant_is_kernel_error() {
    let prompt = variants_prompt();
    let err = prompt
        .get_source(Some("nope"))
        .expect_err("unknown variant must error");
    match err {
        ConsumerError::Kernel(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].code,
                code::UNKNOWN_VARIANT,
                "must map to unknown_variant"
            );
        }
        other => panic!("expected ConsumerError::Kernel, got {other:?}"),
    }
}

/// TS-2(a) — rendering a declared named variant selects that arm.
#[test]
fn named_variant_render_selects_that_arm() {
    let prompt = variants_prompt();
    let vars = NameVars {
        name: "Ada".to_string(),
    };
    let result = prompt
        .render(&vars, Some("concise"), &GuardConfig::default(), false)
        .expect("named variant must render");

    assert_eq!(result.name, "greet");
    assert_eq!(
        result.variant, "concise",
        "the selected variant must be surfaced"
    );
    assert_eq!(
        result.text, "Hi Ada",
        "text must come from the variant's body"
    );
}

/// TS-2(b) — rendering an unknown variant resolves to a normalized `Kernel` error with
/// `code::UNKNOWN_VARIANT`.
#[test]
fn render_unknown_variant_is_kernel_error() {
    let prompt = variants_prompt();
    let vars = NameVars {
        name: "Ada".to_string(),
    };
    let err = prompt
        .render(&vars, Some("nope"), &GuardConfig::default(), false)
        .expect_err("unknown variant must error");
    match err {
        ConsumerError::Kernel(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].code,
                code::UNKNOWN_VARIANT,
                "must map to unknown_variant"
            );
        }
        other => panic!("expected ConsumerError::Kernel, got {other:?}"),
    }
}

/// Lowercase 64-char hex (a SHA256 digest).
fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
