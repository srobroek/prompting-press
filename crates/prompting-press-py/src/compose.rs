//! Multi-message composition for the Python binding (spec 008 Phase 4, T039; FR-012/FR-013).
//!
//! A [`Composition`] is an **explicit, ordered** sequence of `(Prompt, vars, variant?)`
//! entries that [`resolve`](Composition::resolve)s — in append order — to a `list[Message]`,
//! where each [`Message`] is the `Prompt` rendered with its own validated vars and tagged
//! with that prompt definition's `role` (FR-012). It is the few-shot / system+user sequence
//! builder. Construction is `Composition()` + [`append`](Composition::append), or the
//! [`from_messages`](Composition::from_messages) bulk constructor — there is deliberately **no**
//! fluent `.chain()` API (FR-013; it cannot cross the PyO3 boundary and collides with
//! `Iterator::chain`).
//!
//! ## Phase 4 reshape (T039)
//!
//! In the pre-reshape binding `Composition.append(name, vars, variant=None)` stored prompt
//! NAMES and `resolve(registry)` resolved them against a registry. Phase 4 replaces that
//! with `Composition.append(prompt, vars, variant=None)` — the entry holds the **Prompt
//! object** directly, and `resolve()` takes no registry argument. This eliminates the
//! registry concept from the composition surface (contract `prompt-api.md`).
//!
//! The `from_messages` bulk constructor correspondingly accepts `(Prompt, vars)` /
//! `(Prompt, vars, variant)` tuples.
//!
//! ## Why a binding-OWNED `Composition` (critique E1 / C-01)
//!
//! The Rust consumer's [`prompting_press::Composition`] is generic over `V: Serialize + Validate`
//! — a **garde** type. This binding has no such type: validation is owned in **Python** (against
//! the caller's Pydantic Vars model — Q1), so there is no `V` to instantiate the consumer's
//! `Composition` with. Therefore this module owns its **own** `Composition` `#[pyclass]` holding
//! already-marshaled entries, and [`resolve`](Composition::resolve) calls the **kernel directly**
//! per entry — exactly mirroring how the binding's [`Prompt::render`](crate::prompt::Prompt)
//! works. This is still **zero engine logic** (Principle I): the kernel renders; the binding only
//! validates (in Python), marshals, and surfaces results.
//!
//! ## Eager validation at `append` — option (a) (no partial state)
//!
//! Each entry is validated + marshaled **eagerly at `append`**: the Pydantic vars are validated
//! NOW via the US1 Python-validation path ([`render::validate_in_python`](crate::render)), the
//! validated payload is marshaled to the kernel's [`minijinja::Value`], and only then is the
//! entry stored. On a validation failure the call raises
//! [`PromptValidationError`](crate::error) and **stores nothing** — the composition is left
//! exactly as it was, so [`resolve`](Composition::resolve) can only ever see fully-validated
//! entries.
//!
//! ## The entry vars shape — a Pydantic model **instance** per entry
//!
//! A composition entry's `vars` is a constructed Pydantic model **instance** (the natural fit
//! for a list of `(prompt, vars)` entries). It is validated via the same instance path `render`
//! uses when `data is None`. A single shape per concern keeps the surface narrow (Scope
//! Discipline).
//!
//! ## resolve: render in order, no registry
//!
//! [`resolve`](Composition::resolve) walks the stored entries in append order. For each it calls
//! the kernel **directly** with the entry's pre-validated value and the bound
//! `PromptDefinition`. Each result becomes `Message { role, text }`. One entry's failure
//! (unknown variant, strict-undefined reference) propagates as the mapped Python exception and
//! the partial result is **discarded** — never returned as success. An empty composition
//! resolves to `[]`.

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use prompting_press_core::GuardConfig as KernelGuardConfig;

use crate::error::kernel_error_to_pyerr;
use crate::marshal::to_kernel_value;
use crate::prompt::Prompt;
use crate::render::validate_in_python;

