//! The Python render helpers — the [`RenderResult`] and [`GuardConfig`] pyclasses, plus the
//! [`validate_in_python`] helper shared by `prompt.rs` and `compose.rs` (FR-002, FR-009..011).
//!
//! ## Spec 008 Phase 4 note
//!
//! The pre-reshape module-level `render(reg, name, vars, ...)` and `get_source(reg, name, ...)`
//! free functions are removed. Render and source lookup are now methods on
//! [`Prompt`](crate::prompt::Prompt): `prompt.render(...)` and `prompt.get_source(...)`.
//!
//! This module retains:
//! - The [`RenderResult`] and [`GuardConfig`] pyclasses (Python-visible output / config types).
//! - The [`validate_in_python`] helper: owns Pydantic validation in Python (FR-002 / Q1), shared
//!   by `prompt.rs` and `compose.rs`.
//! - The SEC-004-PY Pydantic error scrubbing helpers.
//!
//! ## Why the kernel is called DIRECTLY (critique E1 / C-01)
//!
//! `prompt.rs` calls [`prompting_press_core::render`] directly — not the Rust consumer's
//! `prompting_press::render` which is generic over `V: Serialize + Validate` (a *garde* type).
//! Validation is owned in Python (the caller's Pydantic Vars model), so after validating in
//! Python and marshaling, the kernel is reached directly. That is still **zero engine logic**
//! (Principle I): the kernel *is* the shared core; the binding only marshals into it.
//!
//! ## SEC-004-PY — Pydantic error scrubbing
//!
//! A `pydantic.ValidationError` row carries `input` (the rejected value — possibly a secret/PII)
//! and `ctx` (validator-supplied context). The mapper [`validation_error_to_pyerr`] copies
//! **only** the `msg` field into the normalized row's `message`, plus the `loc` path into
//! `field`. `input`/`ctx` are never read, so a secret in the rejected value cannot leak onto the
//! Python error surface.

use pyo3::prelude::*;

use prompting_press::error::code;
use prompting_press::{ConsumerError, FieldError as ConsumerFieldError};
use prompting_press_core::{GuardConfig as KernelGuardConfig, RenderResult as KernelRenderResult};

use crate::error::consumer_error_to_pyerr;

/// The opt-in guard-expansion config, surfaced to Python and **plumbed through** to the kernel
/// (FR-009).
///
/// A 1:1 mirror of the kernel's [`prompting_press_core::GuardConfig`] — `enabled` only (spec 015
/// removed the custom `template` override; the guard wording is now fixed). This pyclass is
/// **config only**; it carries no logic. The kernel owns guard *expansion* (spec 015 / FR-022..025);
/// the binding only marshals `enabled` across the boundary and surfaces whatever
/// [`RenderResult::guard`] the kernel populates. Read-only after construction (`frozen`): build
/// it once via the constructor.
// `skip_from_py_object`: it is constructed by `#[new]` and read by-ref in `render`'s signature
// (PyO3 extracts an `Option<&GuardConfig>` from the pyclass registry directly), never via a
// `FromPyObject` derive — so opt out of the implicit derive PyO3 0.29 would otherwise pull in.
#[pyclass(
    name = "GuardConfig",
    frozen,
    module = "prompting_press",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct GuardConfig {
    /// When `False`, the render is plain and [`RenderResult::guard`] is `None`.
    #[pyo3(get)]
    pub enabled: bool,
}

#[pymethods]
impl GuardConfig {
    /// `GuardConfig(enabled=False)` — defaults match a disabled guard, so
    /// `GuardConfig()` is equivalent to passing no guard at all.
    #[new]
    #[pyo3(signature = (*, enabled=false))]
    fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// `repr(guard)` — fixed-shape.
    fn __repr__(&self) -> String {
        format!(
            "GuardConfig(enabled={})",
            if self.enabled { "True" } else { "False" }
        )
    }
}

impl From<&GuardConfig> for KernelGuardConfig {
    fn from(g: &GuardConfig) -> Self {
        Self { enabled: g.enabled }
    }
}

