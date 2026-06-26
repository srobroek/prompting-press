//! US1 validation + boundary contract (spec 003, T008).
//!
//! Pins that validation runs ONCE, before any render (FR-002), and that the consumer's
//! public return is a normalized `Result<RenderResult, ConsumerError>` with no garde /
//! kernel type reachable (FR-004 / FR-014):
//!
//! - **V1.2** one validator violated → `Err(ConsumerError::Validation(rows))` naming the
//!   offending field, and (because it is the `Validation` variant, not `Kernel`) NO render
//!   was attempted.
//! - **V1.3** two fields violated at once → both appear in `rows` (one whole validation
//!   pass, not field-by-field short-circuit).
//! - **V1.4** the public signature is `Result<RenderResult, ConsumerError>`; a
//!   compile-level binding pins that no garde `Report` / kernel `KernelError` is reachable.
//! - **E1 three-sets gap** a Vars struct whose field name does NOT match the prompt's
//!   declared `variables` (struct has `usrname`; the template/`variables` use `username`):
//!   garde `validate()` PASSES (the value is fine), but `render` returns
//!   `Err(ConsumerError::Kernel(..))` carrying an `undefined_variable`-code row — the
//!   kernel's strict-undefined surfaced loudly + normalized, NOT a silent empty render.
//!   This pins the documented three-sets invariant (spec Assumptions / critique E1).

use garde::Validate;
use prompting_press::error::code;
use prompting_press::{render, ConsumerError, Registry, RenderResult};
use prompting_press_core::GuardConfig;
use serde::Serialize;

/// At most 100 (custom validator; same signature as the happy-path test).
fn at_most_100(value: &u32, _ctx: &()) -> garde::Result {
    if *value <= 100 {
        Ok(())
    } else {
        Err(garde::Error::new("n must be at most 100"))
    }
}

/// Typed Vars matching the prompt's declared `variables` (`name`, `n`).
#[derive(Debug, Serialize, Validate)]
struct Vars {
    #[garde(length(min = 1, max = 20))]
    name: String,
    #[garde(custom(at_most_100))]
    n: u32,
}

fn registry_with_greeting() -> Registry {
    let mut reg = Registry::new();
    let def = serde_json::from_value(serde_json::json!({
        "name": "greeting",
        "role": "user",
        "body": "Hi {{ name }}, n={{ n }}",
        "variables": {
            "name": { "type": "string",  "provenance": "trusted" },
            "n":    { "type": "integer", "provenance": "trusted" }
        }
    }))
    .expect("valid prompt definition");
    reg.insert(def);
    reg
}

/// V1.2 — one validator violated → `Validation` error naming the field; no render attempted.
#[test]
fn single_validation_failure_blocks_render() {
    let reg = registry_with_greeting();
    let vars = Vars {
        name: "Ada".to_string(),
        n: 999, // violates the custom `at_most_100` validator
    };

    let err = render(&reg, "greeting", &vars, None, &GuardConfig::default())
        .expect_err("invalid vars must not render");

    match err {
        // The `Validation` variant (not `Kernel`) is the proof no render was attempted:
        // validation short-circuits before the kernel is ever called (FR-002).
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
    let reg = registry_with_greeting();
    let vars = Vars {
        name: String::new(), // violates length(min = 1)
        n: 12_345,           // violates at_most_100
    };

    let err = render(&reg, "greeting", &vars, None, &GuardConfig::default())
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

/// V1.4 — the public return type is exactly `Result<RenderResult, ConsumerError>`. This is
/// a compile-level / inspection assertion: the binding below only type-checks because the
/// error half is `ConsumerError` (not a garde `Report` or a kernel `KernelError`) and the
/// ok half is the library-owned `RenderResult` (FR-004 / FR-014).
#[test]
fn public_return_type_is_normalized() {
    let reg = registry_with_greeting();
    let vars = Vars {
        name: "Ada".to_string(),
        n: 1,
    };

    // If `render` returned a garde/kernel type on either half, this annotation would fail
    // to compile — that is the assertion.
    let result: Result<RenderResult, ConsumerError> =
        render(&reg, "greeting", &vars, None, &GuardConfig::default());
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------------------
// E1 — three-sets gap: struct-field-name ↔ declared-`variables` MISMATCH.
// ---------------------------------------------------------------------------------------

/// A Vars struct whose field is `usrname` — deliberately misnamed versus the prompt's
/// declared `username`. garde validates the struct's *values* (the value is fine), but the
/// serialized value carries `usrname`, so the template's `{{ username }}` reference is
/// undefined at render → the kernel's strict-undefined fires.
#[derive(Debug, Serialize, Validate)]
struct MisnamedVars {
    /// Non-empty — passes garde, but the NAME disagrees with the prompt's `username`.
    #[garde(length(min = 1))]
    usrname: String,
}

/// E1 — a misnamed Vars field is NOT silent: garde passes (value is valid), but the kernel's
/// strict-undefined surfaces as a normalized `Kernel` error with the `undefined_variable`
/// code. Pins the documented three-sets invariant (spec Assumptions / critique E1).
#[test]
fn misnamed_vars_field_surfaces_undefined_variable() {
    let mut reg = Registry::new();
    let def = serde_json::from_value(serde_json::json!({
        "name": "welcome",
        "role": "user",
        "body": "Welcome {{ username }}",
        "variables": {
            "username": { "type": "string", "provenance": "trusted" }
        }
    }))
    .expect("valid prompt definition");
    reg.insert(def);

    let vars = MisnamedVars {
        usrname: "ada".to_string(), // valid VALUE; wrong NAME
    };

    // garde itself passes — the value is fine; the mismatch is a name-level gap.
    assert!(vars.validate().is_ok(), "garde must accept the valid value");

    let err = render(&reg, "welcome", &vars, None, &GuardConfig::default())
        .expect_err("a name mismatch must surface loudly, not render empty");

    match err {
        // NOT Validation (garde passed) and NOT silent — it is the kernel's strict-undefined
        // normalized into the consumer's `Kernel` variant with the stable code.
        ConsumerError::Kernel(rows) => {
            assert!(
                rows.iter().any(|r| r.code == code::UNDEFINED_VARIABLE),
                "expected an undefined_variable-code row, got {rows:?}"
            );
        }
        other => panic!("expected ConsumerError::Kernel(undefined_variable), got {other:?}"),
    }
}
