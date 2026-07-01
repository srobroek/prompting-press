//! Normalized error surface for the consumer crate (spec 003, FR-014/FR-015).
//!
//! [`ConsumerError`] + [`FieldError`] are the **only** error type on this crate's public
//! API. The two native sources — garde's [`garde::Report`] and the kernel's
//! [`prompting_press_core::KernelError`] — are normalized **here** via the [`From`] impls
//! below and **never leak** across the public boundary (constitution Principle VI / C-06;
//! FR-014).
//!
//! ## Closed code vocabulary (critique E2)
//!
//! Every [`FieldError::code`] is drawn from the stable, closed set documented on the
//! [`code`](crate::error::code) module constants. Consumers may match on these strings; they are part
//! of the crate's compatibility surface and will not silently change meaning.
//!
//! ## SEC-004 / FR-015 — detail scrubbing
//!
//! The kernel's [`prompting_press_core::KernelError::Parse`] /
//! [`prompting_press_core::KernelError::Render`] `detail` strings may embed
//! **bound-value content** (untrusted input / PII / secrets). The normalizer in
//! [`From<KernelError>`](ConsumerError) MUST NOT copy that raw `detail` into a
//! [`FieldError::message`]. Instead it emits a fixed, templated message plus a stable
//! `code`, so a secret embedded in a render-error value can never surface in the
//! [`ConsumerError`] string. See the unit tests at the bottom of this module.

use prompting_press_core::KernelError;

/// Stable, closed vocabulary of [`FieldError::code`] values.
///
/// These strings are a compatibility surface: a consumer may match on them to react to a
/// specific failure class. The set is intentionally small and closed — a new failure mode
/// reuses an existing code or adds one here deliberately (it is never synthesized ad hoc).
///
/// ## Why `code` is a `String`, not a Rust enum (TY-1, deliberate)
///
/// [`FieldError::code`] is a `String` drawn from this closed const vocabulary **on purpose**,
/// not a Rust enum. The `code` value crosses the `PyO3` / napi FFI boundary as a **string** in
/// the Python / TypeScript bindings: the `[{field, code, message}]` row is the cross-language
/// error contract (constitution Principle VII / roadmap decision C-06), and a string `code` is
/// what survives marshaling identically across all three bindings. A Rust-only enum would
/// diverge the Rust binding's `FieldError` shape from the Python/TS bindings' shape, breaking
/// that structural-parity contract. Consumers match on the documented const set (e.g.
/// `row.code == code::UNKNOWN_VARIANT`) rather than on enum variants.
pub mod code {
    /// A garde validation failure (the consumer synthesizes this; garde exposes no machine
    /// code). One row per reported path.
    pub const VALIDATION: &str = "validation";

    /// The kernel was asked for a variant the definition does not declare
    /// ([`prompting_press_core::KernelError::UnknownVariant`]).
    pub const UNKNOWN_VARIANT: &str = "unknown_variant";

    /// A strict-undefined variable was hit at render time
    /// ([`prompting_press_core::KernelError::UndefinedVariable`]).
    pub const UNDEFINED_VARIABLE: &str = "undefined_variable";

    /// The template failed to parse ([`prompting_press_core::KernelError::Parse`]). The
    /// underlying detail is **scrubbed** (SEC-004 / FR-015).
    pub const PARSE: &str = "parse";

    /// A render-time failure other than an undefined variable
    /// ([`prompting_press_core::KernelError::Render`]). The underlying detail is
    /// **scrubbed** (SEC-004 / FR-015).
    pub const RENDER: &str = "render";

    /// The template used an excluded feature
    /// ([`prompting_press_core::KernelError::ExcludedFeature`]).
    pub const EXCLUDED_FEATURE: &str = "excluded_feature";

    /// Malformed YAML/JSON input, or a deserialize failure, in the dual-input loader.
    pub const LOAD: &str = "load";
}