/// A rendered prompt + its content-addressed provenance, read-only from Python.
///
/// The Python mirror of the kernel's [`prompting_press_core::RenderResult`] (data-model
/// §RenderResult; FR-015). Surfaced **1:1** — the binding adds nothing and interprets nothing.
/// Read-only (`frozen`): a result is produced by [`render`], never constructed from Python.
// `skip_from_py_object`: output-only — Python reads the getters, never passes a `RenderResult`
// *in* — so opt out of the implicit `FromPyObject` derive PyO3 0.29 would otherwise pull in.
#[pyclass(
    name = "RenderResult",
    frozen,
    module = "prompting_press",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct RenderResult {
    /// The rendered body text (FR-001). The guard text is NEVER concatenated here.
    #[pyo3(get)]
    pub text: String,
    /// The prompt name.
    #[pyo3(get)]
    pub name: String,
    /// The resolved variant name (the reserved `default`, or the named arm).
    #[pyo3(get)]
    pub variant: String,
    /// Lowercase-hex `SHA256(resolved variant source)` (FR-012).
    #[pyo3(get)]
    pub template_hash: String,
    /// Lowercase-hex `SHA256(rendered text)` (FR-013).
    #[pyo3(get)]
    pub render_hash: String,
    /// The opt-in guard instruction text (present iff a guard was enabled and the prompt declares
    /// an untrusted/external field); `None` for a plain render. Never part of `text`.
    #[pyo3(get)]
    pub guard: Option<String>,
}

#[pymethods]
impl RenderResult {
    /// `repr(result)` — a compact, fixed-shape rendering. The hashes content-address the render,
    /// so they are safe to surface; `text`/`guard` are the caller's own (already-rendered) output.
    fn __repr__(&self) -> String {
        format!(
            "RenderResult(name={:?}, variant={:?}, template_hash={:?}, render_hash={:?})",
            self.name, self.variant, self.template_hash, self.render_hash
        )
    }
}

impl From<KernelRenderResult> for RenderResult {
    fn from(r: KernelRenderResult) -> Self {
        Self {
            text: r.text,
            name: r.name,
            variant: r.variant,
            template_hash: r.template_hash,
            render_hash: r.render_hash,
            guard: r.guard,
        }
    }
}

/// Validate `vars` in Python, then render `name`'s resolved variant through the kernel (FR-009).
/// Validate `vars`/`data` in Python and return the validated payload, dumped with
/// `model_dump(mode="json")` (Q1; SEC-004-PY).
///
/// On a `pydantic.ValidationError` this raises [`PromptValidationError`](crate::error) (mapping
/// each row's `loc` → `field` and `msg` → `message`, never `input`/`ctx`). Any *other* Python
/// exception (e.g. the object has no `model_validate` — not a Pydantic model) is surfaced as-is:
/// it is a caller-API misuse, not a validation failure, so it must not masquerade as one.
///
/// `pub(crate)` so the [`compose`](crate::compose) module reuses the **exact same**
/// validate-then-dump path for each composition entry (option (a): eager-validate at `append`).
/// Sharing this function — rather than re-deriving validation — keeps validation owned in one
/// place (Q1) and guarantees a composed entry is validated identically to a single `render`.
pub(crate) fn validate_in_python<'py>(
    py: Python<'py>,
    vars: &Bound<'py, PyAny>,
    data: Option<&Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyAny>> {
    // Pick the model class + the value to validate:
    //   data given  → `vars` is the class, validate `data`.
    //   data is None → `vars` is an instance, validate `type(vars).model_validate(vars.dump())`.
    let (model_cls, to_validate): (Bound<'py, PyAny>, Bound<'py, PyAny>) = match data {
        Some(d) => (vars.clone(), d.clone()),
        None => {
            let cls = vars.get_type().into_any();
            let dumped = dump_json(vars)?;
            (cls, dumped)
        }
    };

    // model_cls.model_validate(to_validate) — the one validation pass (FR-002). A
    // ValidationError is normalized to PromptValidationError; the validated model is then
    // re-dumped to a JSON-mode payload for marshaling.
    match model_cls.call_method1("model_validate", (to_validate,)) {
        Ok(validated) => dump_json(&validated),
        Err(err) => {
            if is_pydantic_validation_error(py, &err) {
                Err(validation_error_to_pyerr(py, &err))
            } else {
                // Not a validation failure (e.g. not a Pydantic model at all) — surface as-is.
                Err(err)
            }
        }
    }
}

