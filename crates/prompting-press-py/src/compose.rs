//! Multi-message composition for the Python binding (spec 004, US4; T020; FR-012/FR-013).
//!
//! A [`Composition`] is an **explicit, ordered** sequence of `(prompt-name, vars, variant)`
//! entries that [`resolve`](Composition::resolve)s — in append order — to a `list[Message]`,
//! where each [`Message`] is the named prompt rendered with its own validated vars and tagged
//! with that prompt definition's `role` (FR-012). It is the few-shot / system+user sequence
//! builder. Construction is `Composition()` + [`append`](Composition::append), or the
//! [`from_messages`](Composition::from_messages) bulk constructor — there is deliberately **no**
//! fluent `.chain()` API (FR-013; it cannot cross the PyO3 boundary and collides with
//! `Iterator::chain`).
//!
//! ## Why a binding-OWNED `Composition` (critique E1 / C-01)
//!
//! The Rust consumer's [`prompting_press::Composition`] is generic over `V: Serialize + Validate`
//! — a **garde** type. This binding has no such type: validation is owned in **Python** (against
//! the caller's Pydantic Vars model — Q1), so there is no `V` to instantiate the consumer's
//! `Composition` with. Therefore this module owns its **own** `Composition` `#[pyclass]` holding
//! already-marshaled entries, and [`resolve`](Composition::resolve) calls the **kernel directly**
//! per entry — exactly mirroring how the binding's [`render`](crate::render::render) works (US1).
//! This is still **zero engine logic** (Principle I): the kernel renders; the binding only
//! validates (in Python), marshals, and surfaces results. Render byte-parity with the
//! Rust/TS bindings stays structural because each entry's value is built by the same
//! validate → [`to_kernel_value`](crate::marshal::to_kernel_value) path that single-render uses.
//!
//! ## Eager validation at `append` — option (a) (no partial state)
//!
//! Each entry is validated + marshaled **eagerly at `append`** (mirroring spec-003's option (a)):
//! the Pydantic vars are validated NOW via the US1 Python-validation path
//! ([`render::validate_in_python`](crate::render)), the validated payload is marshaled to the
//! kernel's [`minijinja::Value`], and only then is the entry stored. On a validation failure the
//! call raises [`PromptValidationError`](crate::error) and **stores nothing** — the composition is
//! left exactly as it was, so [`resolve`](Composition::resolve) can only ever see fully-validated
//! entries. The library's central guarantee (validation always runs; a partial result is never
//! returned as success) is upheld incrementally as the composition is built.
//!
//! ## The entry vars shape — a Pydantic model **instance** per entry
//!
//! A composition entry's `vars` is a constructed Pydantic model **instance** (the natural fit for
//! a list of `(name, vars)` entries). It is validated via the same instance path `render` uses
//! when `data is None`: `type(vars).model_validate(vars.model_dump(mode="json"))` — so even an
//! instance built with `model_construct` (validation-skipped) is re-checked here. The class +
//! `data` two-argument form `render` also accepts is intentionally **not** offered for entries:
//! a list of constructed instances is the one clean idiom for composition, and a single shape per
//! concern keeps the surface narrow (Scope Discipline).
//!
//! ## resolve: prompt resolution + render, in order
//!
//! [`resolve`](Composition::resolve) walks the stored entries in append order. For each it
//! resolves the prompt by name against the [`Registry`](crate::registry::Registry) (absent ⇒
//! [`UnknownPromptError`](crate::error), never a panic) and delegates rendering to the kernel via
//! [`prompting_press_core::render`] with the entry's pre-validated value. Each result becomes
//! `Message { role: <def.role stringified>, text: result.text }`. One entry's failure (unknown
//! prompt, unknown variant, a strict-undefined reference) propagates as the mapped Python
//! exception and the partial result built so far is **discarded** — never returned as success. An
//! empty composition resolves to `[]`.

use pyo3::prelude::*;
use pyo3::types::PyTuple;

