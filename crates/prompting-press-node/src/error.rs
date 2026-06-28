//! The Rust side of error normalization for the Node binding (FR-014/FR-015, research D4 /
//! C-06 / SEC-004).
//!
//! ## The contract
//!
//! The JS `Error`-subclass **hierarchy lives in TypeScript** (`packages/typescript/src/index.ts`,
//! built by the TS agent), not in Rust — JS error classes cannot be minted cleanly from napi,
//! and `instanceof` + the `readonly errors` rows are cleaner TS-side (research D4). The Rust
//! side's job is to surface a **structured, already-scrubbed payload** that the TS facade decodes
//! into the right subclass.
//!
//! Native error types — the Rust [`ConsumerError`]/[`KernelError`] — **never** cross the boundary
//! as themselves (C-06). They are mapped to a [`napi::Error`] whose `reason` is a JSON document:
//!
//! ```json
//! { "code": "<top-level code>", "errors": [ { "field": "...", "code": "...", "message": "..." } ] }
//! ```
//!
//! where every `code` is from the **same closed vocabulary** the Rust consumer exposes
//! ([`prompting_press::error::code`]). The TS facade `JSON.parse`s `error.message` (which napi
//! sets from `reason`), reads `.code` to pick the subclass, and exposes `.errors`.
//!
//! ## Why JSON-in-`reason` (research D4 sub-item)
//!
//! `napi::Error` carries only `status: Status` + `reason: String` + an optional `cause` — there
//! is **no** native slot for a structured Rust payload that survives to JS as data. The two
//! candidates were a custom `Status`/`reason` string or a JSON-encoded `reason` the TS side
//! parses; the JSON-encoded form is chosen because it keeps the **rows structured** (a real
//! `[{field,code,message}]` array on the JS side after `JSON.parse`), not flattened into a
//! human string. The rows are already scrubbed before they are encoded, so the JSON document
//! carries no kernel detail (SEC-004). `status` is left at the default `GenericFailure` — the
//! `code` inside the JSON, not the napi `Status`, is the discriminant.
//!
//! ## SEC-004 — never copy raw kernel detail
//!
//! A raw [`KernelError`] may carry bound-value content (secrets/PII) in its `Parse`/`Render`/
//! `ExcludedFeature` `detail`. [`kernel_error_to_napi_err`] therefore routes the kernel error
//! through the consumer's **existing, tested** `From<KernelError> for ConsumerError` scrubber
//! *first* (which discards `detail` and emits a fixed message), then encodes the resulting
//! (scrubbed) `ConsumerError` rows. Raw `KernelError::detail` is **never** read in this file.

use napi::{Error as NapiError, Status};
use serde::Serialize;

use prompting_press::error::code;
use prompting_press::{ConsumerError, FieldError as ConsumerFieldError};
use prompting_press_core::KernelError;

/// One normalized failure row, JSON-serialized into the napi error payload.
///
/// The Rust-side mirror of the consumer's [`prompting_press::FieldError`] and the cross-language
/// error contract `[{field, code, message}]` (Principle VII / C-06). It is **output-only**:
/// produced by the translation and serialized into the [`napi::Error`] `reason`, never decoded
/// back into Rust. The TS facade reconstructs an equivalent `FieldError` from the JSON.
#[derive(Debug, Serialize)]
struct FieldErrorPayload {
    /// The offending field or path; `""` when no single field applies.
    field: String,
    /// A stable code from the consumer's closed [`code`](prompting_press::error::code) vocabulary.
    code: String,
    /// A human-readable, **scrubbed** message safe to log.
    message: String,
}

impl From<ConsumerFieldError> for FieldErrorPayload {
    fn from(row: ConsumerFieldError) -> Self {
        Self {
            field: row.field,
            code: row.code,
            message: row.message,
        }
    }
}

/// The full structured payload encoded into the [`napi::Error`] `reason` (research D4).
///
/// `code` is the **top-level** discriminant the TS facade switches on to pick the `Error`
/// subclass (`validation` → `PromptValidationError`, `unknown_prompt` → `UnknownPromptError`,
/// `load` → `LoadError`, and any kernel code → `PromptRenderError`); `errors` is the structured
/// row array exposed as `exc.errors` on the JS side. Both are already scrubbed.
#[derive(Debug, Serialize)]
struct ErrorPayload {
    /// The top-level discriminant code (the subclass selector — see [`top_level_code`]).
    code: String,
    /// The structured, already-scrubbed `[{field, code, message}]` rows.
    errors: Vec<FieldErrorPayload>,
}

