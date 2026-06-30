//! US1 validation + boundary contract (spec 008 reshape of spec 003, T008).
//!
//! Post-reshape, `render` is a method on `Prompt`. Behavioral contract is unchanged:
//! validation runs ONCE, before any render (FR-002).
//!
//! - **V1.2** one validator violated → `Err(ConsumerError::Validation(rows))` naming the
//!   field; NO render attempted.
//! - **V1.3** two fields violated at once → both appear in `rows` (one whole validation
//!   pass).
//! - **V1.4** the public signature is `Result<RenderResult, ConsumerError>` — compile-level
//!   binding.
//! - **E1 three-sets gap** a Vars struct whose field name does NOT match the prompt's
//!   declared `variables`: garde passes (value is fine), but `render` returns
//!   `Err(ConsumerError::Kernel(..))` carrying an `undefined_variable`-code row — never a
//!   silent empty render (three-sets invariant, spec Assumptions / critique E1).

use garde::Validate;
use prompting_press::error::code;
use prompting_press::{ConsumerError, Prompt, RenderResult};
use prompting_press_core::GuardConfig;
use serde::Serialize;

fn at_most_100(value: &u32, _ctx: &()) -> garde::Result {
    if *value <= 100 {
        Ok(())
    } else {
        Err(garde::Error::new("n must be at most 100"))
    }
}

#[derive(Debug, Serialize, Validate)]
struct Vars {
    #[garde(length(min = 1, max = 20))]
    name: String,
    #[garde(custom(at_most_100))]
    n: u32,
}

fn greeting_prompt() -> Prompt {
    Prompt::from_json(
        r#"{
        "name": "greeting",
        "role": "user",
        "body": "Hi {{ name }}, n={{ n }}",
        "variables": {
            "name": { "type": "string",  "trusted": true },
            "n":    { "type": "integer", "trusted": true }
        }
    }"#,
    )
    .expect("valid greeting prompt")
}

/// V1.2 — one validator violated → `Validation` error naming the field; no render attempted.
#[test]
fn single_validation_failure_blocks_render() {
    let prompt = greeting_prompt();
    let vars = Vars {
        name: "Ada".to_string(),
        n: 999, // violates at_most_100
    };

    let err = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect_err("invalid vars must not render");

    match err {
        ConsumerError::Validation(rows) => {
            assert_eq!(rows.len(), 1, "exactly one field failed");
            assert_eq!(rows[0].field, "n", "the offending field is named");
            assert_eq!(rows[0].code, code::VALIDATION);
        }
        other => panic!("expected ConsumerError::Validation, got {other:?}"),
    }
}

/// V1.3 — two fields violated at once → both reported (one whole validation pass).
#[test]
fn multiple_validation_failures_all_reported() {
    let prompt = greeting_prompt();
    let vars = Vars {
        name: String::new(), // violates length(min = 1)
        n: 12_345,           // violates at_most_100
    };

    let err = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect_err("invalid vars must not render");

    match err {
        ConsumerError::Validation(rows) => {
            let fields: Vec<&str> = rows.iter().map(|r| r.field.as_str()).collect();
            assert!(
                fields.contains(&"name"),
                "name failure reported: {fields:?}"
            );
            assert!(fields.contains(&"n"), "n failure reported: {fields:?}");
            assert_eq!(rows.len(), 2, "both failures reported, not short-circuited");
        }
        other => panic!("expected ConsumerError::Validation, got {other:?}"),
    }
}

/// V1.4 — the public return type is exactly `Result<RenderResult, ConsumerError>`.
/// This is a compile-level assertion: the binding only type-checks if both halves are
/// the correct public types (FR-004 / FR-014).
#[test]
fn public_return_type_is_normalized() {
    let prompt = greeting_prompt();
    let vars = Vars {
        name: "Ada".to_string(),
        n: 1,
    };
    let result: Result<RenderResult, ConsumerError> =
        prompt.render(&vars, None, &GuardConfig::default(), false);
    assert!(result.is_ok());
}

/// A Vars struct whose field is `usrname` — deliberately misnamed versus the prompt's
/// `username`. garde validates the struct's VALUES (the value is fine), but the serialized
/// value carries `usrname`, so the template's `{{ username }}` reference is undefined at
/// render → the kernel's strict-undefined fires.
#[derive(Debug, Serialize, Validate)]
struct MisnamedVars {
    #[garde(length(min = 1))]
    usrname: String, // note: the prompt declares `username`
}

/// E1 — a misnamed Vars field is NOT silent: garde passes, but the kernel's
/// strict-undefined surfaces as a normalized `Kernel` error with `undefined_variable` code.
/// Pins the documented three-sets invariant.
#[test]
fn misnamed_vars_field_surfaces_undefined_variable() {
    let prompt = Prompt::from_json(
        r#"{
        "name": "welcome",
        "role": "user",
        "body": "Welcome {{ username }}",
        "variables": {
            "username": { "type": "string", "trusted": true }
        }
    }"#,
    )
    .expect("valid prompt must construct");

    let vars = MisnamedVars {
        usrname: "ada".to_string(), // valid VALUE; wrong NAME
    };

    // garde itself passes — the value is fine; the mismatch is a name-level gap.
    assert!(vars.validate().is_ok(), "garde must accept the valid value");

    let err = prompt
        .render(&vars, None, &GuardConfig::default(), false)
        .expect_err("a name mismatch must surface loudly, not render empty");

    match err {
        ConsumerError::Kernel(rows) => {
            assert!(
                rows.iter().any(|r| r.code == code::UNDEFINED_VARIABLE),
                "expected an undefined_variable-code row, got {rows:?}"
            );
        }
        other => panic!("expected ConsumerError::Kernel(undefined_variable), got {other:?}"),
    }
}