use prompting_press::ConsumerError;
use prompting_press_core::GuardConfig as KernelGuardConfig;

use crate::error::{consumer_error_to_pyerr, kernel_error_to_pyerr};
use crate::marshal::to_kernel_value;
use crate::registry::Registry;
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
/// The vars are already marshaled into a [`minijinja::Value`] (the same type the kernel renders
/// against), so the `Vec` of entries is homogeneous despite each entry's source Pydantic model
/// differing. The entry holds only the data the kernel needs at render time.
struct Entry {
    /// The prompt's registry name (resolved at `resolve`, not at `append`).
    name: String,
    /// The pre-validated, marshaled vars (the FFI bridge value — FR-003a), ready for the kernel.
    values: minijinja::Value,
    /// The selected variant (`None` ⇒ the reserved `default` / root body).
    variant: Option<String>,
}

/// An explicit, ordered sequence of `(prompt-name, vars, variant)` entries that resolves to a
/// `list[Message]` in append order (FR-012). Built with `Composition()` + [`append`](Self::append)
/// or [`from_messages`](Self::from_messages); there is **no** fluent `.chain()` (FR-013).
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
    /// `(name, vars)` or `(name, vars, variant)` tuples, eager-validating each in order.
    ///
    /// `entries` is any Python iterable of 2- or 3-element tuples (a list of tuples is the
    /// idiomatic form). `vars` is a constructed Pydantic model **instance** (see module docs);
    /// `variant` defaults to `None` (the reserved `default` arm) for the 2-tuple form.
    ///
    /// Each entry is validated + marshaled via the same path [`append`](Self::append) uses, in
    /// order. The first invalid entry raises [`PromptValidationError`](crate::error) and the
    /// whole construction fails — **no** `Composition` is returned (no partial state).
    ///
    /// # Errors
    /// - [`PromptValidationError`](crate::error) — Pydantic rejected an entry's vars.
    /// - `TypeError` — an entry is not a 2- or 3-element tuple/sequence.
    #[staticmethod]
    fn from_messages(py: Python<'_>, entries: &Bound<'_, PyAny>) -> PyResult<Self> {
        let mut composition = Self::new();
        for item in entries.try_iter()? {
            let item = item?;
            let (name, vars, variant) = unpack_entry(&item)?;
            // Eager-validate + marshal + store, exactly as `append` does. A failure here raises
            // and discards the partially-built composition — nothing is returned (no partial
            // state), since `composition` is a local that is dropped on the `?` early-return.
            composition.append_entry(py, &name, &vars, variant.as_deref())?;
        }
        Ok(composition)
    }

    /// `append(name, vars, variant=None)` — eager-validate + marshal + store one entry.
    ///
    /// `vars` is a constructed Pydantic model **instance** (see module docs); it is validated
    /// **now** via the US1 Python-validation path and marshaled to the kernel's value type before
    /// the entry is stored. On a validation failure this raises
    /// [`PromptValidationError`](crate::error) and **stores nothing** — the composition is left
    /// exactly as it was (no partial state), so a later [`resolve`](Self::resolve) never sees a
    /// half-validated entry. The prompt `name` is **not** resolved here — an unknown name surfaces
    /// at [`resolve`](Self::resolve) as [`UnknownPromptError`](crate::error).
    ///
    /// Takes `&mut self` and returns `None` (not `self`): the builder is intentionally **not**
    /// fluent/chainable (FR-013).
    ///
    /// # Errors
    /// [`PromptValidationError`](crate::error) — Pydantic rejected `vars`. The entry is not stored.
    #[pyo3(signature = (name, vars, *, variant=None))]
    fn append(
        &mut self,
        py: Python<'_>,
        name: &str,
        vars: &Bound<'_, PyAny>,
        variant: Option<&str>,
    ) -> PyResult<()> {
        self.append_entry(py, name, vars, variant)
    }

    /// `len(composition)` — the number of appended entries (== the resolved-message count on
    /// success).
    fn __len__(&self) -> usize {
        self.entries.len()
    }

    /// `repr(composition)` — a compact, fixed-shape rendering naming only the entry count and the
    /// ordered prompt names (caller-supplied identifiers, not bound-value content). The marshaled
    /// vars are never surfaced.
    fn __repr__(&self) -> String {
        let names: Vec<&str> = self.entries.iter().map(|e| e.name.as_str()).collect();
        format!("Composition(entries={}, names={:?})", names.len(), names)
    }

    /// `resolve(registry)` — resolve the composition to an ordered `list[Message]` (FR-012),
    /// rendering each entry — in append order — through the kernel.
    ///
    /// For each entry, in order: resolve the prompt by name against `reg` (absent ⇒
    /// [`UnknownPromptError`](crate::error), never a panic), then delegate rendering to
    /// [`prompting_press_core::render`] **directly** (critique E1 / C-01) with the entry's
    /// **pre-validated** value (vars were validated at [`append`](Self::append)). The render
    /// result becomes `Message { role: <def.role stringified>, text: result.text }`. Composition
    /// uses no guard expansion — a default [`GuardConfig`](prompting_press_core::GuardConfig) is
    /// passed, which leaves `text` unchanged.
    ///
    /// One entry's render failure (unknown prompt, unknown variant, a strict-undefined reference,
    /// a parse/render error) propagates as the mapped Python exception and the partial result
    /// built so far is **discarded** — never returned as success. An empty composition resolves
    /// to `[]`.
    ///
    /// # Errors
    /// - [`UnknownPromptError`](crate::error) — an entry's name is absent from `reg`.
    /// - [`PromptRenderError`](crate::error) — the kernel rejected an entry's render (unknown
    ///   variant, a strict-undefined reference, a parse/render failure). `parse`/`render` detail
    ///   is scrubbed (SEC-004).
    fn resolve(&self, py: Python<'_>, reg: &Registry) -> PyResult<Vec<Message>> {
        let mut messages = Vec::with_capacity(self.entries.len());

        for entry in &self.entries {
            // Resolve the prompt by name (absent ⇒ structured error, never a panic).
            let Some(def) = reg.inner().get(&entry.name) else {
                return Err(consumer_error_to_pyerr(
                    py,
                    ConsumerError::UnknownPrompt(entry.name.clone()),
                ));
            };

            // Render by calling the KERNEL DIRECTLY (critique E1 / C-01) with the already-validated
            // value. Composition does no guard expansion; a default GuardConfig leaves `text`
            // unchanged. A kernel failure propagates here, discarding the partial `messages`
            // built so far — no partial-as-success.
            let result = prompting_press_core::render(
                def,
                entry.variant.as_deref(),
                entry.values.clone(),
                &KernelGuardConfig::default(),
            )
            .map_err(|e| kernel_error_to_pyerr(py, e))?;

            messages.push(Message {
                // The prompt definition's role, stringified via its `Display` impl.
                role: def.role.to_string(),
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
    /// `type(vars).model_validate(vars.model_dump(mode="json"))`), marshals the validated payload
    /// to the kernel's value type, and only then pushes the entry. On any failure (a
    /// `PromptValidationError` from validation, or a `LoadError` from marshaling) the error
    /// propagates and **nothing is stored** — `self.entries` is mutated only on the success path.
    fn append_entry(
        &mut self,
        py: Python<'_>,
        name: &str,
        vars: &Bound<'_, PyAny>,
        variant: Option<&str>,
    ) -> PyResult<()> {
        // Validate in Python, BEFORE marshaling (FR-002 / Q1). `data = None` selects the instance
        // path — the same one `render` uses for a constructed model. A ValidationError surfaces as
        // PromptValidationError; the validated, JSON-dumped payload is returned on success.
        let dumped = validate_in_python(py, vars, None)?;

        // Marshal the validated payload through the single FFI value bridge (FR-003a). A shape the
        // serde data model cannot represent surfaces as a LoadError (never bound-value content).
        let values = to_kernel_value(&dumped).map_err(|e| consumer_error_to_pyerr(py, e))?;

        // Only on full success do we mutate state — no partial entry is ever stored.
        self.entries.push(Entry {
            name: name.to_string(),
            values,
            variant: variant.map(str::to_string),
        });
        Ok(())
    }
}

/// Unpack one `from_messages` entry into `(name, vars, variant)`.
///
/// Accepts a 2-element `(name, vars)` (variant defaults to `None`) or a 3-element
/// `(name, vars, variant)` tuple/sequence. `variant` may be `None` even in the 3-tuple form
/// (explicit "use the default arm"). A wrong arity or a non-string `name`/`variant` raises a
/// `TypeError` — a caller-API misuse, not a validation failure, so it is surfaced as-is rather
/// than masquerading as a `PromptValidationError`.
fn unpack_entry<'py>(
    item: &Bound<'py, PyAny>,
) -> PyResult<(String, Bound<'py, PyAny>, Option<String>)> {
    let tuple = item.cast::<PyTuple>().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "composition entry must be a tuple of (name, vars) or (name, vars, variant)",
        )
    })?;
    let len = tuple.len();
    if !(2..=3).contains(&len) {
        return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
            "composition entry must have 2 or 3 elements (name, vars[, variant]), got {len}"
        )));
    }
    let name: String = tuple.get_item(0)?.extract()?;
    let vars = tuple.get_item(1)?;
    let variant: Option<String> = if len == 3 {
        tuple.get_item(2)?.extract()?
    } else {
        None
    };
    Ok((name, vars, variant))
}