/// Translate a [`ConsumerError`] into a [`napi::Error`] carrying the scrubbed structured payload.
///
/// **Exhaustive** over the closed [`ConsumerError`] enum — no wildcard arm — so a new variant is a
/// compile error here until it is mapped (research D4: a new Rust variant must not silently fall
/// through). The payload `code` / rows are derived from the (already scrubbed) `ConsumerError`
/// data, never from raw kernel detail.
pub fn consumer_error_to_napi_err(err: ConsumerError) -> NapiError {
    let (top_code, rows): (&'static str, Vec<ConsumerFieldError>) = match err {
        ConsumerError::Validation(rows) => (code::VALIDATION, rows),
        ConsumerError::Kernel(rows) => {
            // The kernel rows already carry their specific code (unknown_variant /
            // undefined_variable / parse / render / excluded_feature). The top-level code uses
            // the first row's code when present, so the TS facade can route a kernel failure to
            // PromptRenderError while `.errors[*].code` keeps the precise per-row code. An empty
            // kernel row set (not produced by the consumer today) falls back to `render`.
            let top = rows.first().map_or(code::RENDER, |r| {
                // Borrow a `'static` code constant matching the row's value so the payload's
                // top-level `code` is a stable vocabulary string, not a copy. The consumer only
                // ever emits these five from a KernelError (error.rs `From<KernelError>`).
                kernel_top_code(&r.code)
            });
            (top, rows)
        }
        ConsumerError::UnknownPrompt(name) => {
            // A caller-supplied identifier (the key looked up), not bound-value content — safe to
            // surface, matching the Rust consumer's `Display`.
            let row = ConsumerFieldError {
                field: "name".to_string(),
                code: code::UNKNOWN_PROMPT.to_string(),
                message: format!("unknown prompt: `{name}`"),
            };
            (code::UNKNOWN_PROMPT, vec![row])
        }
        ConsumerError::Load(detail) => {
            // Loader serde detail is parse-location text (line/column / "missing field"), not
            // bound-value content — the consumer surfaces it, so the binding mirrors that.
            let row = ConsumerFieldError {
                field: String::new(),
                code: code::LOAD.to_string(),
                message: detail,
            };
            (code::LOAD, vec![row])
        }
    };

    let payload = ErrorPayload {
        code: top_code.to_string(),
        errors: rows.into_iter().map(FieldErrorPayload::from).collect(),
    };

    // JSON-encode the (scrubbed) payload into the napi error `reason`; the TS facade parses it.
    // `serde_json::to_string` over this owned, all-String payload is effectively infallible — but
    // on the impossible failure we still emit a fixed, value-free reason rather than panic across
    // the FFI boundary (and never the raw error detail).
    let reason = serde_json::to_string(&payload).unwrap_or_else(|_| {
        r#"{"code":"render","errors":[{"field":"","code":"render","message":"error encoding failed"}]}"#
            .to_string()
    });

    NapiError::new(Status::GenericFailure, reason)
}

/// Translate a **raw** [`KernelError`] into a [`napi::Error`] — SEC-004 safe.
///
/// Routes through the consumer's tested scrubber (`ConsumerError::from(kernel)`) **first**, which
/// replaces `Parse`/`Render`/`ExcludedFeature` detail with a fixed message and discards the raw
/// `detail`. The resulting (scrubbed) `ConsumerError` is then mapped by
/// [`consumer_error_to_napi_err`]. Raw `KernelError::detail` is never read here.
pub fn kernel_error_to_napi_err(err: KernelError) -> NapiError {
    let scrubbed = ConsumerError::from(err);
    consumer_error_to_napi_err(scrubbed)
}