/// One resolved message in a composition's output: a role-tagged rendered string.
///
/// The Python mirror of the consumer's `Message` (data-model §Message). `role` is the prompt
/// definition's role stringified (`"system"` / `"user"` / `"assistant"`); `text` is that prompt
/// rendered with the entry's own validated vars. Read-only (`frozen`): a message is produced by
/// [`Composition::resolve`], never constructed from Python.
// `skip_from_py_object`: output-only — Python reads the getters, never passes a `Message` *in* —
// so opt out of the implicit `FromPyObject` derive PyO3 0.29 would otherwise pull in.
#[pyclass(
    name = "Message",
    frozen,
    module = "prompting_press",
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct Message {
    /// The conversational role, taken from the prompt definition's `role`.
    #[pyo3(get)]
    pub role: String,
    /// The rendered body text for this entry. The guard text is never concatenated here.
    #[pyo3(get)]
    pub text: String,
}

#[pymethods]
impl Message {
    /// `repr(message)` — a compact, fixed-shape rendering. `text` is the caller's own
    /// (already-rendered) output for this entry.
    fn __repr__(&self) -> String {
        format!("Message(role={:?}, text={:?})", self.role, self.text)
    }
}

/// One appended entry, captured after eager validation + marshaling (option (a)).
///
/// The vars are already marshaled into a [`minijinja::Value`] (the same type the kernel
/// renders against). The `Prompt` is cloned via its `Py<Prompt>` reference — the pyclass
/// is not `Clone`, but `Py<T>` is (it holds a reference-counted Python object pointer).
struct Entry {
    /// The bound `Prompt` object (the phase-4 replacement for a prompt name + registry lookup).
    prompt: Py<Prompt>,
    /// The pre-validated, marshaled vars (the FFI bridge value — FR-003a), ready for the kernel.
    values: minijinja::Value,
    /// The selected variant (`None` ⇒ the reserved `default` / root body).
    variant: Option<String>,
}

/// An explicit, ordered sequence of `(Prompt, vars, variant?)` entries that resolves to a
/// `list[Message]` in append order (FR-012). Built with `Composition()` +
/// [`append`](Self::append) or [`from_messages`](Self::from_messages); there is **no** fluent
/// `.chain()` (FR-013).
#[pyclass(name = "Composition", module = "prompting_press")]
pub struct Composition {
    /// Entries in append order — the resolved-message order (FR-012).
    entries: Vec<Entry>,
}

#[pymethods]
impl Composition {
    /// `Composition()` — create an empty composition. An empty composition resolves to `[]`.
    #[new]
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// `Composition.from_messages(entries)` — build a composition from a sequence of
    /// `(prompt, vars)` or `(prompt, vars, variant)` tuples, eager-validating each in order.
    ///
    /// `entries` is any Python iterable of 2- or 3-element tuples. `prompt` is a `Prompt`
    /// object (the phase-4 replacement for a name string); `vars` is a constructed Pydantic
    /// model **instance** (see module docs); `variant` defaults to `None` (the reserved
    /// `default` arm) for the 2-tuple form.
    ///
    /// Each entry is validated + marshaled via the same path [`append`](Self::append) uses, in
    /// order. The first invalid entry raises
    /// [`PromptValidationError`](crate::error::PromptValidationError) and the whole construction
    /// fails — **no** `Composition` is returned (no partial state).
    ///
    /// # Errors
    /// - [`PromptValidationError`](crate::error::PromptValidationError) — Pydantic rejected an
    ///   entry's vars.
    /// - `TypeError` — an entry is not a 2- or 3-element tuple/sequence, or the first element
    ///   is not a `Prompt`.
    #[staticmethod]
    fn from_messages(py: Python<'_>, entries: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut composition = Self::new();
        for item in entries.try_iter()? {
            let item = item?;
            let (prompt, vars, variant) = unpack_entry(py, &item)?;
            composition.append_entry(py, &prompt, &vars, variant.as_deref())?;
        }
        Ok(composition)
    }

