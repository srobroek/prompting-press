//! US1 render-error suite (spec 002, T015).
//!
//! Covers quickstart scenarios V1.7 (strict undefined — SC-009/FR-001a) and V1.4
//! (unknown variant — FR-009). Template bodies are in JSON data fixtures, never inlined.

mod common;

use common::{load_def_fixture, load_prompt_definition};
use prompting_press_core::{render, GuardConfig, KernelError};

fn no_guard() -> GuardConfig {
    GuardConfig {
        enabled: false,
        template: None,
    }
}

/// V1.7 — rendering `"Hello {{ name }}"` with no `name` supplied is a LOUD error,
/// never the silent `"Hello "`. Strict undefined handling. [FR-001a, SC-009]
#[test]
fn v1_7_undefined_variable_is_loud_error() {
    let def = load_def_fixture("hello");
    let empty = minijinja::Value::from_serialize(serde_json::json!({}));

    let err = render(&def, None, empty, &no_guard())
        .expect_err("undefined variable must be a loud error, not a silent empty render");

    assert!(
        matches!(err, KernelError::UndefinedVariable { .. }),
        "expected UndefinedVariable, got {err:?}"
    );
}

/// V1.4 — rendering an unknown variant name returns `UnknownVariant` naming the request.
/// [FR-009, SC-004]
#[test]
fn v1_4_unknown_variant_errors_naming_request() {
    let def = load_prompt_definition("multi-variant");
    let values =
        minijinja::Value::from_serialize(serde_json::json!({ "article": "x", "max_words": 10 }));

    let err =
        render(&def, Some("nope"), values, &no_guard()).expect_err("unknown variant must error");

    match err {
        KernelError::UnknownVariant { requested } => assert_eq!(requested, "nope"),
        other => panic!("expected UnknownVariant{{requested:\"nope\"}}, got {other:?}"),
    }
}
