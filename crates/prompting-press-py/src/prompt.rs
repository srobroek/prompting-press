//! The Python [`Prompt`] pyclass — the primary public type for spec 008 Phase 4 (T035–T038).
//!
//! ## Design
//!
//! [`Prompt`] is the immutable, fully-validated prompt object that replaces the
//! `(Registry, name)` split previously required for every operation. It mirrors the
//! Rust consumer's [`prompting_press::Prompt`] shape exactly, but delegates
//! **everything** — construction, render, check, source lookup, and mutation — to the
//! Rust consumer via the [`prompting_press`] crate. The binding adds only:
//!
//! - PyO3 extraction of the `shape` argument (Pydantic model OR dict → JSON → consumer
//!   constructor, reusing the duck-typing path from `registry.rs`).
//! - The `validators` coverage check (T036): if any `PromptVariable` carries
//!   `validation_required = true`, the supplied Pydantic model class must declare that
//!   field (introspected via `model.model_fields`). This is the Python-specific
//!   runtime enforcement of Principle VI v1.2.0.
//! - Storing a bound Pydantic model class (or `None`) for `render`.
//! - Marshaling validated Pydantic vars → [`to_kernel_value`] (reusing the same bridge
//!   the old `render.rs` uses).
//! - Wrapping errors via [`consumer_error_to_pyerr`] / [`kernel_error_to_pyerr`]
//!   (SEC-004 scrub preserved throughout).
//!
//! ## validators kwarg — a single Pydantic model CLASS (not a map)
//!
//! The spec contract says `validators: ValidatorMap | None`. In Python the natural
//! shape is a **single Pydantic `BaseModel` subclass** whose `model_fields` covers the
//! variables that have `validation_required = true`. A dict-of-models per variable
//! would work for multi-model prompts, but constitution Scope Discipline says "one
//! concrete path per concern until a second consumer needs it." One model class covers
//! all variables of the prompt (the idiomatic shape), and the coverage check introspects
//! `model.model_fields` for each `validation_required` variable. If a multi-model use
//! case arises, iterate then.
//!
//! ## derive naming
//!
//! The sole mutator is named `derive` (identical across Rust, Python, and TypeScript —
//! no trailing underscore needed since `derive` is not a Python keyword).
//!
//! ## No I/O (Principle III / C-03)
//!
//! The text-factory methods accept already-read text; the Python `tomllib` stdlib module
//! at the 3.12 floor is not needed because the Rust consumer's `Prompt::from_toml`
//! already parses TOML via the `toml` crate — the text just crosses FFI.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use prompting_press::error::code;
use prompting_press::{ConsumerError, FieldError as ConsumerFieldError};
use prompting_press_core::GuardConfig as KernelGuardConfig;

use crate::check::CheckReport;
use crate::error::{consumer_error_to_pyerr, kernel_error_to_pyerr};
use crate::marshal::to_kernel_value;
use crate::render::{validate_in_python, GuardConfig, RenderResult};

// ─── Prompt ──────────────────────────────────────────────────────────────────

/// An immutable, fully-validated prompt object.
///
/// Wraps a [`prompting_press::Prompt`] (the Rust consumer). All construction invariants —
/// shape-valid, template-parseable, agreement-sound, reserved-name clean — are enforced
/// at construction time by the Rust consumer. There are no setters; the sole mutator is
/// [`derive`](Self::derive).
///
/// ## Construction
///
/// Three factory forms:
/// - `Prompt(shape, *, validators=None)` — primary constructor. `shape` is a generated
///   `PromptDefinition` instance or a plain `dict`; the optional `validators` kwarg is a
///   Pydantic model class whose `model_fields` covers every `validation_required` variable.
/// - `Prompt.from_yaml(text, *, validators=None)` — parse already-read YAML, then validate.
/// - `Prompt.from_json(text, *, validators=None)` — parse already-read JSON, then validate.
/// - `Prompt.from_toml(text, *, validators=None)` — parse already-read TOML, then validate.
///   TOML parsing is done by the Rust consumer (the `toml` crate); no Python `tomllib`
///   dependency is added here because the text is handed to the Rust consumer intact.
///
/// Construction **raises** [`PromptValidationError`](crate::error::PromptValidationError)
/// on any invariant violation (invalid shape, parse failure, agreement failure, reserved
/// variant name, or uncovered `validation_required` variable — T036). Never panics.
///
/// ## SEC-004 scrub
///
/// Every error path routes through [`consumer_error_to_pyerr`] or the consumer's `From`
/// scrubber (for `KernelError`). Raw kernel `Parse`/`Render`/`ExcludedFeature` detail is
/// never read here.
#[pyclass(name = "Prompt", module = "prompting_press")]
#[derive(Debug)]
pub struct Prompt {
    /// The validated Rust consumer prompt. Private; exposed through read-only properties.
    inner: prompting_press::Prompt,
    /// The bound Pydantic model class (optional). Stored at construction and used as
    /// the default Vars validator for `render`. When `validators=None`, render must be
    /// called with a model class/instance directly.
    validators: Option<Py<PyAny>>,
}