/// One normalized failure row — the common structured shape shared across every binding
/// (`[{field, code, message}]`).
///
/// - `field`: the offending field / path (a garde dot-path, a variable name, `"variant"`,
///   or `"template"`), or `""` when no single field applies.
/// - `code`: a stable string from the [`code`] vocabulary.
/// - `message`: a human-readable, **scrubbed** description safe to log (FR-015).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    /// The offending field or path; `""` when no single field applies.
    pub field: String,
    /// A stable code from the [`code`] vocabulary.
    pub code: String,
    /// A human-readable, scrubbed message safe to log.
    pub message: String,
}

/// The single public error type for the consumer crate (FR-014).
///
/// Native error types ([`garde::Report`], [`prompting_press_core::KernelError`]) are mapped
/// into this shape by the [`From`] impls in this module and never appear on a public
/// signature (C-06). The enum is **closed** (no `#[non_exhaustive]`): the variant set is
/// part of the compatibility surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumerError {
    /// Typed-Vars validation failed (garde). One [`FieldError`] per reported path, each
    /// carrying [`code::VALIDATION`].
    Validation(Vec<FieldError>),

    /// The kernel rejected the render/source/analysis call. One [`FieldError`] per kernel
    /// failure, carrying the mapped code from the [`code`] vocabulary. `Render` detail is
    /// scrubbed (FR-015); `Parse` detail is preserved (pre-binding template syntax).
    Kernel(Vec<FieldError>),

    /// Malformed input to the dual-input loader (bad YAML/JSON, or a deserialize error).
    /// Holds a short, loader-level description (FR-007). Nothing is partially loaded.
    Load(String),
}

impl std::fmt::Display for ConsumerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(rows) => {
                write!(f, "validation failed ({} error(s))", rows.len())?;
                for row in rows {
                    write!(f, "; {}: {} [{}]", row.field, row.message, row.code)?;
                }
                Ok(())
            }
            Self::Kernel(rows) => {
                write!(f, "kernel error ({} item(s))", rows.len())?;
                for row in rows {
                    write!(f, "; {}: {} [{}]", row.field, row.message, row.code)?;
                }
                Ok(())
            }
            Self::Load(detail) => write!(f, "failed to load prompt definition: {detail}"),
        }
    }
}

impl std::error::Error for ConsumerError {}

/// Normalize a garde [`Report`](garde::Report) into [`ConsumerError::Validation`].
///
/// garde 0.23's `Report` exposes `iter() -> impl Iterator<Item = &(Path, Error)>` (there is
/// **no** `flatten()`). Each pair becomes one [`FieldError`]:
/// `field = path.to_string()` (`Path: Display` → dot-path), `message = error.message()`,
/// and `code = code::VALIDATION` (garde carries no machine code, so the consumer
/// synthesizes a stable one).
impl From<garde::Report> for ConsumerError {
    fn from(report: garde::Report) -> Self {
        let rows = report
            .iter()
            .map(|(path, error)| FieldError {
                field: path.to_string(),
                code: code::VALIDATION.to_string(),
                message: error.message().to_string(),
            })
            .collect();
        Self::Validation(rows)
    }
}