    /// `append(prompt, vars, *, variant=None)` — eager-validate + marshal + store one entry.
    ///
    /// `prompt` is a `Prompt` object; `vars` is a constructed Pydantic model **instance**
    /// (see module docs); it is validated **now** via the US1 Python-validation path and
    /// marshaled to the kernel's value type before the entry is stored. On a validation failure
    /// this raises [`PromptValidationError`](crate::error::PromptValidationError) and **stores
    /// nothing** — the composition is left exactly as it was (no partial state).
    ///
    /// Takes `&mut self` and returns `None` (not `self`): the builder is intentionally **not**
    /// fluent/chainable (FR-013).
    ///
    /// # Errors
    /// [`PromptValidationError`](crate::error::PromptValidationError) — Pydantic rejected
    /// `vars`. The entry is not stored.
    #[pyo3(signature = (prompt, vars, *, variant = None))]
    fn append(
        &mut self,
        py: Python<'_>,
        prompt: &Bound<'_, Prompt>,
        vars: &Bound<'_, PyAny>,
        variant: Option<&str>,
    ) -> PyResult<()> {
        self.append_entry(py, prompt, vars, variant)
    }

    /// `len(composition)` — the number of appended entries (== the resolved-message count on
    /// success).
    fn __len__(&self) -> usize {
        self.entries.len()
    }

    /// `repr(composition)` — a compact, fixed-shape rendering naming only the entry count and
    /// the ordered prompt names (caller-supplied identifiers, not bound-value content). The
    /// marshaled vars are never surfaced.
    fn __repr__(&self, py: Python<'_>) -> String {
        let names: Vec<String> = self
            .entries
            .iter()
            .map(|e| {
                e.prompt
                    .bind(py)
                    .borrow()
                    .inner_prompt()
                    .name()
                    .to_string()
            })
            .collect();
        format!("Composition(entries={}, names={:?})", names.len(), names)
    }

    /// `resolve()` — resolve the composition to an ordered `list[Message]` (FR-012), rendering
    /// each entry — in append order — through the kernel.
    ///
    /// For each entry, in order: call the kernel **directly** (critique E1 / C-01) with the
    /// entry's bound `Prompt` definition and **pre-validated** value (vars were validated at
    /// [`append`](Self::append)). The render result becomes
    /// `Message { role: <def.role stringified>, text: result.text }`. Composition uses no
    /// guard expansion — a default [`GuardConfig`] is passed, which leaves `text` unchanged.
    ///
    /// One entry's render failure (unknown variant, a strict-undefined reference, a parse/render
    /// error) propagates as the mapped Python exception and the partial result built so far is
    /// **discarded** — never returned as success. An empty composition resolves to `[]`.
    ///
    /// # Errors
    /// - [`PromptRenderError`](crate::error::PromptRenderError) — the kernel rejected an
    ///   entry's render (unknown variant, a strict-undefined reference, a parse/render failure).
    ///   `parse`/`render` detail is scrubbed (SEC-004).
    fn resolve(&self, py: Python<'_>) -> PyResult<Vec<Message>> {
        let mut messages = Vec::with_capacity(self.entries.len());

        for entry in &self.entries {
            let prompt_ref = entry.prompt.bind(py);
            let prompt = prompt_ref.borrow();
            let inner = prompt.inner_prompt();

            // Render by calling the KERNEL DIRECTLY (critique E1 / C-01) with the
            // already-validated value. Composition does no guard expansion; a default
            // GuardConfig leaves `text` unchanged. A kernel failure propagates here,
            // discarding the partial `messages` built so far — no partial-as-success.
            let result = prompting_press_core::render(
                inner.definition(),
                entry.variant.as_deref(),
                entry.values.clone(),
                &KernelGuardConfig::default(),
            )
            .map_err(|e| kernel_error_to_pyerr(py, e))?;

            messages.push(Message {
                // The prompt definition's role, stringified via its `Display` impl.
                role: inner.role().to_string(),
                text: result.text,
            });
        }

        Ok(messages)
    }
}