#[pymethods]
impl Prompt {
    // ── primary constructor ──────────────────────────────────────────────────

    /// `Prompt(shape, *, validators=None)` — the primary validating constructor.
    ///
    /// `shape` is a `PromptDefinition` Pydantic model instance (duck-typed: has
    /// `model_dump_json`) or a plain `dict` / Mapping. Either is reduced to a JSON string
    /// and handed to the Rust consumer's `Prompt::from_json` (the one accept/reject
    /// contract — Q3). On any construction invariant violation a structured
    /// [`PromptValidationError`](crate::error::PromptValidationError) is raised; nothing
    /// is constructed (never a panic).
    ///
    /// `validators` is an optional Pydantic model class. When supplied, construction
    /// CHECKS that every `validation_required` variable in `shape.variables` is covered
    /// by a field in `validators.model_fields`. An uncovered variable raises
    /// [`PromptValidationError`](crate::error::PromptValidationError) naming the variable
    /// (T036 / FR-022..024 / constitution Principle VI v1.2.0).
    ///
    /// # Errors
    ///
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — shape/parse/
    ///   agreement/reserved-name failure, or uncovered `validation_required` variable.
    /// - [`LoadError`](crate::error::LoadError) — `shape` could not be serialized to JSON
    ///   (unexpected Python object, not a Pydantic model or a Mapping).
    #[new]
    #[pyo3(signature = (shape, *, validators = None))]
    fn py_new(
        py: Python<'_>,
        shape: &Bound<'_, PyAny>,
        validators: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let json = definition_to_json(py, shape)?;
        let inner = prompting_press::Prompt::from_json(&json)
            .map_err(|e| consumer_error_to_pyerr(py, e))?;
        check_validator_coverage(py, &inner, validators)?;
        Ok(Self {
            inner,
            validators: validators.map(|v| v.clone().unbind()),
        })
    }

    // ── factory staticmethods ────────────────────────────────────────────────
    //
    // PyO3 0.29 staticmethods don't receive a class argument; they are plain
    // functions namespaced on the type. This is the correct pattern for language
    // bindings where the factory doesn't need `cls` to dispatch subclasses.

    /// `Prompt.from_yaml(text, *, validators=None)` — parse already-read **YAML** and validate.
    ///
    /// Delegates to the Rust consumer's `Prompt::from_yaml`. The binding reads no files
    /// (Principle III / C-03). Error semantics mirror `__init__`.
    ///
    /// # Errors
    ///
    /// - [`LoadError`](crate::error::LoadError) — `text` is not valid YAML or does not
    ///   match the `PromptDefinition` shape.
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — any construction
    ///   invariant violation or uncovered `validation_required` variable.
    #[staticmethod]
    #[pyo3(signature = (text, *, validators = None))]
    fn from_yaml(
        py: Python<'_>,
        text: &str,
        validators: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let inner =
            prompting_press::Prompt::from_yaml(text).map_err(|e| consumer_error_to_pyerr(py, e))?;
        check_validator_coverage(py, &inner, validators)?;
        Ok(Self {
            inner,
            validators: validators.map(|v| v.clone().unbind()),
        })
    }

    /// `Prompt.from_json(text, *, validators=None)` — parse already-read **JSON** and validate.
    ///
    /// Delegates to the Rust consumer's `Prompt::from_json`. Error semantics mirror
    /// [`from_yaml`](Self::from_yaml).
    ///
    /// # Errors
    ///
    /// - [`LoadError`](crate::error::LoadError) — `text` is not valid JSON or does not
    ///   match the `PromptDefinition` shape.
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — construction invariant
    ///   violation or uncovered `validation_required` variable.
    #[staticmethod]
    #[pyo3(signature = (text, *, validators = None))]
    fn from_json(
        py: Python<'_>,
        text: &str,
        validators: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let inner =
            prompting_press::Prompt::from_json(text).map_err(|e| consumer_error_to_pyerr(py, e))?;
        check_validator_coverage(py, &inner, validators)?;
        Ok(Self {
            inner,
            validators: validators.map(|v| v.clone().unbind()),
        })
    }

