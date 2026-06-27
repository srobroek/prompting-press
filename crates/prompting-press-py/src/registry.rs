//! The Python [`Registry`] `#[pyclass]` — a thin wrapper over the Rust consumer's
//! [`prompting_press::Registry`] (FR-005..008a, research D3).
//!
//! The wrapper holds the consumer registry and forwards to it. Loading (`load_yaml` /
//! `load_json`) is **not** implemented in this foundational phase — it is US2 work (a later
//! task) and is left as an explicit stub below so it slots in without reshaping this type.
//! What *is* here now: construction, `insert` of a constructed definition, and an internal
//! accessor the render/check/compose modules use to resolve a prompt by name (a missing name
//! becomes an [`UnknownPromptError`](crate::error::UnknownPromptError) at the call site, never
//! a panic across FFI).

use pyo3::prelude::*;
use pyo3::Borrowed;

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

    /// `insert(definition)` — insert an already-marshaled [`PromptDefinition`], keyed by its
    /// `name` (an existing entry with the same name is replaced).
    ///
    /// This foundational-phase signature takes the kernel's `PromptDefinition` directly. The
    /// US2 caller-facing path (a generated Pydantic `PromptDefinition` → `model_dump_json` →
    /// consumer `load_json`) layers on top of the loader stubs below; this method is the
    /// constructed-object insert the consumer already supports.
    fn insert(&mut self, definition: PromptDefinitionArg) {
        self.inner.insert(definition.0);
    }

    // ----------------------------------------------------------------------------------
    // US2 (later task): the dual-input loaders. Intentionally NOT implemented here.
    //
    //   fn load_yaml(&mut self, text: &str) -> PyResult<()> { ... consumer.load_yaml ... }
    //   fn load_json(&mut self, text: &str) -> PyResult<()> { ... consumer.load_json ... }
    //
    // Both will marshal the text to `prompting_press::Registry::{load_yaml,load_json}` and map
    // a `ConsumerError::Load` through `crate::error::consumer_error_to_pyerr` → `LoadError`.
    // Left as a stub so render/check (US1/US3) can resolve names against an insert-populated
    // registry now, and US2 slots the loaders in without reshaping this pyclass.
    // ----------------------------------------------------------------------------------
}

impl Registry {
    /// Borrow the inner consumer registry, for the render/check/compose modules to resolve a
    /// prompt by name. A missing name is mapped to `UnknownPromptError` at the call site
    /// (FR-008a) — this accessor itself never panics.
    ///
    /// `allow(dead_code)`: this accessor's only callers are the render (US1), check (US3), and
    /// compose (US4) modules, which land in later tasks. It is deliberately present now so those
    /// tasks slot in without reshaping this pyclass.
    #[allow(dead_code)]
    pub(crate) fn inner(&self) -> &prompting_press::Registry {
        &self.inner
    }
}

/// Newtype wrapper letting `insert` accept a `PromptDefinition` extracted from Python.
///
/// `PromptDefinition` is the kernel's generated serde struct; PyO3 cannot derive `FromPyObject`
/// for it directly here, so this phase extracts it from a Python object via `depythonize`
/// through the marshaling bridge's serde path. (The richer Pydantic-instance path is US2.)
struct PromptDefinitionArg(PromptDefinition);

impl<'a, 'py> FromPyObject<'a, 'py> for PromptDefinitionArg {
    type Error = PyErr;

    fn extract(obj: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        let def: PromptDefinition = pythonize::depythonize(&obj).map_err(|e| {
            // A shape mismatch here is a constructed-object problem; surface it as a value
            // error (US2 will route this through LoadError once the Pydantic path lands).
            pyo3::exceptions::PyValueError::new_err(format!(
                "could not read PromptDefinition from object: {e}"
            ))
        })?;
        Ok(Self(def))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyo3::types::PyDict;

    /// `insert` of a constructed definition (as a Python dict matching the shape) round-trips:
    /// the inner consumer registry resolves it by name via the internal accessor.
    #[test]
    fn insert_then_inner_get_round_trips() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "greet").unwrap();
            dict.set_item("role", "user").unwrap();
            dict.set_item("body", "Hi {{ name }}").unwrap();

            let arg: PromptDefinitionArg = dict.as_any().extract().expect("valid shape");
            let mut reg = Registry::new();
            reg.insert(arg);

            assert!(reg.inner().get("greet").is_some(), "present after insert");
            assert!(reg.inner().get("absent").is_none());
        });
    }

    /// A malformed definition object raises (does not panic across FFI).
    #[test]
    fn insert_malformed_object_errors() {
        Python::attach(|py| {
            let dict = PyDict::new(py);
            dict.set_item("name", "greet").unwrap();
            // missing required `role` / `body`
            let res = dict.as_any().extract::<PromptDefinitionArg>();
            assert!(
                res.is_err(),
                "missing required fields must error, not panic"
            );
        });
    }
}
