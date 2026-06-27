//! The Python [`Registry`] `#[pyclass]` — a thin wrapper over the Rust consumer's
//! [`prompting_press::Registry`] (FR-005..008a, research D3).
//!
//! The wrapper holds the consumer registry and forwards to it. All three population paths —
//! [`load_yaml`](Registry::load_yaml), [`load_json`](Registry::load_json), and
//! [`insert`](Registry::insert) — normalize through the **one** consumer loader (Q3 / FR-005):
//! the binding never parses YAML/JSON itself, so accept/reject behavior and YAML↔JSON parity are
//! structural properties of the shared core (Principle I), with no Python YAML dependency. The
//! text/object is marshaled across FFI to [`prompting_press::Registry::load_json`] /
//! [`load_yaml`](prompting_press::Registry::load_yaml); a malformed document maps to
//! [`LoadError`](crate::error::LoadError) and inserts **nothing** (FR-007, guaranteed by the
//! consumer). The internal accessor lets the render/check/compose modules resolve a prompt by
//! name (a missing name becomes an [`UnknownPromptError`](crate::error::UnknownPromptError) at the
//! call site, never a panic across FFI).

use pyo3::prelude::*;

use crate::error::consumer_error_to_pyerr;
use prompting_press::ConsumerError;
// `PromptDefinition` is referenced only by the `#[cfg(test)]` `from_defs_for_test` seeder (the
// public Python paths all flow through the consumer loader as text/JSON), so gate its import to
// test builds to keep the release build warning-clean.
#[cfg(test)]
use prompting_press::PromptDefinition;

/// Python `Registry`: a library-owned map of prompt name → loaded definition.
///
/// Wraps [`prompting_press::Registry`] (BTreeMap-backed → deterministic `check` order). The
/// inner consumer registry is the single source of truth; this type adds only the PyO3
/// facade (C-02: marshaling + facade, no engine logic).
#[pyclass(name = "Registry", module = "prompting_press")]
pub struct Registry {
    inner: prompting_press::Registry,
}

#[pymethods]
impl Registry {
    /// `Registry()` — create an empty registry.
    #[new]
    fn new() -> Self {
        Self {
            inner: prompting_press::Registry::new(),
        }
    }

    /// `load_yaml(text)` — load a prompt definition from an already-read **YAML** document and
    /// insert it, keyed by its `name` (an existing entry with the same name is replaced).
    ///
    /// The binding does **not** parse YAML itself: the text is marshaled across FFI to the Rust
    /// consumer's [`load_yaml`](prompting_press::Registry::load_yaml), so accept/reject behavior
    /// and YAML↔JSON parity are structural properties of the shared core (Q3 / Principle I) —
    /// there is no Python YAML dependency. A malformed document (bad YAML, or a shape violation)
    /// raises [`LoadError`](crate::error::LoadError) and inserts **nothing** (FR-007, guaranteed
    /// by the consumer).
    ///
    /// # Errors
    ///
    /// Raises [`LoadError`](crate::error::LoadError) if `text` is not valid YAML or does not
    /// match the prompt-definition shape.
    fn load_yaml(&mut self, py: Python<'_>, text: &str) -> PyResult<()> {
        self.inner
            .load_yaml(text)
            .map(|_| ())
            .map_err(|e| consumer_error_to_pyerr(py, e))
    }

    /// `load_json(text)` — load a prompt definition from an already-read **JSON** document and
    /// insert it, keyed by its `name` (an existing entry with the same name is replaced).
    ///
    /// As with [`load_yaml`](Registry::load_yaml), the text is marshaled to the consumer's
    /// [`load_json`](prompting_press::Registry::load_json) — the binding parses nothing. A
    /// malformed document raises [`LoadError`](crate::error::LoadError) and inserts **nothing**
    /// (FR-007).
    ///
    /// # Errors
    ///
    /// Raises [`LoadError`](crate::error::LoadError) if `text` is not valid JSON or does not
    /// match the prompt-definition shape.
    fn load_json(&mut self, py: Python<'_>, text: &str) -> PyResult<()> {
        self.inner
            .load_json(text)
            .map(|_| ())
            .map_err(|e| consumer_error_to_pyerr(py, e))
    }

    /// `insert(definition)` — insert a constructed prompt definition, keyed by its `name` (an
    /// existing entry with the same name is replaced).
    ///
    /// `definition` is the **constructed-object** input form (FR-005, third path; contract
    /// `python-api.md`). Two shapes are accepted, both normalizing through the **same** consumer
    /// loader as the text paths (Q3 / FR-008 — one accept/reject contract, no parallel shape):
    ///
    /// - a generated Pydantic **`PromptDefinition` instance** (duck-typed: anything exposing a
    ///   callable `model_dump_json`). Its `model_dump_json(exclude_none=True)` is taken directly
    ///   as the JSON text — Pydantic already emits schema-correct JSON, and `exclude_none` drops
    ///   the unset optionals (so an `Option`-typed-but-`null` field never reaches the kernel's
    ///   `deny_unknown_fields` map fields, which reject an explicit `null`).
    /// - a plain `dict` / Mapping (e.g. the result of `model_dump(mode="json", exclude_none=True)`)
    ///   — `depythonize`d into a lossless `serde_json::Value`, then re-serialized to JSON.
    ///
    /// Either shape is then handed to [`load_json`](prompting_press::Registry::load_json).
    ///
    /// # Errors
    ///
    /// Raises [`LoadError`](crate::error::LoadError) if `definition` does not match the
    /// prompt-definition shape (a missing required field, an unknown field, a type mismatch);
    /// nothing is inserted (FR-007).
    fn insert(&mut self, py: Python<'_>, definition: &Bound<'_, PyAny>) -> PyResult<()> {
        let json = self.definition_to_json(py, definition)?;
        self.inner
            .load_json(&json)
            .map(|_| ())
            .map_err(|e| consumer_error_to_pyerr(py, e))
    }
}