    /// `Prompt.from_toml(text, *, validators=None)` — parse already-read **TOML** and validate.
    ///
    /// Delegates to the Rust consumer's `Prompt::from_toml` (which uses the `toml` crate
    /// internally). No Python `tomllib` dependency is needed: the text is handed to Rust
    /// intact. At the abi3-py312 floor `tomllib` is available in stdlib, but is not used
    /// here — the Rust consumer already owns the TOML parse. Error semantics mirror
    /// [`from_yaml`](Self::from_yaml).
    ///
    /// # Errors
    ///
    /// - [`LoadError`](crate::error::LoadError) — `text` is not valid TOML or does not
    ///   match the `PromptDefinition` shape.
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — construction invariant
    ///   violation or uncovered `validation_required` variable.
    #[staticmethod]
    #[pyo3(signature = (text, *, validators = None))]
    fn from_toml(
        py: Python<'_>,
        text: &str,
        validators: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        let inner =
            prompting_press::Prompt::from_toml(text).map_err(|e| consumer_error_to_pyerr(py, e))?;
        check_validator_coverage(py, &inner, validators)?;
        Ok(Self {
            inner,
            validators: validators.map(|v| v.clone().unbind()),
        })
    }

    // ── read-only properties ─────────────────────────────────────────────────

    /// The prompt's `name` field.
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// The conversational role (`"system"` / `"user"` / `"assistant"`).
    ///
    /// Stringified from the Rust `PromptDefinitionRole` enum (the same `Display`
    /// implementation the kernel uses for `Message::role` in composition).
    #[getter]
    fn role(&self) -> String {
        self.inner.role().to_string()
    }

    /// The root body template source (the default arm's unrendered template).
    #[getter]
    fn body(&self) -> &str {
        self.inner.body()
    }

