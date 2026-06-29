//! The Python exception hierarchy and the `ConsumerError`/`KernelError` → `PyErr`
//! translation (FR-014/FR-015, research D4 / C-06 / SEC-004).
//!
//! ## The contract
//!
//! Every fallible call on the Python public surface raises a [`PromptingPressError`] (or a
//! subtype). Native error types — the Rust [`ConsumerError`]/[`KernelError`] and (handled in
//! the render/compose modules) Pydantic's `ValidationError` — **never** appear on the Python
//! surface (C-06). The single structured payload is `.errors`: a list of [`FieldError`] rows,
//! each `{field, code, message}`, where `code` is the **same closed vocabulary** the Rust
//! consumer exposes ([`prompting_press::error::code`]).
//!
//! ## Why `create_exception!`, not `#[pyclass(extends=PyException)]`
//!
//! Research D4 lists two ways to build the base: a field-carrying `#[pyclass(extends=PyException)]`,
//! or `create_exception!`. The **first is incompatible with this crate's abi3 floor**: subclassing a
//! native exception type from a Rust `#[pyclass]` requires Python ≥ 3.12 under the `abi3` feature,
//! but the crate targets `abi3-py310` (floor 3.10) — the compiler rejects it outright. So the whole
//! hierarchy is built with [`create_exception!`], which mints the exception types through CPython's
//! C API and works on the 3.10 floor. The base subclasses `Exception`; each of the four subtypes
//! subclasses the base, giving Python `except PromptingPressError` (catch-all) and
//! `except PromptRenderError` (one class).
//!
//! ## How `.errors` reaches Python
//!
//! A `create_exception!` type cannot carry a typed Rust field. The structured payload is instead
//! attached as an **instance attribute** when the error is raised: [`raise_with_rows`] constructs
//! the exception with a fixed message, then sets `exc.errors` to a `list[FieldError]`. Python
//! exceptions accept arbitrary attributes, so callers read `exc.errors[0].field` exactly as the
//! contract specifies. [`FieldError`] is a plain read-only `#[pyclass]` (it does **not** extend a
//! native type, so it is abi3-safe).
//!
//! We deliberately do **not** override `__str__`/`__repr__` (research D4 / SEC-004): the default
//! `Exception.__str__` renders only the message arg we pass — a fixed, scrubbed summary — never the
//! raw rows or kernel detail.
//!
//! ## SEC-004 — never copy raw kernel detail
//!
//! A raw [`KernelError`] may carry bound-value content (secrets/PII) in its `Parse`/`Render`/
//! `ExcludedFeature` `detail`. [`kernel_error_to_pyerr`] therefore routes the kernel error through
//! the consumer's **existing, tested** `From<KernelError> for ConsumerError` scrubber *first* (which
//! discards `detail` and emits a fixed message), then maps the resulting `ConsumerError` rows. Raw
//! `KernelError::detail` is **never** read in this file.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::PyTypeInfo;

use prompting_press::error::code;
use prompting_press::{ConsumerError, FieldError as ConsumerFieldError};
use prompting_press_core::KernelError;

/// One normalized failure row, readable from Python as `row.field` / `row.code` / `row.message`.
///
/// The Python mirror of the consumer's [`prompting_press::FieldError`] — the cross-language error
/// contract `[{field, code, message}]` (Principle VII / C-06). Read-only (`frozen`): the getters
/// are the whole surface; rows are produced by the translation, never constructed from Python.
// `skip_from_py_object`: `FieldError` is output-only (the translation produces rows; Python
// never passes one *in*), so opt out of the `FromPyObject` derive that the `Clone` impl would
// otherwise pull in (which PyO3 0.29 deprecates as implicit).
#[pyclass(
    name = "FieldError",
    frozen,
    module = "prompting_press",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct FieldError {
    /// The offending field or path; `""` when no single field applies.
    #[pyo3(get)]
    pub field: String,
    /// A stable code from the consumer's closed [`code`](prompting_press::error::code) vocabulary.
    #[pyo3(get)]
    pub code: String,
    /// A human-readable, **scrubbed** message safe to log.
    #[pyo3(get)]
    pub message: String,
}

#[pymethods]
impl FieldError {
    /// `repr(row)` — a compact, fixed-shape rendering of the row's own (already scrubbed)
    /// normalized fields. This is contract content, not raw kernel detail.
    fn __repr__(&self) -> String {
        format!(
            "FieldError(field={:?}, code={:?}, message={:?})",
            self.field, self.code, self.message
        )
    }
}

impl From<ConsumerFieldError> for FieldError {
    fn from(row: ConsumerFieldError) -> Self {
        Self {
            field: row.field,
            code: row.code,
            message: row.message,
        }
    }
}