#[cfg(test)]
mod tests {
    //! Composition coverage that is drivable in Rust WITHOUT a Pydantic model.
    //!
    //! The eager-validate-at-append behavior (a real Pydantic model, `PromptValidationError` on
    //! invalid vars, "nothing stored on failure"), the resolved message ORDER, and the
    //! no-partial-success guarantee across a multi-entry resolve all need a Python Pydantic model
    //! and are covered Python-side in T019. Here we exercise the parts that need no Pydantic: an
    //! empty composition resolves to `[]`, and an unknown-name entry surfaces as
    //! `UnknownPromptError` at `resolve` (via the inner-registry resolution) — i.e. that the
    //! kernel-direct resolve loop and its error mapping are wired correctly.

    use super::*;
    use prompting_press::PromptDefinition;

    use crate::error::UnknownPromptError;

    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    /// An empty composition resolves to an empty list (the `[]` edge case), with no registry
    /// lookups performed.
    #[test]
    fn empty_composition_resolves_to_empty() {
        Python::attach(|py| {
            let comp = Composition::new();
            assert_eq!(comp.__len__(), 0, "a fresh composition has no entries");

            let reg = Registry::from_defs_for_test([]);
            let messages = comp.resolve(py, &reg).expect("empty resolve");
            assert!(messages.is_empty(), "empty composition ⇒ empty list");
        });
    }