    /// The declared variables map (`{ name: PromptVariable }`), as a Python `dict`.
    ///
    /// Serialized via `serde_json` → `depythonize` path so Python receives a plain dict
    /// matching the schema-generated `PromptVariable` shape. Read-only at the object level
    /// (returning a new dict each time prevents Python from mutating the inner state).
    #[getter]
    fn variables<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        map_to_pydict(py, self.inner.variables())
    }

    /// The named variants map (`{ name: PromptVariant }`), as a Python `dict`.
    ///
    /// Empty when the prompt has no named variants (only the implicit default arm).
    #[getter]
    fn variants<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        map_to_pydict(py, self.inner.variants())
    }

    /// The `output_model` reference (`str | None`). Carried as metadata only — the
    /// library never parses against it (Principle III).
    #[getter]
    fn output_model(&self) -> Option<&str> {
        self.inner.output_model()
    }

    /// The `metadata` opaque map (`dict | None`) — library-defined top-level annotations.
    #[getter]
    fn metadata<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        json_map_to_pydict(py, self.inner.metadata())
    }

    // ── operations ────────────────────────────────────────────────────────────

    /// Validate-then-render this prompt.
    ///
    /// Mirrors the current module-level `render(reg, name, vars, *, data, variant, guard)`
    /// signature, adapted for the object surface (C-11 keyword-only tail):
    ///
    /// ```python
    /// p.render(model, *, data=None, variant=None, guard=None)
    /// ```
    ///
    /// - When `data` is provided, `model` is treated as a Pydantic model **class** and
    ///   validated via `model.model_validate(data)`.
    /// - When `data` is `None`, `model` is treated as an already-constructed model
    ///   **instance** and re-validated via `type(model).model_validate(model.model_dump(...))`.
    /// - When `model` is `None` (omitted), the bound `validators` class stored at
    ///   construction is used as the model class (requires `data` to be provided in that
    ///   case since we don't have an instance of a type we didn't pass).
    ///
    /// Validation is owned in Python (Q1 / FR-002), BEFORE any templating. On a
    /// `pydantic.ValidationError`, raises
    /// [`PromptValidationError`](crate::error::PromptValidationError) with one row per
    /// offending field — **the kernel is never reached** on validation failure (SEC-004-PY
    /// scrub: `msg` + `loc` only, never `input`/`ctx`).
    ///
    /// `variant = None` selects the default (root body) arm. `guard` is the opt-in
    /// [`GuardConfig`] plumbed straight through to the kernel (FR-009).
    ///
    /// # Errors
    ///
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — Pydantic rejected
    ///   the vars. Raised **before** any templating (FR-002).
    /// - [`PromptRenderError`](crate::error::PromptRenderError) — the kernel rejected the
    ///   render (unknown variant, strict-undefined reference, parse/render failure). Detail
    ///   scrubbed (SEC-004).
    #[pyo3(signature = (model = None, *, data = None, variant = None, guard = None))]
    fn render(
        &self,
        py: Python<'_>,
        model: Option<&Bound<'_, PyAny>>,
        data: Option<&Bound<'_, PyAny>>,
        variant: Option<&str>,
        guard: Option<&GuardConfig>,
    ) -> PyResult<RenderResult> {
        // Resolve the effective (model, data) pair:
        //  - explicit model given: use it directly (same dual path as the module-level render).
        //  - model omitted: use the bound validator as the model class (requires data).
        let (effective_model, effective_data): (Bound<'_, PyAny>, Option<Bound<'_, PyAny>>) =
            match model {
                Some(m) => (m.clone(), data.cloned()),
                None => {
                    // No model passed — must have a bound validator + explicit data.
                    let Some(bound) = &self.validators else {
                        return Err(consumer_error_to_pyerr(
                            py,
                            ConsumerError::Validation(vec![ConsumerFieldError {
                                field: String::new(),
                                code: code::VALIDATION.to_string(),
                                message:
                                    "render() requires a model or validators bound at construction"
                                        .to_string(),
                            }]),
                        ));
                    };
                    let bound_cls = bound.bind(py);
                    let Some(d) = data else {
                        return Err(consumer_error_to_pyerr(
                            py,
                            ConsumerError::Validation(vec![ConsumerFieldError {
                                field: String::new(),
                                code: code::VALIDATION.to_string(),
                                message: "render() with no model arg requires explicit data= when using a bound validator class".to_string(),
                            }]),
                        ));
                    };
                    (bound_cls.clone(), Some(d.clone()))
                }
            };

        // Validate in Python, BEFORE any templating (FR-002 / Q1).
        let dumped = validate_in_python(py, &effective_model, effective_data.as_ref())?;

        // Marshal the validated payload through the single FFI value bridge (FR-003a).
        let values = to_kernel_value(&dumped).map_err(|e| consumer_error_to_pyerr(py, e))?;

        // Plumb the guard config and call the kernel DIRECTLY (critique E1 / C-01).
        let guard_cfg = guard.map_or_else(KernelGuardConfig::default, KernelGuardConfig::from);

        prompting_press_core::render(self.inner.definition(), variant, values, &guard_cfg)
            .map(RenderResult::from)
            .map_err(|e| kernel_error_to_pyerr(py, e))
    }

    /// Return a variant's **unrendered** template source.
    ///
    /// Pure source lookup: no vars, no validation. `variant = None` returns the root body
    /// source.
    ///
    /// # Errors
    ///
    /// [`PromptRenderError`](crate::error::PromptRenderError) — unknown variant name.
    #[pyo3(signature = (*, variant = None))]
    fn get_source(&self, py: Python<'_>, variant: Option<&str>) -> PyResult<String> {
        self.inner
            .get_source(variant)
            .map(str::to_owned)
            .map_err(|e| consumer_error_to_pyerr(py, e))
    }

    /// Pure advisory lint: returns a [`CheckReport`] containing only the origin/guard
    /// finding class (the only LIVE finding class for a constructed `Prompt`).
    ///
    /// Construction already enforces agreement, parse, and reserved-name invariants, so
    /// those finding classes are structurally unreachable here. Pure: takes `&self`, never
    /// renders, never mutates (FR-019).
    fn check(&self) -> CheckReport {
        self.inner.check().into()
    }

    /// The sole mutator: shallow-replace top-level fields, re-validate, return a new `Prompt`.
    ///
    /// ```python
    /// derived = p.derive(overlay, *, validators=None)
    /// ```
    ///
    /// `overlay` is a `dict` of top-level fields to replace (any subset of `name`, `role`,
    /// `body`, `variables`, `variants`, `output_model`, `metadata`). Fields absent
    /// from the dict are kept from the original. The merged definition is routed through the
    /// Rust consumer's `Prompt::derive` (full re-validation: agreement, parse, reserved name).
    ///
    /// Validators carry forward from `self` by default (R6). Pass `validators=SomeModel` to
    /// override or augment the bound validator on the derived prompt. Coverage is re-checked
    /// against the merged definition.
    ///
    /// The original `Prompt` is untouched (immutability — SC-004).
    ///
    /// # Errors
    ///
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — the merged
    ///   definition violates any construction invariant, or the (carried/supplied) validators
    ///   do not cover a `validation_required` variable in the merged definition.
    /// - [`LoadError`](crate::error::LoadError) — `overlay` could not be deserialized.
    #[pyo3(signature = (overlay, *, validators = None))]
    fn derive(
        &self,
        py: Python<'_>,
        overlay: &Bound<'_, PyAny>,
        validators: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        // Build a PromptOverlay from the Python dict via JSON round-trip.
        let rust_overlay = build_overlay(py, overlay)?;

        let derived_inner = self
            .inner
            .derive(rust_overlay)
            .map_err(|e| consumer_error_to_pyerr(py, e))?;

        // Determine the effective validators for the derived prompt (R6):
        //   - overlay supplies new validators  → use those
        //   - overlay supplies None explicitly → None (clear)
        //   - overlay kwarg not supplied       → carry forward from self
        // The kwarg is always Some(value)/None as passed by Python; we distinguish
        // "not supplied" from "supplied as None" by checking if `validators` kwarg
        // was present (always Some in PyO3 — the signature gives us the Option<&...>).
        //
        // In practice: if the caller passes `validators=None` we treat it as "clear";
        // if the caller omits `validators` entirely, PyO3 gives us `None` too —
        // indistinguishable. To be safe and ergonomic: treat Option::None as "carry
        // forward from self" (the R6 default). If the caller explicitly wants to drop
        // validators, they'd need to do `p.derive(overlay, validators=None)`.
        // Since we can't distinguish "omitted" vs "explicitly None" in PyO3 without a
        // sentinel, we use the R6 carry-forward rule for None (least surprise).
        let effective_validators: Option<Py<PyAny>> = match validators {
            Some(v) => Some(v.clone().unbind()),
            None => self.validators.as_ref().map(|v| v.clone_ref(py)), // carry forward (R6)
        };

        // Coverage check against the merged definition using effective validators.
        let effective_validators_bound = effective_validators
            .as_ref()
            .map(|v: &Py<PyAny>| v.bind(py).clone());
        check_validator_coverage(py, &derived_inner, effective_validators_bound.as_ref())?;

        Ok(Self {
            inner: derived_inner,
            validators: effective_validators,
        })
    }

    // ── repr ─────────────────────────────────────────────────────────────────

    /// `repr(prompt)` — compact, fixed-shape. Safe to surface: name and role are schema
    /// metadata, not bound-value content.
    fn __repr__(&self) -> String {
        format!(
            "Prompt(name={:?}, role={:?})",
            self.inner.name(),
            self.inner.role().to_string()
        )
    }
}