impl Registry {
    /// Reduce a constructed-object `insert` argument to a JSON string for the consumer loader.
    ///
    /// A Pydantic model instance (duck-typed: has a callable `model_dump_json`) yields its JSON
    /// directly via `model_dump_json(exclude_none=True)`; any other object (a `dict` / Mapping) is
    /// `depythonize`d into a `serde_json::Value` and re-serialized. Both keep the constructed-object
    /// path on the one consumer loader (Q3 / FR-008). Any failure maps to `LoadError`, never a
    /// panic across FFI.
    fn definition_to_json(
        &self,
        py: Python<'_>,
        definition: &Bound<'_, PyAny>,
    ) -> PyResult<String> {
        // A Pydantic model instance: take its own JSON. `model_dump_json` emits schema-correct
        // JSON; `exclude_none=True` drops unset optionals so a `null` never lands on a kernel map
        // field (which `deny_unknown_fields` would reject). Duck-typed so the binding does not
        // import pydantic and any future model with the same method works.
        if definition
            .hasattr("model_dump_json")
            .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?
        {
            let dumper = definition
                .getattr("model_dump_json")
                .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
            if dumper.is_callable() {
                let kwargs = pyo3::types::PyDict::new(py);
                kwargs
                    .set_item("exclude_none", true)
                    .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
                // SEC-004 (review L-3): a `model_dump_json` failure can carry value-bearing text
                // (a serialization error naming the offending value). Withhold the raw detail —
                // emit a fixed message, never fold `e.to_string()` into the surfaced LoadError.
                let json_obj = dumper.call((), Some(&kwargs)).map_err(|_| {
                    consumer_error_to_pyerr(
                        py,
                        ConsumerError::Load(
                            "could not serialize the prompt definition".to_string(),
                        ),
                    )
                })?;
                let json: String = json_obj
                    .extract::<String>()
                    .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
                return Ok(json);
            }
        }

        // Otherwise a plain dict / Mapping: depythonize → lossless `serde_json::Value` → JSON
        // text. The same lossless serde intermediate the marshaling bridge uses.
        let value: serde_json::Value = pythonize::depythonize(definition)
            .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))?;
        serde_json::to_string(&value)
            .map_err(|e| consumer_error_to_pyerr(py, ConsumerError::Load(e.to_string())))
    }
}

impl Registry {
    /// Borrow the inner consumer registry, for the render/check/compose modules to resolve a
    /// prompt by name. A missing name is mapped to `UnknownPromptError` at the call site
    /// (FR-008a) — this accessor itself never panics. Called from `render`/`get_source`
    /// (render.rs), `check` (check.rs), and `Composition::resolve` (compose.rs).
    pub(crate) fn inner(&self) -> &prompting_press::Registry {
        &self.inner
    }

    /// Build a `Registry` pre-populated with the given already-constructed
    /// [`PromptDefinition`]s, for the sibling modules' `#[cfg(test)]` render/check/compose tests.
    ///
    /// Test-only: the public Python path populates a registry via `insert` / the US2 loaders;
    /// this bypasses the PyO3 extraction so a Rust test can seed kernel `PromptDefinition`s
    /// directly (the generated newtypes make a struct literal awkward, so tests build each def
    /// from JSON and hand them here).
    #[cfg(test)]
    pub(crate) fn from_defs_for_test(defs: impl IntoIterator<Item = PromptDefinition>) -> Self {
        let mut inner = prompting_press::Registry::new();
        for def in defs {
            inner.insert(def);
        }
        Self { inner }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::LoadError;
    use pyo3::types::PyDict;

    /// `insert` of a constructed definition (as a Python dict matching the shape) round-trips:
    /// the inner consumer registry resolves it by name via the internal accessor. This exercises
    /// the Q3 path — the dict is depythonized → JSON → the consumer's `load_json`.
    #[test]
    fn insert_then_inner_get_round_trips() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "greet").unwrap();
            dict.set_item("role", "user").unwrap();
            dict.set_item("body", "Hi {{ name }}").unwrap();

            let mut reg = Registry::new();
            reg.insert(py, dict.as_any()).expect("valid shape");