    /// An entry naming a prompt absent from the registry surfaces as `UnknownPromptError` at
    /// `resolve` (FR-008a) — a loud, structured error, never a panic across FFI. The entry is
    /// constructed directly (bypassing append's Python validation) to drive the resolve loop's
    /// name-resolution branch without a Pydantic model.
    #[test]
    fn unknown_name_entry_is_loud_at_resolve() {
        Python::attach(|py| {
            // A registry with a different prompt, so the lookup genuinely misses.
            let present = def_from_json(r#"{ "name": "present", "role": "user", "body": "hi" }"#);
            let reg = Registry::from_defs_for_test([present]);

            // An entry referencing an absent name, with a trivial (empty) marshaled value — the
            // name is resolved BEFORE the value is used, so the value is irrelevant to this path.
            let comp = Composition {
                entries: vec![Entry {
                    name: "absent".to_string(),
                    values: minijinja::Value::from_serialize(serde_json::json!({})),
                    variant: None,
                }],
            };

            let err = comp
                .resolve(py, &reg)
                .expect_err("an unknown-name entry must error at resolve");
            assert!(
                err.value(py).is_instance_of::<UnknownPromptError>(),
                "an absent prompt name maps to UnknownPromptError, got {:?}",
                err.value(py).get_type().name().unwrap()
            );
        });
    }

    /// `resolve` renders each entry in append order through the kernel and tags it with the
    /// prompt's role. The entries are constructed directly (already-marshaled values, bypassing
    /// the Python validation `append` performs) so this drives the kernel-direct resolve loop +
    /// the `role` stringification + the message ORDER without a Pydantic model. (Full
    /// validate-at-append behavior is covered Python-side in T019.)
    #[test]
    fn resolve_renders_in_order_with_roles() {
        Python::attach(|py| {
            let system = def_from_json(
                r#"{ "name": "sys", "role": "system", "body": "You are {{ persona }}." }"#,
            );
            let user =
                def_from_json(r#"{ "name": "ask", "role": "user", "body": "Question: {{ q }}" }"#);
            let reg = Registry::from_defs_for_test([system, user]);

            let comp = Composition {
                entries: vec![
                    Entry {
                        name: "sys".to_string(),
                        values: minijinja::Value::from_serialize(
                            serde_json::json!({ "persona": "a helpful assistant" }),
                        ),
                        variant: None,
                    },
                    Entry {
                        name: "ask".to_string(),
                        values: minijinja::Value::from_serialize(
                            serde_json::json!({ "q": "why?" }),
                        ),
                        variant: None,
                    },
                ],
            };

            let messages = comp.resolve(py, &reg).expect("resolve succeeds");
            assert_eq!(messages.len(), 2, "one message per entry");

            // Append order is preserved (FR-012).
            assert_eq!(messages[0].role, "system");
            assert_eq!(messages[0].text, "You are a helpful assistant.");
            assert_eq!(messages[1].role, "user");
            assert_eq!(messages[1].text, "Question: why?");
        });
    }

    /// A later entry's render failure propagates as the mapped exception and DISCARDS the partial
    /// result — `resolve` returns `Err`, never a truncated `list[Message]` (no partial-as-success).
    /// The second entry references a prompt whose body needs a root the marshaled value lacks,
    /// driving the kernel's strict-undefined path; the first entry alone would have succeeded.
    #[test]
    fn one_entry_failure_discards_partial_result() {
        Python::attach(|py| {
            let ok = def_from_json(r#"{ "name": "ok", "role": "user", "body": "fine" }"#);
            let needs = def_from_json(
                r#"{ "name": "needs", "role": "user", "body": "Hello {{ missing }}!" }"#,
            );
            let reg = Registry::from_defs_for_test([ok, needs]);

            let comp = Composition {
                entries: vec![
                    Entry {
                        name: "ok".to_string(),
                        values: minijinja::Value::from_serialize(serde_json::json!({})),
                        variant: None,
                    },
                    Entry {
                        // No `missing` in the value ⇒ strict-undefined kernel error.
                        name: "needs".to_string(),
                        values: minijinja::Value::from_serialize(serde_json::json!({})),
                        variant: None,
                    },
                ],
            };

            let err = comp.resolve(py, &reg).expect_err(
                "the second entry's strict-undefined render must fail the whole resolve",
            );
            // It is a render-time failure, surfaced as a PromptingPressError subtype — the partial
            // (first-entry) result is NOT returned as success.
            assert!(
                err.value(py)
                    .is_instance_of::<crate::error::PromptRenderError>(),
                "a strict-undefined render maps to PromptRenderError, got {:?}",
                err.value(py).get_type().name().unwrap()
            );
        });
    }
}