// ─── crate-internal accessors ─────────────────────────────────────────────────

impl Prompt {
    /// Borrow the inner Rust consumer `Prompt`, for crate-internal callers (e.g. `compose.rs`).
    pub(crate) fn inner_prompt(&self) -> &prompting_press::Prompt {
        &self.inner
    }

    /// Build a `Prompt` from a JSON string without validators, for crate-internal Rust unit
    /// tests (e.g. `compose.rs`, `prompt.rs` `#[cfg(test)]`). Panics on invalid input.
    #[cfg(test)]
    pub(crate) fn from_json_for_test(json: &str) -> Self {
        let inner = prompting_press::Prompt::from_json(json).expect("valid prompt JSON in test");
        Self {
            inner,
            validators: None,
        }
    }
}

// ─── helpers ──────────────────────────────────────────────────────────────────

/// Reduce a `shape` argument (Pydantic model instance OR dict / Mapping) to a JSON
/// string for the Rust consumer's `from_json`. Reuses the duck-typing path from
/// `registry.rs::definition_to_json` but is reproduced here to keep the coupling local
/// to this module and avoid making `registry.rs` a dep of `prompt.rs`.
///
/// SEC-004 (review L-3): a `model_dump_json` failure can carry value-bearing text.
/// Withhold the raw detail — emit a fixed message.
fn definition_to_json(py: Python<'_>, definition: &Bound<'_, PyAny>) -> PyResult<String> {
    // Pydantic model instance (duck-typed: has a callable `model_dump_json`).
    if definition
        .hasattr("model_dump_json")
        .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?
    {
        let dumper = definition
            .getattr("model_dump_json")
            .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
        if dumper.is_callable() {
            let kwargs = PyDict::new(py);
            kwargs
                .set_item("exclude_none", true)
                .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
            let json_obj = dumper.call((), Some(&kwargs)).map_err(|_| {
                consumer_error_to_pyerr(
                    py,
                    ConsumerError::Load("could not serialize the prompt definition".to_string()),
                )
            })?;
            let json: String = json_obj
                .extract::<String>()
                .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
            return Ok(json);
        }
    }

    // Plain dict / Mapping: depythonize → serde_json::Value → JSON.
    let value: serde_json::Value = pythonize::depythonize(definition)
        .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
    serde_json::to_string(&value)
        .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))
}