            assert!(reg.inner().get("greet").is_some(), "present after insert");
            assert!(reg.inner().get("absent").is_none());
        });
    }

    /// `insert` of a **Pydantic-like model instance** (the constructed-object form the contract
    /// advertises) routes through `model_dump_json(exclude_none=True)` → the consumer loader, and
    /// resolves by name. The model is duck-typed here (a tiny stub class exposing
    /// `model_dump_json`) so the test needs no pydantic; the real generated-model proof lives in
    /// the Python suite (T013). The stub asserts it is called with `exclude_none=True`, pinning the
    /// kwarg the binding must pass.
    #[test]
    fn insert_pydantic_like_instance_routes_through_model_dump_json() {
        Python::attach(|py| {
            // A stand-in for a generated Pydantic `PromptDefinition`: a `model_dump_json` method
            // that emits the schema-correct JSON and verifies the binding passes exclude_none=True.
            let stub = py
                .eval(
                    cr#"
type("PromptDefinitionStub", (), {
    "model_dump_json": lambda self, exclude_none=False: (
        '{"name": "greet", "role": "user", "body": "Hi {{ name }}"}'
        if exclude_none
        else (_ for _ in ()).throw(AssertionError("binding must pass exclude_none=True"))
    ),
})()
"#,
                    None,
                    None,
                )
                .expect("build a model-like stub instance");

            let mut reg = Registry::new();
            reg.insert(py, &stub).expect("model-like instance accepted");

            assert!(
                reg.inner().get("greet").is_some(),
                "the instance's model_dump_json JSON was loaded"
            );
        });
    }

    /// A malformed definition object raises `LoadError` (does not panic across FFI) and inserts
    /// nothing (FR-007) — the constructed-object path now shares the loader's accept/reject
    /// contract (Q3).
    #[test]
    fn insert_malformed_object_errors() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "greet").unwrap();
            // missing required `role` / `body`
            let mut reg = Registry::new();
            let res = reg.insert(py, dict.as_any());
            assert!(
                res.is_err(),
                "missing required fields must error, not panic"
            );
            let err = res.unwrap_err();
            assert!(
                err.value(py).is_instance_of::<LoadError>(),
                "a shape violation maps to LoadError"
            );
            // FR-007: nothing was inserted.
            assert!(reg.inner().get("greet").is_none(), "no partial load");
        });
    }

    /// `load_json(text)` of a valid JSON document inserts the definition, resolvable by name.
    #[test]
    fn load_json_inserts_and_resolves() {
        Python::attach(|py| {
            let mut reg = Registry::new();
            reg.load_json(
                py,
                r#"{"name": "greet", "role": "user", "body": "Hi {{ x }}"}"#,
            )
            .expect("valid json document");
            assert!(reg.inner().get("greet").is_some());
        });
    }

    /// `load_yaml(text)` of a valid YAML document inserts the definition, resolvable by name.
    #[test]
    fn load_yaml_inserts_and_resolves() {
        Python::attach(|py| {
            let mut reg = Registry::new();
            reg.load_yaml(py, "name: greet\nrole: user\nbody: \"Hi {{ x }}\"\n")
                .expect("valid yaml document");
            assert!(reg.inner().get("greet").is_some());
        });
    }

    /// YAML↔JSON parity is a structural property of the shared core (Q3 / Principle I): the
    /// equivalent JSON and YAML documents, loaded via the consumer through this binding, produce
    /// the **same** stored definition — byte-identical template source — so the binding adds no
    /// behavior. We drive both through the `#[pyclass]` methods, then compare the consumer's
    /// stored result.
    #[test]
    fn load_yaml_and_load_json_of_equivalent_docs_agree() {
        Python::attach(|py| {
            let json = r#"{"name": "greet", "role": "user", "body": "Hi {{ x }}"}"#;
            let yaml = "name: greet\nrole: user\nbody: \"Hi {{ x }}\"\n";

            let mut from_json = Registry::new();
            from_json.load_json(py, json).expect("valid json");
            let mut from_yaml = Registry::new();
            from_yaml.load_yaml(py, yaml).expect("valid yaml");

            let j = from_json.inner().get("greet").expect("json def present");
            let y = from_yaml.inner().get("greet").expect("yaml def present");

            // The consumer treats them identically: re-serializing each stored definition yields
            // the same JSON (a structural-equality check that does not depend on the binding).
            let j_json = serde_json::to_string(j).expect("serialize json-loaded def");
            let y_json = serde_json::to_string(y).expect("serialize yaml-loaded def");
            assert_eq!(
                j_json, y_json,
                "YAML and JSON loads must agree (structural parity)"
            );
        });
    }

    /// A malformed JSON document raises `LoadError` and inserts nothing (FR-007).
    #[test]
    fn load_json_malformed_errors_and_inserts_nothing() {
        Python::attach(|py| {
            let mut reg = Registry::new();
            // missing required `body`
            let res = reg.load_json(py, r#"{"name": "greet", "role": "user"}"#);
            assert!(res.is_err(), "missing required field must error");
            assert!(
                res.unwrap_err().value(py).is_instance_of::<LoadError>(),
                "malformed document maps to LoadError"
            );
            assert!(
                reg.inner().get("greet").is_none(),
                "no partial load (FR-007)"
            );
        });
    }
}