/// `obj.model_dump(mode="json")` — the JSON-primitive payload the marshaler consumes (research
/// D2: `mode="json"` stringifies `datetime`/`Decimal` deterministically).
fn dump_json<'py>(obj: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let kwargs = pyo3::types::PyDict::new(obj.py());
    kwargs.set_item("mode", "json")?;
    obj.call_method("model_dump", (), Some(&kwargs))
}

/// `True` iff `err` is (an instance of) `pydantic.ValidationError`.
///
/// Pydantic's `ValidationError` is the only error class whose `.errors()` rows we map. Importing
/// pydantic lazily (only on the error path) keeps the binding from requiring pydantic at import
/// time for callers who never hit a validation failure; a missing/old pydantic simply means this
/// returns `false` and the original error is surfaced unchanged.
fn is_pydantic_validation_error(py: Python<'_>, err: &PyErr) -> bool {
    let Ok(module) = py.import("pydantic") else {
        return false;
    };
    let Ok(ve) = module.getattr("ValidationError") else {
        return false;
    };
    err.get_type(py).is(&ve) || err.value(py).is_instance(&ve).unwrap_or(false)
}

/// Map a `pydantic.ValidationError` to a [`PromptValidationError`](crate::error) (SEC-004-PY).
///
/// Reads `err.value.errors()` and copies **only** `loc` (joined by `.`) → `field` and `msg` →
/// `message`; `input` and `ctx` are never read, so a secret in the rejected value cannot leak.
/// `code` is the consumer's stable `"validation"`. If `.errors()` cannot be introspected (e.g. an
/// unexpected pydantic shape), we DISCARD detail and raise a fixed-message `PromptValidationError`
/// with zero rows — we never surface the raw `pydantic.ValidationError`, whose `str()`/`errors()`
/// embed the rejected `input_value` (SEC-004-PY: the scrub must hold by construction, not surface a
/// native type on the degenerate path — security review M-1).
fn validation_error_to_pyerr(py: Python<'_>, err: &PyErr) -> PyErr {
    let value = err.value(py);
    let rows = match collect_validation_rows(value.as_any()) {
        Ok(rows) => rows,
        // Could not introspect the ValidationError — withhold detail (a fixed, value-free row)
        // rather than leak the raw pydantic error, which embeds the rejected input value.
        Err(_) => vec![ConsumerFieldError {
            field: String::new(),
            code: code::VALIDATION.to_string(),
            message: "input validation failed (error detail withheld)".to_string(),
        }],
    };
    consumer_error_to_pyerr(py, ConsumerError::Validation(rows))
}

/// Pull `[{field, code, message}]` rows out of a `pydantic.ValidationError` instance.
///
/// `msg` only (SEC-004-PY) — `input`/`ctx` are deliberately not read.
fn collect_validation_rows(value: &Bound<'_, PyAny>) -> PyResult<Vec<ConsumerFieldError>> {
    let errors = value.call_method0("errors")?;
    let mut rows = Vec::new();
    for item in errors.try_iter()? {
        let item = item?;
        let field = item
            .get_item("loc")
            .ok()
            .map(|loc| join_loc(&loc))
            .unwrap_or_default();
        let message: String = item
            .get_item("msg")
            .and_then(|m| m.extract())
            .unwrap_or_else(|_| "validation error".to_string());
        rows.push(ConsumerFieldError {
            field,
            code: code::VALIDATION.to_string(),
            message,
        });
    }
    Ok(rows)
}

/// Join a Pydantic `loc` tuple (`("a", 0, "b")`) into a dotted path (`a.0.b`).
///
/// `loc` is a tuple of `str | int`; each element is stringified through Python's `str()` (so an
/// `int` index becomes its decimal form) and the parts are joined with `.`. A non-iterable `loc`
/// (unexpected) falls back to stringifying the whole value.
fn join_loc(loc: &Bound<'_, PyAny>) -> String {
    match loc.try_iter() {
        Ok(it) => it
            .flatten()
            .map(|p| part_to_string(&p))
            .collect::<Vec<_>>()
            .join("."),
        Err(_) => part_to_string(loc),
    }
}