/// Build a [`prompting_press::PromptOverlay`] from a Python dict.
///
/// The overlay dict maps field names to their new values. Each present key is
/// deserialized individually from its Python representation. Fields absent from the
/// dict are left as `None` in the overlay (unchanged in the merged definition).
///
/// Approach: serialize the whole overlay dict to JSON, then deserialize the
/// individual `Option<T>` fields from the resulting `serde_json::Value`. This keeps
/// the deserialization logic in Rust (one path, no per-field extraction code) and
/// lets `serde_json` handle all the nested-type conversions.
fn build_overlay(
    py: Python<'_>,
    overlay: &Bound<'_, PyAny>,
) -> PyResult<prompting_press::PromptOverlay> {
    // Depythonize the dict to a serde_json::Value map.
    let overlay_value: serde_json::Value = pythonize::depythonize(overlay)
        .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;

    let obj = match &overlay_value {
        serde_json::Value::Object(m) => m,
        _ => {
            return Err(consumer_error_to_pyerr(
                py,
                ConsumerError::Load("overlay must be a dict".to_string()),
            ));
        }
    };

    use prompting_press_core::generated::prompt_definition::{
        PromptDefinitionName, PromptDefinitionRole, PromptVariable, PromptVariant,
    };
    use std::collections::HashMap;

    // Each field is deserialized only if present in the overlay dict.
    macro_rules! overlay_field {
        ($field:expr, $ty:ty) => {
            obj.get($field)
                .map(|v| serde_json::from_value::<$ty>(v.clone()))
                .transpose()
                .map_err(|e| {
                    consumer_error_to_pyerr(
                        py,
                        ConsumerError::Load(format!("overlay field `{}`: {e}", $field)),
                    )
                })?
        };
    }

    Ok(prompting_press::PromptOverlay {
        name: overlay_field!("name", PromptDefinitionName),
        role: overlay_field!("role", PromptDefinitionRole),
        body: overlay_field!("body", String),
        variables: overlay_field!("variables", HashMap<String, PromptVariable>),
        variants: overlay_field!("variants", HashMap<String, PromptVariant>),
        output_model: overlay_field!("output_model", Option<String>),
        metadata: overlay_field!(
            "metadata",
            serde_json::Map<String, serde_json::Value>
        ),
    })
}

/// Coverage check for `validation_required` variables (T036 / FR-022..024).
///
/// For every variable in `prompt.variables()` that has `validation_required = true`,
/// this verifies that the supplied Pydantic model class (if any) exposes that field in
/// `model_fields`. Raises [`PromptValidationError`](crate::error::PromptValidationError)
/// naming the first uncovered variable.
///
/// When `validators = None` and at least one variable is `validation_required`, raises
/// immediately (you cannot silently skip required validation).
///
/// Python introspection: `validators.model_fields` is a dict keyed by field name. We
/// only check key presence — the field type and validator logic are the Pydantic class's
/// own concern (Principle VI: the library does not invent its own validation framework).
fn check_validator_coverage(
    py: Python<'_>,
    prompt: &prompting_press::Prompt,
    validators: Option<&Bound<'_, PyAny>>,
) -> PyResult<()> {
    // Collect variables that require a validator.
    let required: Vec<&str> = prompt
        .variables()
        .iter()
        .filter(|(_, decl)| decl.validation_required)
        .map(|(name, _)| name.as_str())
        .collect();

    if required.is_empty() {
        return Ok(());
    }

    // At least one variable is validation_required.
    let Some(validators_cls) = validators else {
        // No validator supplied — raise naming the first uncovered variable.
        let field = required[0];
        return Err(consumer_error_to_pyerr(
            py,
            ConsumerError::Validation(vec![ConsumerFieldError {
                field: field.to_string(),
                code: code::VALIDATION.to_string(),
                message: format!(
                    "variable `{field}` has `validation_required = true` but no \
                     validators were supplied at construction"
                ),
            }]),
        ));
    };

    // Introspect `validators.model_fields` — a dict keyed by field name (Pydantic V2).
    let model_fields = validators_cls.getattr("model_fields").map_err(|_| {
        consumer_error_to_pyerr(
            py,
            ConsumerError::Validation(vec![ConsumerFieldError {
                field: String::new(),
                code: code::VALIDATION.to_string(),
                message: "validators must be a Pydantic model class (must have `model_fields`)"
                    .to_string(),
            }]),
        )
    })?;

    for var_name in required {
        let covered = model_fields.contains(var_name).unwrap_or(false);
        if !covered {
            return Err(consumer_error_to_pyerr(
                py,
                ConsumerError::Validation(vec![ConsumerFieldError {
                    field: var_name.to_string(),
                    code: code::VALIDATION.to_string(),
                    message: format!(
                        "variable `{var_name}` has `validation_required = true` but the \
                         supplied validators model does not declare this field \
                         (introspected via `model_fields`)"
                    ),
                }]),
            ));
        }
    }

    Ok(())
}