// The exception hierarchy. `create_exception!` mints CPython exception types (abi3-safe on the
// 3.10 floor). The base subclasses `Exception`; the four subtypes subclass the base, mirroring
// the closed `ConsumerError` variant set (research D4). `.errors` is attached per-instance at
// raise time (see `raise_with_rows`).

create_exception!(
    prompting_press,
    PromptingPressError,
    PyException,
    "Base for every Prompting Press error. Carries `.errors`: a list of FieldError rows \
     ({field, code, message}). `except PromptingPressError` catches all subtypes."
);

create_exception!(
    prompting_press,
    PromptValidationError,
    PromptingPressError,
    "Typed-Vars validation failed (code = \"validation\"). One row per offending field."
);

create_exception!(
    prompting_press,
    PromptRenderError,
    PromptingPressError,
    "A kernel render/source/analysis failure (code in unknown_variant|undefined_variable|parse|\
     render|excluded_feature). parse/render/excluded_feature messages are scrubbed (SEC-004)."
);

create_exception!(
    prompting_press,
    LoadError,
    PromptingPressError,
    "Malformed YAML/JSON or a shape violation in the dual-input loader (code = \"load\"). \
     Nothing is partially loaded."
);

/// Construct a `PyErr` of the exception type `E` (a `create_exception!` type) carrying `summary`
/// as the message and `rows` as the `.errors` instance attribute.
///
/// `summary` MUST already be scrubbed — callers derive it from the (scrubbed) rows / a
/// caller-supplied name, never from raw kernel detail.
fn raise_with_rows<E>(
    py: Python<'_>,
    rows: Vec<ConsumerFieldError>,
    summary: String,
) -> PyResult<PyErr>
where
    E: PyTypeInfo,
{
    // Materialize the structured rows as `list[FieldError]`.
    let py_rows: Vec<Py<FieldError>> = rows
        .into_iter()
        .map(|row| Py::new(py, FieldError::from(row)))
        .collect::<PyResult<_>>()?;

    // Instantiate the exception with the fixed summary as its single arg (so `str(exc)` shows
    // only that), then attach the structured payload as `exc.errors`.
    let exc_type = py.get_type::<E>();
    let exc = exc_type.call1((summary,))?;
    exc.setattr("errors", py_rows)?;
    Ok(PyErr::from_value(exc))
}

/// Translate a [`ConsumerError`] into the matching Python exception.
///
/// **Exhaustive** over the closed [`ConsumerError`] enum — no wildcard arm — so a new variant is a
/// compile error here until it is mapped to a subtype (research D4: a new Rust variant must not
/// silently fall through). The `summary` is derived from the (already scrubbed) rows / name, never
/// from raw kernel detail.
pub fn consumer_error_to_pyerr(py: Python<'_>, err: ConsumerError) -> PyErr {
    let result = match err {
        ConsumerError::Validation(rows) => {
            let summary = summarize("validation failed", &rows);
            raise_with_rows::<PromptValidationError>(py, rows, summary)
        }
        ConsumerError::Kernel(rows) => {
            let summary = summarize("render failed", &rows);
            raise_with_rows::<PromptRenderError>(py, rows, summary)
        }
        ConsumerError::Load(detail) => {
            // Loader serde detail is parse-location text (line/column / "missing field"), not
            // bound-value content — the consumer surfaces it, so the binding mirrors that.
            let row = ConsumerFieldError {
                field: String::new(),
                code: code::LOAD.to_string(),
                message: detail.clone(),
            };
            let summary = format!("failed to load prompt definition: {detail}");
            raise_with_rows::<LoadError>(py, vec![row], summary)
        }
    };

    // `raise_with_rows` only fails if allocating the Python objects fails (e.g. interpreter OOM);
    // PyO3 has a live Python error set in that case, so surface it rather than panic across FFI.
    result.unwrap_or_else(|e| e)
}

/// Translate a **raw** [`KernelError`] into a Python exception — SEC-004 safe.
///
/// Routes through the consumer's tested scrubber (`ConsumerError::from(kernel)`) **first**, which
/// replaces `Parse`/`Render`/`ExcludedFeature` detail with a fixed message and discards the raw
/// `detail`. The resulting (scrubbed) `ConsumerError` is then mapped by [`consumer_error_to_pyerr`].
/// Raw `KernelError::detail` is never read here.
pub fn kernel_error_to_pyerr(py: Python<'_>, err: KernelError) -> PyErr {
    let scrubbed = ConsumerError::from(err);
    consumer_error_to_pyerr(py, scrubbed)
}