/// One `loc` element → its string form via Python's `str()` (str stays as-is; int → decimal).
fn part_to_string(part: &Bound<'_, PyAny>) -> String {
    part.str()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    //! Render-path coverage that is drivable in Rust WITHOUT a Pydantic model.
    //!
    //! The validate-then-render behavior (a real `@field_validator`, `PromptValidationError` on
    //! invalid input, "no render happened") needs a Python Pydantic model and is covered
    //! Python-side in T009. Here we exercise the kernel-direct render + the `KernelError` →
    //! Python exception mapping that the pyfn delegates to — the parts that need no Pydantic.

    use super::*;
    use prompting_press::PromptDefinition;
    use pyo3::types::PyDict;

    use crate::error::{kernel_error_to_pyerr, PromptRenderError};
    use crate::marshal::to_kernel_value;

    /// Build a `PromptDefinition` from JSON (the idiomatic in-test construction the consumer's
    /// own tests use — the generated newtypes validate, so a struct literal is awkward).
    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    /// The happy path the pyfn's tail performs: a marshaled value map → `prompting_press_core::
    /// render` (DIRECTLY — critique E1) → `RenderResult::from`. Asserts the rendered text and
    /// that both provenance hashes are 64-char lowercase hex (FR-012/FR-013).
    #[test]
    fn kernel_direct_render_produces_text_and_hex_hashes() {
        Python::attach(|py| {
            let def = def_from_json(
                r#"{ "name": "greet", "role": "user", "body": "Hello {{ name }}!" }"#,
            );

            // Marshal the values through the SAME bridge the pyfn uses (a Python dict, dumped
            // payload stand-in), so the value handed to the kernel is built identically.
            let data = PyDict::new(py);
            data.set_item("name", "Ada").expect("set name");
            let values = to_kernel_value(data.as_any()).expect("marshals");

            let kernel =
                prompting_press_core::render(&def, None, values, &KernelGuardConfig::default())
                    .expect("render succeeds");
            let result = RenderResult::from(kernel);

            assert_eq!(result.text, "Hello Ada!");
            assert_eq!(result.name, "greet");
            assert_eq!(
                result.variant, "default",
                "no variant ⇒ reserved default arm"
            );
            assert!(
                result.guard.is_none(),
                "default guard config ⇒ no guard text"
            );

            for (label, hash) in [
                ("template_hash", &result.template_hash),
                ("render_hash", &result.render_hash),
            ] {
                assert_eq!(hash.len(), 64, "{label} must be 64 hex chars, got {hash:?}");
                assert!(
                    hash.chars()
                        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "{label} must be lowercase hex, got {hash:?}"
                );
            }
        });
    }

    /// `RenderResult` surfaces the kernel result 1:1, including the resolved NAMED variant.
    #[test]
    fn named_variant_is_surfaced() {
        Python::attach(|py| {
            let def = def_from_json(
                r#"{
                    "name": "greet",
                    "role": "user",
                    "body": "default {{ name }}",
                    "variants": { "formal": { "body": "Good day, {{ name }}." } }
                }"#,
            );
            let data = PyDict::new(py);
            data.set_item("name", "Ada").expect("set name");
            let values = to_kernel_value(data.as_any()).expect("marshals");

            let kernel = prompting_press_core::render(
                &def,
                Some("formal"),
                values,
                &KernelGuardConfig::default(),
            )
            .expect("render formal");
            let result = RenderResult::from(kernel);

            assert_eq!(result.text, "Good day, Ada.");
            assert_eq!(result.variant, "formal");
        });
    }

    /// **Three-sets gap (critique E1 / spec assumptions).** A value map missing a
    /// template-referenced root drives the kernel's strict-undefined path. Routed through the
    /// binding's `kernel_error_to_pyerr`, it must surface as a `PromptRenderError` carrying the
    /// `undefined_variable` code — a LOUD error, never a silent empty render.
    #[test]
    fn missing_root_is_loud_undefined_variable() {
        Python::attach(|py| {
            let def = def_from_json(
                r#"{ "name": "greet", "role": "user", "body": "Hello {{ name }}!" }"#,
            );

            // The value map lacks `name` — the struct↔variables field-name agreement is the
            // caller's responsibility; a miss is NOT silent (it hits strict-undefined).
            let empty = PyDict::new(py);
            let values = to_kernel_value(empty.as_any()).expect("marshals");

            let kernel_err =
                prompting_press_core::render(&def, None, values, &KernelGuardConfig::default())
                    .expect_err("missing root ⇒ strict-undefined kernel error");

            let pyerr = kernel_error_to_pyerr(py, kernel_err);
            let value = pyerr.value(py);

            assert!(
                value.is_instance_of::<PromptRenderError>(),
                "a render-time kernel error maps to PromptRenderError, got {:?}",
                value.get_type().name().unwrap()
            );

            let errors = value.getattr("errors").expect("exc.errors");
            let rows: Vec<Bound<'_, PyAny>> = errors
                .try_iter()
                .expect("iterable")
                .collect::<PyResult<_>>()
                .expect("rows");
            assert_eq!(rows.len(), 1, "one undefined-variable row");
            let codev: String = rows[0].getattr("code").unwrap().extract().unwrap();
            assert_eq!(
                codev,
                code::UNDEFINED_VARIABLE,
                "a missing referenced root is a loud undefined_variable, not an empty render"
            );
        });
    }

    /// **Guard plumb-through (FR-009).** The binding's `GuardConfig` is converted to the kernel's
    /// and passed through unchanged: an enabled guard on a prompt that declares an untrusted field
    /// ⇒ `RenderResult.guard` is `Some(...)`; a default (disabled) guard ⇒ `None`. This asserts
    /// only that the field is *surfaced vs not* — the guard-text content/logic is the kernel's
    /// (spec 002) and is NOT re-tested here.
    #[test]
    fn guard_config_is_plumbed_through() {
        Python::attach(|py| {
            // A prompt declaring an untrusted variable (trusted: false), so the guard has
            // something to delimit.
            let def = def_from_json(
                r#"{
                    "name": "ask",
                    "role": "user",
                    "body": "Answer: {{ q }}",
                    "variables": { "q": { "type": "string", "trusted": false } }
                }"#,
            );

            let make_values = || {
                let d = PyDict::new(py);
                d.set_item("q", "hello").expect("set q");
                to_kernel_value(d.as_any()).expect("marshals")
            };

            // Enabled guard (built via the binding pyclass → kernel `From`, the SAME conversion
            // `render` performs) ⇒ guard text present.
            let enabled = GuardConfig::new(true);
            let kernel_cfg = KernelGuardConfig::from(&enabled);
            let with_guard = prompting_press_core::render(&def, None, make_values(), &kernel_cfg)
                .map(RenderResult::from)
                .expect("render with guard");
            assert!(
                with_guard.guard.is_some(),
                "an enabled guard on a prompt with an untrusted field must surface guard text"
            );
            // Spec 015: untrusted values are wrapped in <untrusted>…</untrusted> in the rendered
            // body.
            assert!(
                with_guard.text.contains("<untrusted>"),
                "enabled guard must wrap untrusted values in the rendered body text"
            );

            // Default (disabled) guard ⇒ no guard text.
            let plain = prompting_press_core::render(
                &def,
                None,
                make_values(),
                &KernelGuardConfig::default(),
            )
            .map(RenderResult::from)
            .expect("render plain");
            assert!(
                plain.guard.is_none(),
                "a default/disabled guard must leave RenderResult.guard as None"
            );
        });
    }

    /// `get_source` via the kernel directly — the `Prompt` object surface (spec 008 Phase 4).
    /// The registry-based `get_source(reg, name, variant)` free function is removed; the Rust
    /// test uses `prompting_press_core::get_source` directly as `prompt.rs` does.
    #[test]
    fn get_source_returns_unrendered_source() {
        Python::attach(|_py| {
            let def = def_from_json(
                r#"{ "name": "greet", "role": "user", "body": "Hello {{ name }}!" }"#,
            );

            let src = prompting_press_core::get_source(&def, None).expect("source");
            assert_eq!(
                src, "Hello {{ name }}!",
                "source is UNrendered (no interpolation)"
            );

            // Unknown variant ⇒ KernelError.
            let err = prompting_press_core::get_source(&def, Some("absent"))
                .expect_err("unknown variant → kernel error");
            assert!(
                matches!(
                    err,
                    prompting_press_core::KernelError::UnknownVariant { .. }
                ),
                "unknown variant must be a KernelError::UnknownVariant, got {err:?}"
            );
        });
    }
}