/// Serialize a `HashMap<String, T: Serialize>` to a Python dict via `serde_json` +
/// `pythonize`. Returns a generic `PyAny` (a dict at runtime).
fn map_to_pydict<'py, T: serde::Serialize>(
    py: Python<'py>,
    map: &std::collections::HashMap<String, T>,
) -> PyResult<Bound<'py, PyAny>> {
    let json_val = serde_json::to_value(map).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("serialization error: {e}"))
    })?;
    pythonize::pythonize(py, &json_val).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("pythonize error: {e}"))
    })
}

/// Serialize a `serde_json::Map<String, Value>` to a Python dict.
fn json_map_to_pydict<'py>(
    py: Python<'py>,
    map: &serde_json::Map<String, serde_json::Value>,
) -> PyResult<Bound<'py, PyAny>> {
    let json_val = serde_json::Value::Object(map.clone());
    pythonize::pythonize(py, &json_val).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("pythonize error: {e}"))
    })
}

// ─── unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make(json: &str) -> Prompt {
        Prompt::from_json_for_test(json)
    }

    fn valid_json() -> &'static str {
        r#"{"name":"greet","role":"user","body":"Hi {{ name }}","variables":{"name":{"type":"string","origin":"trusted"}}}"#
    }

    // ── construction (T035) ───────────────────────────────────────────────────

    /// A valid JSON shape constructs a `Prompt` with correct accessors.
    #[test]
    fn from_json_valid_constructs_and_accessors_work() {
        Python::attach(|_py| {
            let prompt = make(valid_json());
            assert_eq!(prompt.name(), "greet");
            assert_eq!(prompt.role(), "user");
            assert_eq!(prompt.body(), "Hi {{ name }}");
            assert!(prompt.inner.variables().contains_key("name"));
            assert!(prompt.inner.variants().is_empty());
        });
    }

    /// `from_json` rejects undeclared variables — construction fails with a ConsumerError.
    #[test]
    fn from_json_undeclared_variable_raises() {
        let json = r#"{"name":"bad","role":"user","body":"{{ ghost }}","variables":{"name":{"type":"string","origin":"trusted"}}}"#;
        let result = prompting_press::Prompt::from_json(json);
        assert!(
            result.is_err(),
            "undeclared variable must fail construction"
        );
        match result.unwrap_err() {
            prompting_press::ConsumerError::Kernel(rows) => {
                assert!(
                    rows.iter()
                        .any(|r| r.code == prompting_press::error::code::UNDEFINED_VARIABLE),
                    "expected undefined_variable code, got {rows:?}"
                );
            }
            other => panic!("expected ConsumerError::Kernel, got {other:?}"),
        }
    }

    /// Reserved variant name rejects at construction.
    #[test]
    fn from_json_reserved_variant_name_raises() {
        let json = r#"{"name":"bad","role":"user","body":"Hi","variables":{},"variants":{"default":{"body":"shadowed"}}}"#;
        assert!(
            prompting_press::Prompt::from_json(json).is_err(),
            "reserved variant name must fail"
        );
    }

    // ── from_toml (T035) ──────────────────────────────────────────────────────

    #[test]
    fn from_toml_valid_constructs() {
        let toml_text = r#"
name = "greeting"
role = "user"
body = "Hi {{ name }}"

[variables.name]
type = "string"
origin = "trusted"
"#;
        let p = prompting_press::Prompt::from_toml(toml_text).expect("TOML must construct");
        assert_eq!(p.name(), "greeting");
        assert_eq!(p.body(), "Hi {{ name }}");
    }

    // ── from_yaml (T035) ──────────────────────────────────────────────────────

    #[test]
    fn from_yaml_valid_constructs() {
        let yaml = "name: hi\nrole: user\nbody: \"Hello {{ name }}\"\nvariables:\n  name:\n    type: string\n    origin: trusted\n";
        let p = prompting_press::Prompt::from_yaml(yaml).expect("YAML must construct");
        assert_eq!(p.name(), "hi");
    }

    // ── validation_required coverage (T036) ───────────────────────────────────

    /// A prompt with no `validation_required` variables — from_json_for_test works without validators.
    #[test]
    fn no_validation_required_no_validators_ok() {
        Python::attach(|_py| {
            let prompt = make(valid_json());
            assert_eq!(prompt.name(), "greet");
        });
    }

    /// A prompt with `validation_required = true` and no validators raises via `check_validator_coverage`.
    #[test]
    fn validation_required_without_validators_raises() {
        Python::attach(|py| {
            let json = r#"{"name":"strict","role":"user","body":"Hi {{ name }}","variables":{"name":{"type":"string","origin":"trusted","validation_required":true}}}"#;
            // Build the consumer Prompt directly (bypasses the coverage check, which is
            // the binding's responsibility). Then test the coverage function directly.
            let inner = prompting_press::Prompt::from_json(json).expect("consumer accepts");
            let result = check_validator_coverage(py, &inner, None);
            assert!(result.is_err(), "must raise PromptValidationError");
            let err = result.unwrap_err();
            assert!(
                err.value(py)
                    .is_instance_of::<crate::error::PromptValidationError>(),
                "must be PromptValidationError, got {:?}",
                err.value(py).get_type().name().unwrap()
            );
        });
    }

    // ── render via kernel-direct (T037) ───────────────────────────────────────

    #[test]
    fn render_kernel_direct_produces_text_and_hashes() {
        use pyo3::types::PyDict;

        Python::attach(|py| {
            let prompt = make(valid_json());
            let data = PyDict::new(py);
            data.set_item("name", "Ada").expect("set name");
            let values = to_kernel_value(data.as_any()).expect("marshal");

            let result = prompting_press_core::render(
                prompt.inner.definition(),
                None,
                values,
                &KernelGuardConfig::default(),
            )
            .map(RenderResult::from)
            .expect("render succeeds");

            assert_eq!(result.text, "Hi Ada");
            assert_eq!(result.name, "greet");
            assert_eq!(result.variant, "default");
            assert_eq!(result.template_hash.len(), 64);
        });
    }

    // ── get_source (T037) ─────────────────────────────────────────────────────

    #[test]
    fn get_source_returns_unrendered_root() {
        Python::attach(|py| {
            let prompt = make(valid_json());
            let src = prompt.get_source(py, None).expect("root source");
            assert_eq!(src, "Hi {{ name }}");
        });
    }

    #[test]
    fn get_source_unknown_variant_raises() {
        Python::attach(|py| {
            let prompt = make(valid_json());
            let err = prompt
                .get_source(py, Some("nope"))
                .expect_err("unknown variant must err");
            assert!(
                err.value(py)
                    .is_instance_of::<crate::error::PromptRenderError>()
                    || err
                        .value(py)
                        .is_instance_of::<crate::error::PromptingPressError>(),
                "must be a PromptingPressError subtype, got {:?}",
                err.value(py).get_type().name().unwrap()
            );
        });
    }

    // ── check (T037) ──────────────────────────────────────────────────────────

    #[test]
    fn check_returns_origin_advisory() {
        Python::attach(|_py| {
            let json = r#"{"name":"unguarded","role":"user","body":"{{ payload }}","variables":{"payload":{"type":"string","origin":"untrusted"}}}"#;
            let prompt = make(json);
            let report = prompt.check();
            assert!(
                !report.findings.is_empty(),
                "unguarded untrusted must produce a finding"
            );
        });
    }

    // ── derive (T038) ────────────────────────────────────────────────────────

    #[test]
    fn derive_overlay_creates_derived_original_untouched() {
        use pyo3::types::{PyDict, PyString};

        Python::attach(|py| {
            let original = make(valid_json());
            let original_body = original.body().to_string();

            let overlay = PyDict::new(py);
            overlay
                .set_item("body", PyString::new(py, "Hey {{ name }}"))
                .expect("set body");

            let derived = original
                .derive(py, overlay.as_any(), None)
                .expect("valid overlay must succeed");

            assert_eq!(derived.body(), "Hey {{ name }}");
            assert_eq!(
                original.body(),
                original_body,
                "original must be untouched (SC-004)"
            );
        });
    }

    #[test]
    fn derive_undeclared_var_overlay_raises() {
        use pyo3::types::{PyDict, PyString};

        Python::attach(|py| {
            let original = make(valid_json());
            let overlay = PyDict::new(py);
            overlay
                .set_item("body", PyString::new(py, "{{ ghost }}"))
                .expect("set body");

            let result = original.derive(py, overlay.as_any(), None);
            assert!(
                result.is_err(),
                "overlay introducing undeclared var must fail"
            );
        });
    }
}