impl Composition {
    /// The shared eager-validate + marshal + store step behind both [`append`](Self::append) and
    /// [`from_messages`](Self::from_messages).
    ///
    /// Validates `vars` (a Pydantic model instance) via the US1 Python-validation path
    /// ([`validate_in_python`] with `data = None` — the instance path:
    /// `type(vars).model_validate(vars.model_dump(mode="json"))`), marshals the validated
    /// payload to the kernel's value type, and only then pushes the entry. On any failure
    /// the error propagates and **nothing is stored** — `self.entries` is mutated only on
    /// the success path.
    fn append_entry(
        &mut self,
        py: Python<'_>,
        prompt: &Bound<'_, Prompt>,
        vars: &Bound<'_, PyAny>,
        variant: Option<&str>,
    ) -> PyResult<()> {
        // Validate in Python, BEFORE marshaling (FR-002 / Q1). `data = None` selects the
        // instance path. A ValidationError surfaces as PromptValidationError; the validated,
        // JSON-dumped payload is returned on success.
        let dumped = validate_in_python(py, vars, None)?;

        // Marshal the validated payload through the single FFI value bridge (FR-003a).
        let values =
            to_kernel_value(&dumped).map_err(|e| crate::error::consumer_error_to_pyerr(py, e))?;

        // Only on full success do we mutate state — no partial entry is ever stored.
        self.entries.push(Entry {
            prompt: prompt.clone().unbind(),
            values,
            variant: variant.map(str::to_string),
        });
        Ok(())
    }
}

/// Unpack one `from_messages` entry into `(prompt, vars, variant)`.
///
/// Accepts a 2-element `(prompt, vars)` (variant defaults to `None`) or a 3-element
/// `(prompt, vars, variant)` tuple/sequence. `variant` may be `None` even in the 3-tuple
/// form (explicit "use the default arm"). A wrong arity, a non-Prompt first element, or
/// a non-string `variant` raises a `TypeError` — a caller-API misuse, not a validation
/// failure.
fn unpack_entry<'py>(
    _py: Python<'py>,
    item: &Bound<'py, PyAny>,
) -> PyResult<(Bound<'py, Prompt>, Bound<'py, PyAny>, Option<String>)> {
    let tuple = item.cast::<PyTuple>().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "composition entry must be a tuple of (prompt, vars) or (prompt, vars, variant)",
        )
    })?;
    let len = tuple.len();
    if !(2..=3).contains(&len) {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "composition entry must have 2 or 3 elements (prompt, vars[, variant]), got {len}"
        )));
    }
    let prompt_any = tuple.get_item(0)?;
    let prompt = prompt_any.cast::<Prompt>().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "composition entry first element must be a Prompt object",
        )
    })?;
    let vars = tuple.get_item(1)?;
    let variant: Option<String> = if len == 3 {
        tuple.get_item(2)?.extract()?
    } else {
        None
    };
    Ok((prompt.clone(), vars, variant))
}

#[cfg(test)]
mod tests {
    //! Composition coverage drivable in Rust WITHOUT a Pydantic model.
    //!
    //! The eager-validate-at-append behavior needs a real Pydantic model and is covered
    //! Python-side in T040. Here we exercise: an empty composition resolves to `[]`, and a
    //! render failure (unknown variant, strict-undefined) surfaces as `PromptRenderError`.

    use super::*;
    use crate::error::PromptRenderError;
    use crate::prompt::Prompt;
    use crate::render::RenderResult;

    fn make_prompt_py(py: Python<'_>, json: &str) -> Py<Prompt> {
        let prompt = Prompt::from_json_for_test(json);
        Py::new(py, prompt).expect("Py::new")
    }

    /// An empty composition resolves to an empty list (the `[]` edge case).
    #[test]
    fn empty_composition_resolves_to_empty() {
        Python::attach(|py| {
            let comp = Composition::new();
            assert_eq!(comp.__len__(), 0, "a fresh composition has no entries");

            let messages = comp.resolve(py).expect("empty resolve");
            assert!(messages.is_empty(), "empty composition ⇒ empty list");
        });
    }