/// Build a fixed, scrubbed one-line summary from a header plus the (already scrubbed) rows.
///
/// Only the rows' own `field`/`code`/`message` are used — those are the normalized, scrubbed
/// contract values, so this introduces no leak surface beyond what `.errors` already carries.
fn summarize(header: &str, rows: &[ConsumerFieldError]) -> String {
    let mut s = format!("{header} ({} error(s))", rows.len());
    for row in rows {
        s.push_str(&format!("; {}: {} [{}]", row.field, row.message, row.code));
    }
    s
}

/// Register the exception hierarchy + the [`FieldError`] row class on the module.
///
/// Python imports them as `prompting_press.PromptingPressError` etc.
pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FieldError>()?;
    m.add(
        "PromptingPressError",
        m.py().get_type::<PromptingPressError>(),
    )?;
    m.add(
        "PromptValidationError",
        m.py().get_type::<PromptValidationError>(),
    )?;
    m.add("PromptRenderError", m.py().get_type::<PromptRenderError>())?;
    m.add("LoadError", m.py().get_type::<LoadError>())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SEC-004: a secret seeded into a raw `KernelError::Render` `detail` must NOT surface in the
    /// resulting Python exception — not in `str(exc)`, not in `repr(exc)`, and not in any
    /// `exc.errors[*]` row (`field`/`code`/`message`). The scrubber is exercised through the real
    /// translation path, not re-implemented.
    #[test]
    fn render_kernel_detail_secret_is_scrubbed_into_python() {
        const SECRET: &str = "sk-super-secret-token-9f8a7b6c5d4e";
        Python::attach(|py| {
            let kernel = KernelError::Render {
                detail: format!("failed to render value `{SECRET}` in loop"),
            };

            let err = kernel_error_to_pyerr(py, kernel);
            let value = err.value(py);

            // It is a PromptRenderError (kernel failures map to the render subtype) ...
            assert!(
                value.is_instance_of::<PromptRenderError>(),
                "expected PromptRenderError, got {:?}",
                value.get_type().name().unwrap()
            );
            // ... which is a PromptingPressError.
            assert!(value.is_instance_of::<PromptingPressError>());

            // str(exc) and repr(exc) must not leak the secret.
            let as_str = value.str().expect("str(exc)").to_string();
            assert!(
                !as_str.contains(SECRET),
                "str(exc) leaked the secret: {as_str}"
            );
            let as_repr = value.repr().expect("repr(exc)").to_string();
            assert!(
                !as_repr.contains(SECRET),
                "repr(exc) leaked the secret: {as_repr}"
            );

            // exc.errors rows must not leak the secret, and must carry the scrubbed code.
            let errors = value.getattr("errors").expect("exc.errors");
            let rows: Vec<Bound<'_, PyAny>> = errors
                .try_iter()
                .expect("iterable")
                .collect::<PyResult<_>>()
                .expect("rows");
            assert_eq!(rows.len(), 1, "render error maps to one row");
            for row in &rows {
                let field: String = row.getattr("field").unwrap().extract().unwrap();
                let codev: String = row.getattr("code").unwrap().extract().unwrap();
                let message: String = row.getattr("message").unwrap().extract().unwrap();
                assert_eq!(field, "template");
                assert_eq!(codev, code::RENDER, "must carry the scrubbed render code");
                assert!(
                    !message.contains(SECRET),
                    "exc.errors message leaked the secret: {message}"
                );
            }
        });
    }

    /// A `Load` consumer error maps to `LoadError` with the `load` code.
    #[test]
    fn load_maps_to_subtype() {
        Python::attach(|py| {
            let err = consumer_error_to_pyerr(
                py,
                ConsumerError::Load("missing field `body`".to_string()),
            );
            let value = err.value(py);
            assert!(value.is_instance_of::<LoadError>());
            let errors = value.getattr("errors").unwrap();
            let rows: Vec<Bound<'_, PyAny>> =
                errors.try_iter().unwrap().collect::<PyResult<_>>().unwrap();
            let codev: String = rows[0].getattr("code").unwrap().extract().unwrap();
            assert_eq!(codev, code::LOAD);
        });
    }

    /// A garde-class `Validation` consumer error maps to `PromptValidationError`, preserving rows.
    #[test]
    fn validation_maps_to_subtype_preserving_rows() {
        Python::attach(|py| {
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
            let err = consumer_error_to_pyerr(py, ConsumerError::Validation(rows));
            let value = err.value(py);
            assert!(value.is_instance_of::<PromptValidationError>());
            let errors = value.getattr("errors").unwrap();
            let got: Vec<Bound<'_, PyAny>> =
                errors.try_iter().unwrap().collect::<PyResult<_>>().unwrap();
            assert_eq!(got.len(), 2);
            let f0: String = got[0].getattr("field").unwrap().extract().unwrap();
            assert_eq!(f0, "name");
        });
    }
}