/// Normalize the kernel's closed [`KernelError`] into [`ConsumerError::Kernel`].
///
/// The match is **exhaustive** over the closed enum (no wildcard arm), so adding a kernel
/// variant is a compile error here until it is mapped.
///
/// **SEC-004 / FR-015:** `Render` `detail` may carry bound-value content (untrusted input,
/// PII, secrets) — rendering is the only stage where values flow into the engine — so the
/// `Render` arm emits a **fixed, templated** message and discards the raw `detail` entirely;
/// the value content never reaches the `message` (and thus never reaches a log derived from
/// it). `Parse` is different: the engine parses the template source eagerly, *before* any
/// value is bound, so a parse error carries only template-syntax context (line/column, the
/// offending construct) and its `detail` is **preserved** for debuggability — no bound value
/// can be present. `UnknownVariant` / `UndefinedVariable` carry only a caller-supplied
/// variant name / a template variable name — neither is bound *value* content — so those are
/// surfaced. `ExcludedFeature` detail describes a template construct (a tag name), not a
/// bound value, but is templated for uniformity and defense in depth.
impl From<KernelError> for ConsumerError {
    fn from(err: KernelError) -> Self {
        let row = match err {
            KernelError::UnknownVariant { requested } => FieldError {
                field: "variant".to_string(),
                code: code::UNKNOWN_VARIANT.to_string(),
                message: format!("unknown variant: `{requested}`"),
            },
            KernelError::UndefinedVariable { name } => FieldError {
                field: name.clone(),
                code: code::UNDEFINED_VARIABLE.to_string(),
                message: format!("undefined variable at render: `{name}`"),
            },
            // Parsing is eager and happens BEFORE any value is bound (the engine parses the
            // template source, then renders with values). A `Parse` error therefore carries
            // only template-syntax context (line/column, the offending construct) — never a
            // bound value — so its detail is safe to surface and is preserved for debugging.
            KernelError::Parse { detail } => FieldError {
                field: "template".to_string(),
                code: code::PARSE.to_string(),
                message: format!("template parse error: {detail}"),
            },
            // SEC-004: `detail` may embed bound-value content — DO NOT copy it. Fixed message.
            KernelError::Render { detail: _ } => FieldError {
                field: "template".to_string(),
                code: code::RENDER.to_string(),
                message: "render error".to_string(),
            },
            // Detail names a template construct, not a bound value; templated for uniformity.
            KernelError::ExcludedFeature { detail: _ } => FieldError {
                field: "template".to_string(),
                code: code::EXCLUDED_FEATURE.to_string(),
                message: "template uses an excluded feature".to_string(),
            },
            // spec-015: raised when a caller-supplied advisory override fails validation.
            // The detail names the missing element(s) — no bound value content.
            KernelError::GuardAdvisoryInvalid { detail } => FieldError {
                field: "guard".to_string(),
                code: code::RENDER.to_string(),
                message: format!("guard advisory override is invalid: {detail}"),
            },
        };
        Self::Kernel(vec![row])
    }
}