    /// An entry with an unknown variant surfaces as `PromptRenderError` at `resolve`.
    /// We construct the entry directly (bypassing the Pydantic validate path) to drive
    /// the render loop's variant-resolution branch without a Pydantic model.
    #[test]
    fn unknown_variant_entry_is_loud_at_resolve() {
        Python::attach(|py| {
            let prompt_py = make_prompt_py(
                py,
                r#"{"name":"greet","role":"user","body":"Hi {{ name }}","variables":{"name":{"type":"string","origin":"trusted"}}}"#,
            );

            let comp = Composition {
                entries: vec![Entry {
                    prompt: prompt_py,
                    values: minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" })),
                    variant: Some("nonexistent".to_string()),
                }],
            };

            let err = comp
                .resolve(py)
                .expect_err("an unknown variant must error at resolve");
            assert!(
                err.value(py).is_instance_of::<PromptRenderError>(),
                "an unknown variant maps to PromptRenderError, got {:?}",
                err.value(py).get_type().name().unwrap()
            );
        });
    }

    /// `resolve` renders each entry in append order through the kernel and tags it with the
    /// prompt's role. The entries are constructed directly (already-marshaled values,
    /// bypassing the Python validation `append` performs) so this drives the kernel-direct
    /// resolve loop + the `role` stringification + the message ORDER without a Pydantic model.
    #[test]
    fn resolve_renders_in_order_with_roles() {
        Python::attach(|py| {
            let system_py = make_prompt_py(
                py,
                r#"{"name":"sys","role":"system","body":"You are {{ persona }}.","variables":{"persona":{"type":"string","origin":"trusted"}}}"#,
            );
            let user_py = make_prompt_py(
                py,
                r#"{"name":"ask","role":"user","body":"Question: {{ q }}","variables":{"q":{"type":"string","origin":"trusted"}}}"#,
            );

            let comp = Composition {
                entries: vec![
                    Entry {
                        prompt: system_py,
                        values: minijinja::Value::from_serialize(
                            serde_json::json!({ "persona": "a helpful assistant" }),
                        ),
                        variant: None,
                    },
                    Entry {
                        prompt: user_py,
                        values: minijinja::Value::from_serialize(
                            serde_json::json!({ "q": "why?" }),
                        ),
                        variant: None,
                    },
                ],
            };

            let messages = comp.resolve(py).expect("resolve succeeds");
            assert_eq!(messages.len(), 2, "one message per entry");

            // Append order is preserved (FR-012).
            assert_eq!(messages[0].role, "system");
            assert_eq!(messages[0].text, "You are a helpful assistant.");
            assert_eq!(messages[1].role, "user");
            assert_eq!(messages[1].text, "Question: why?");
        });
    }

    /// A later entry's render failure propagates as the mapped exception and DISCARDS the
    /// partial result — `resolve` returns `Err`, never a truncated `list[Message]`
    /// (no partial-as-success). The second entry lacks the required root `missing`.
    #[test]
    fn one_entry_failure_discards_partial_result() {
        Python::attach(|py| {
            let ok_py = make_prompt_py(py, r#"{"name":"ok","role":"user","body":"fine"}"#);
            let needs_py = make_prompt_py(
                py,
                r#"{"name":"needs","role":"user","body":"Hello {{ missing }}!","variables":{"missing":{"type":"string","origin":"trusted"}}}"#,
            );

            let comp = Composition {
                entries: vec![
                    Entry {
                        prompt: ok_py,
                        values: minijinja::Value::from_serialize(serde_json::json!({})),
                        variant: None,
                    },
                    Entry {
                        // No `missing` in the value ⇒ strict-undefined kernel error.
                        prompt: needs_py,
                        values: minijinja::Value::from_serialize(serde_json::json!({})),
                        variant: None,
                    },
                ],
            };

            let err = comp.resolve(py).expect_err(
                "the second entry's strict-undefined render must fail the whole resolve",
            );
            assert!(
                err.value(py).is_instance_of::<PromptRenderError>(),
                "a strict-undefined render maps to PromptRenderError, got {:?}",
                err.value(py).get_type().name().unwrap()
            );
            // Suppress unused-import warning for RenderResult (imported for doc context).
            let _ = std::mem::size_of::<RenderResult>();
        });
    }
}