/// Map a kernel-row `code` string back to its `'static` vocabulary constant for the payload's
/// top-level `code`.
///
/// The consumer's `From<KernelError>` only ever emits one of these five codes on a
/// `ConsumerError::Kernel` row; any other value (impossible from that path) falls back to
/// `render` so the TS facade still routes the failure to `PromptRenderError`.
fn kernel_top_code(row_code: &str) -> &'static str {
    match row_code {
        c if c == code::UNKNOWN_VARIANT => code::UNKNOWN_VARIANT,
        c if c == code::UNDEFINED_VARIABLE => code::UNDEFINED_VARIABLE,
        c if c == code::PARSE => code::PARSE,
        c if c == code::EXCLUDED_FEATURE => code::EXCLUDED_FEATURE,
        _ => code::RENDER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse the JSON `reason` napi carries (the payload the TS facade decodes).
    fn payload_of(err: &NapiError) -> serde_json::Value {
        serde_json::from_str(&err.reason).expect("napi error reason is the JSON payload")
    }

    /// SEC-004: a secret seeded into a raw `KernelError::Render` `detail` must NOT surface in the
    /// resulting napi error — not in `reason`, and not in any `errors[*]` row. The scrubber is
    /// exercised through the real translation path, not re-implemented.
    #[test]
    fn render_kernel_detail_secret_is_scrubbed() {
        const SECRET: &str = "sk-super-secret-token-9f8a7b6c5d4e";
        let kernel = KernelError::Render {
            detail: format!("failed to render value `{SECRET}` in loop"),
        };

        let err = kernel_error_to_napi_err(kernel);

        // The whole napi reason (what becomes the JS Error.message) must not leak the secret.
        assert!(
            !err.reason.contains(SECRET),
            "napi reason leaked the secret: {}",
            err.reason
        );

        let payload = payload_of(&err);
        // A kernel failure routes to the render code top-level (→ PromptRenderError in TS).
        assert_eq!(payload["code"], code::RENDER);
        let rows = payload["errors"].as_array().expect("errors array");
        assert_eq!(rows.len(), 1, "render error maps to one row");
        assert_eq!(rows[0]["field"], "template");
        assert_eq!(
            rows[0]["code"],
            code::RENDER,
            "the row carries the scrubbed render code"
        );
        let message = rows[0]["message"].as_str().expect("message string");
        assert!(
            !message.contains(SECRET),
            "errors[0].message leaked the secret: {message}"
        );
    }

    /// SEC-004: a `Parse` detail secret is likewise scrubbed; the row carries the `parse` code.
    #[test]
    fn parse_kernel_detail_secret_is_scrubbed() {
        const SECRET: &str = "PASSWORD=hunter2";
        let kernel = KernelError::Parse {
            detail: format!("syntax error near {SECRET}"),
        };
        let err = kernel_error_to_napi_err(kernel);
        assert!(
            !err.reason.contains(SECRET),
            "reason leaked: {}",
            err.reason
        );
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::PARSE);
        assert_eq!(payload["errors"][0]["code"], code::PARSE);
    }

    /// `ExcludedFeature` maps to the stable `excluded_feature` code and does not leak the raw
    /// detail (which names a template construct).
    #[test]
    fn excluded_feature_maps_without_leaking_detail() {
        const DETAIL: &str = "{% include 'secret-partial.txt' %} is not permitted";
        let kernel = KernelError::ExcludedFeature {
            detail: DETAIL.to_string(),
        };
        let err = kernel_error_to_napi_err(kernel);
        assert!(
            !err.reason.contains(DETAIL),
            "reason leaked: {}",
            err.reason
        );
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::EXCLUDED_FEATURE);
        assert_eq!(payload["errors"][0]["code"], code::EXCLUDED_FEATURE);
    }

    /// `UnknownVariant` surfaces the requested variant name (a caller-supplied identifier) and the
    /// `unknown_variant` code; the kernel failure still routes to that code top-level.
    #[test]
    fn unknown_variant_surfaces_name_with_stable_code() {
        let kernel = KernelError::UnknownVariant {
            requested: "concise".to_string(),
        };
        let err = kernel_error_to_napi_err(kernel);
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::UNKNOWN_VARIANT);
        assert_eq!(payload["errors"][0]["field"], "variant");
        assert_eq!(payload["errors"][0]["code"], code::UNKNOWN_VARIANT);
        let message = payload["errors"][0]["message"].as_str().unwrap();
        assert!(message.contains("concise"), "names the variant: {message}");
    }

    /// `UndefinedVariable` names the offending variable in `field` and maps to the stable code —
    /// the loud "missing referenced root" surface (never a silent empty render).
    #[test]
    fn undefined_variable_names_field() {
        let kernel = KernelError::UndefinedVariable {
            name: "article".to_string(),
        };
        let err = kernel_error_to_napi_err(kernel);
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::UNDEFINED_VARIABLE);
        assert_eq!(payload["errors"][0]["field"], "article");
        assert_eq!(payload["errors"][0]["code"], code::UNDEFINED_VARIABLE);
    }

    /// An `UnknownPrompt` consumer error maps to the `unknown_prompt` code and carries the name.
    #[test]
    fn unknown_prompt_maps_with_name() {
        let err = consumer_error_to_napi_err(ConsumerError::UnknownPrompt("greet".to_string()));
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::UNKNOWN_PROMPT);
        assert_eq!(payload["errors"][0]["code"], code::UNKNOWN_PROMPT);
        let message = payload["errors"][0]["message"].as_str().unwrap();
        assert!(message.contains("greet"), "names the prompt: {message}");
    }

    /// A `Load` consumer error maps to the `load` code.
    #[test]
    fn load_maps_to_load_code() {
        let err =
            consumer_error_to_napi_err(ConsumerError::Load("missing field `body`".to_string()));
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::LOAD);
        assert_eq!(payload["errors"][0]["code"], code::LOAD);
    }

    /// A garde-class `Validation` consumer error maps to the `validation` code, preserving every
    /// row's field name (the cross-language `[{field, code, message}]` contract).
    #[test]
    fn validation_maps_preserving_rows() {
        let rows = vec![
            ConsumerFieldError {
                field: "name".to_string(),
                code: code::VALIDATION.to_string(),
                message: "length is lower than 1".to_string(),
            },
            ConsumerFieldError {
                field: "count".to_string(),
                code: code::VALIDATION.to_string(),
                message: "greater than 100".to_string(),
            },
        ];
        let err = consumer_error_to_napi_err(ConsumerError::Validation(rows));
        let payload = payload_of(&err);
        assert_eq!(payload["code"], code::VALIDATION);
        let got = payload["errors"].as_array().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0]["field"], "name");
        assert_eq!(got[1]["field"], "count");
    }
}