impl ConsumerError {
    /// Normalize a [`KernelError`] into [`ConsumerError`], with an explicit opt-in to
    /// surface the render-error detail.
    ///
    /// ## Behavior
    ///
    /// - When `reveal_render_detail == false` **or** the error is any kind other than
    ///   [`KernelError::Render`]: produces the **exact same result** as
    ///   `ConsumerError::from(err)` — the scrubbing default (SEC-004 unchanged).
    /// - When `reveal_render_detail == true` **and** the error is
    ///   [`KernelError::Render { detail }`]: surfaces the real `detail` verbatim in the
    ///   returned [`FieldError::message`]. All other fields (`field`, `code`) are unchanged.
    ///
    /// ## Risk warning
    ///
    /// Enabling `reveal_render_detail = true` may place **bound-value content** — untrusted
    /// input, PII, secrets — into the returned error message and into any log line or stack
    /// trace derived from it. The caller is responsible for ensuring the error is handled in
    /// a context where such exposure is acceptable (e.g. a controlled debug session with a
    /// trusted log sink). Never enable this in production without deliberate review.
    ///
    /// ## When to use
    ///
    /// Pass `reveal_render_detail = false` everywhere except a deliberate, per-call debug
    /// render where you control the log destination and accept the bound-value exposure risk.
    /// The canonical production call site is always `ConsumerError::from(err)` (or this
    /// function with `false`), which keeps SEC-004 intact.
    #[must_use]
    pub fn from_kernel_revealing(err: KernelError, reveal_render_detail: bool) -> Self {
        // Only the Render arm with reveal=true differs from the scrubbing default.
        if reveal_render_detail {
            if let KernelError::Render { detail } = err {
                return Self::Kernel(vec![FieldError {
                    field: "template".to_string(),
                    code: code::RENDER.to_string(),
                    // Surfaced verbatim — the caller has opted in and accepted responsibility.
                    message: detail,
                }]);
            }
        }
        // All other cases: byte-for-byte identical to the From impl (scrubbing default).
        Self::from(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SEC-004 / FR-015: a secret embedded in a `Render` error `detail` must NOT surface in
    /// the normalized `ConsumerError` (neither in a `FieldError::message` nor in the
    /// `Display` string a log line would derive from it).
    #[test]
    fn render_detail_secret_is_scrubbed() {
        const SECRET: &str = "sk-super-secret-token-9f8a7b6c5d4e";
        let kernel = KernelError::Render {
            detail: format!("failed to render value `{SECRET}` in loop"),
        };

        let normalized = ConsumerError::from(kernel);

        // The structured rows must not contain the secret.
        match &normalized {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].field, "template");
                assert_eq!(rows[0].code, code::RENDER);
                assert!(
                    !rows[0].message.contains(SECRET),
                    "render message leaked the secret: {:?}",
                    rows[0].message
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }

        // The Display string (what a log line derives from) must not contain the secret.
        assert!(
            !normalized.to_string().contains(SECRET),
            "Display leaked the secret: {normalized}"
        );
    }

    /// `Parse` detail is PRESERVED (not scrubbed): parsing happens before any value is
    /// bound, so the detail carries only template-syntax context — never a bound value —
    /// and surfacing it is what makes a syntax error debuggable.
    #[test]
    fn parse_detail_is_preserved_for_debuggability() {
        const DETAIL: &str = "syntax error: unexpected end of input (in kernel:1)";
        let kernel = KernelError::Parse {
            detail: DETAIL.to_string(),
        };

        let normalized = ConsumerError::from(kernel);

        match &normalized {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows[0].field, "template");
                assert_eq!(rows[0].code, code::PARSE);
                assert!(
                    rows[0].message.contains(DETAIL),
                    "parse detail must be preserved: {:?}",
                    rows[0].message
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    /// `UnknownVariant` surfaces the requested name (it is a caller-supplied identifier, not
    /// bound value content) and maps to the stable code.
    #[test]
    fn unknown_variant_surfaces_name_with_stable_code() {
        let kernel = KernelError::UnknownVariant {
            requested: "concise".to_string(),
        };
        let normalized = ConsumerError::from(kernel);
        match normalized {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows[0].field, "variant");
                assert_eq!(rows[0].code, code::UNKNOWN_VARIANT);
                assert!(rows[0].message.contains("concise"));
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    /// TS-3 / SEC-004 — `ExcludedFeature` maps to the stable `code::EXCLUDED_FEATURE` and the
    /// raw `detail` (which names a template construct) is NOT copied into the message; a fixed,
    /// templated message is emitted instead (defense in depth — module docs).
    #[test]
    fn excluded_feature_maps_to_stable_code_without_leaking_detail() {
        const DETAIL: &str = "{% include 'secret-partial.txt' %} is not permitted";
        let kernel = KernelError::ExcludedFeature {
            detail: DETAIL.to_string(),
        };
        let normalized = ConsumerError::from(kernel);
        match &normalized {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].field, "template");
                assert_eq!(
                    rows[0].code,
                    code::EXCLUDED_FEATURE,
                    "must map to excluded_feature"
                );
                assert!(
                    !rows[0].message.contains(DETAIL),
                    "the raw detail must not leak into the message: {:?}",
                    rows[0].message
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
        // The Display string (what a log line derives from) must not carry the raw detail.
        assert!(
            !normalized.to_string().contains(DETAIL),
            "Display leaked the raw detail: {normalized}"
        );
    }

    /// `UndefinedVariable` names the offending variable in `field` and maps to the stable
    /// code.
    #[test]
    fn undefined_variable_names_field() {
        let kernel = KernelError::UndefinedVariable {
            name: "article".to_string(),
        };
        let normalized = ConsumerError::from(kernel);
        match normalized {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows[0].field, "article");
                assert_eq!(rows[0].code, code::UNDEFINED_VARIABLE);
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    // ── from_kernel_revealing (spec 013 T001) ─────────────────────────────────

    /// Helper: build a fresh Render `KernelError` with the given detail string.
    fn make_render(detail: &str) -> KernelError {
        KernelError::Render {
            detail: detail.to_string(),
        }
    }

    /// (a) reveal=false is byte-for-byte identical to From for ALL arms.
    #[test]
    fn from_kernel_revealing_false_equals_from_render() {
        let detail = "secret-detail-value";
        assert_eq!(
            ConsumerError::from(make_render(detail)),
            ConsumerError::from_kernel_revealing(make_render(detail), false),
            "reveal=false must equal From for Render"
        );
    }

    #[test]
    fn from_kernel_revealing_false_equals_from_parse() {
        let k = || KernelError::Parse {
            detail: "syntax error at line 1".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), false),
            "reveal=false must equal From for Parse"
        );
    }

    #[test]
    fn from_kernel_revealing_false_equals_from_excluded_feature() {
        let k = || KernelError::ExcludedFeature {
            detail: "{% include %}".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), false),
            "reveal=false must equal From for ExcludedFeature"
        );
    }

    #[test]
    fn from_kernel_revealing_false_equals_from_unknown_variant() {
        let k = || KernelError::UnknownVariant {
            requested: "concise".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), false),
            "reveal=false must equal From for UnknownVariant"
        );
    }

    #[test]
    fn from_kernel_revealing_false_equals_from_undefined_variable() {
        let k = || KernelError::UndefinedVariable {
            name: "payload".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), false),
            "reveal=false must equal From for UndefinedVariable"
        );
    }

    /// (b) reveal=true for Render surfaces the real detail verbatim.
    #[test]
    fn from_kernel_revealing_true_surfaces_render_detail_verbatim() {
        const DETAIL: &str = "failed to render value `sk-secret-abc` in loop";
        let kernel = KernelError::Render {
            detail: DETAIL.to_string(),
        };

        let revealed = ConsumerError::from_kernel_revealing(kernel, true);

        match &revealed {
            ConsumerError::Kernel(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].field, "template");
                assert_eq!(rows[0].code, code::RENDER);
                assert_eq!(
                    rows[0].message, DETAIL,
                    "reveal=true must surface detail verbatim"
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    /// (c) reveal=true for Parse is identical to the scrubbing/from path (reveal does NOT
    /// change Parse — parse detail is already preserved by the From impl).
    #[test]
    fn from_kernel_revealing_true_does_not_change_parse_arm() {
        const DETAIL: &str = "syntax error: unexpected end of input";
        let k = || KernelError::Parse {
            detail: DETAIL.to_string(),
        };

        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), true),
            "reveal=true must not change the Parse arm (parse detail already preserved)"
        );
    }

    /// reveal=true for `ExcludedFeature` is identical to the scrubbing default.
    #[test]
    fn from_kernel_revealing_true_does_not_change_excluded_feature() {
        let k = || KernelError::ExcludedFeature {
            detail: "{% include %}".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), true),
            "reveal=true must not change ExcludedFeature"
        );
    }

    /// reveal=true for `UnknownVariant` is identical to the scrubbing default.
    #[test]
    fn from_kernel_revealing_true_does_not_change_unknown_variant() {
        let k = || KernelError::UnknownVariant {
            requested: "concise".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), true),
            "reveal=true must not change UnknownVariant"
        );
    }

    /// reveal=true for `UndefinedVariable` is identical to the scrubbing default.
    #[test]
    fn from_kernel_revealing_true_does_not_change_undefined_variable() {
        let k = || KernelError::UndefinedVariable {
            name: "payload".to_string(),
        };
        assert_eq!(
            ConsumerError::from(k()),
            ConsumerError::from_kernel_revealing(k(), true),
            "reveal=true must not change UndefinedVariable"
        );
    }
}
