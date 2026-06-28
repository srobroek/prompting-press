//! The Node [`Registry`] `#[napi]` class — a thin wrapper over the Rust consumer's
//! [`prompting_press::Registry`] (FR-005..008a, research D3).
//!
//! The wrapper holds the consumer registry and forwards to it. All three population paths —
//! [`load_yaml`](Registry::load_yaml) (`loadYaml`), [`load_json`](Registry::load_json)
//! (`loadJson`), and [`insert`](Registry::insert) — normalize through the **one** consumer loader
//! (Q3 / FR-005): the binding never parses YAML/JSON itself, so accept/reject behavior and
//! YAML↔JSON parity are structural properties of the shared core (Principle I), with no JS YAML
//! dependency. The text/object is marshaled across napi to
//! [`prompting_press::Registry::load_json`] / [`load_yaml`](prompting_press::Registry::load_yaml);
//! a malformed document maps to a `load`-coded error and inserts **nothing** (FR-007, guaranteed by
//! the consumer). The internal accessor lets the render/check/compose modules resolve a prompt by
//! name (a missing name becomes an `unknown_prompt` error at the call site, never a panic across
//! napi).

use napi_derive::napi;

use crate::error::consumer_error_to_napi_err;
// `PromptDefinition` is referenced only by the `#[cfg(test)]` `from_defs_for_test` seeder (the
// public JS paths all flow through the consumer loader as text/JSON), so gate its import to
// test builds to keep the release build warning-clean.
#[cfg(test)]
use prompting_press::PromptDefinition;

/// Node `Registry`: a library-owned map of prompt name → loaded definition.
///
/// Wraps [`prompting_press::Registry`] (BTreeMap-backed → deterministic `check` order). The
/// inner consumer registry is the single source of truth; this type adds only the napi facade
/// (C-02: marshaling + facade, no engine logic).
#[napi]
pub struct Registry {
    inner: prompting_press::Registry,
}

#[napi]
impl Registry {
    /// `new Registry()` — create an empty registry.
    #[napi(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: prompting_press::Registry::new(),
        }
    }

    /// `registry.loadYaml(text)` — load a prompt definition from an already-read **YAML** document
    /// and insert it, keyed by its `name` (an existing entry with the same name is replaced).
    ///
    /// The binding does **not** parse YAML itself: the text is marshaled across napi to the Rust
    /// consumer's [`load_yaml`](prompting_press::Registry::load_yaml), so accept/reject behavior and
    /// YAML↔JSON parity are structural properties of the shared core (Q3 / Principle I) — there is
    /// no JS YAML dependency. A malformed document (bad YAML, or a shape violation) throws a
    /// `load`-coded error (→ `LoadError` in the TS facade) and inserts **nothing** (FR-007,
    /// guaranteed by the consumer).
    ///
    /// # Errors
    ///
    /// Throws a `load`-coded napi error if `text` is not valid YAML or does not match the
    /// prompt-definition shape.
    #[napi]
    pub fn load_yaml(&mut self, text: String) -> napi::Result<()> {
        self.inner
            .load_yaml(&text)
            .map(|_| ())
            .map_err(consumer_error_to_napi_err)
    }

    /// `registry.loadJson(text)` — load a prompt definition from an already-read **JSON** document
    /// and insert it, keyed by its `name` (an existing entry with the same name is replaced).
    ///
    /// As with [`load_yaml`](Registry::load_yaml), the text is marshaled to the consumer's
    /// [`load_json`](prompting_press::Registry::load_json) — the binding parses nothing. A malformed
    /// document throws a `load`-coded error and inserts **nothing** (FR-007).
    ///
    /// # Errors
    ///
    /// Throws a `load`-coded napi error if `text` is not valid JSON or does not match the
    /// prompt-definition shape.
    #[napi]
    pub fn load_json(&mut self, text: String) -> napi::Result<()> {
        self.inner
            .load_json(&text)
            .map(|_| ())
            .map_err(consumer_error_to_napi_err)
    }

    /// `registry.insert(definition)` — insert a constructed prompt definition, keyed by its `name`
    /// (an existing entry with the same name is replaced).
    ///
    /// `definition` is the **constructed-object** input form (FR-005, third path): the generated TS
    /// `PromptDefinition` object, which napi decodes into a `serde_json::Value` at the boundary. It
    /// is re-serialized to JSON text and handed to [`load_json`](prompting_press::Registry::load_json),
    /// so the constructed-object path shares the **same** accept/reject contract as the text paths
    /// (Q3 / FR-008 — one loader, one representation, no parallel shape). The binding adds no schema
    /// logic; the consumer/kernel own validation.
    ///
    /// # Errors
    ///
    /// Throws a `load`-coded napi error if `definition` does not match the prompt-definition shape
    /// (a missing required field, an unknown field, a type mismatch); nothing is inserted (FR-007).
    #[napi]
    pub fn insert(&mut self, definition: serde_json::Value) -> napi::Result<()> {
        // Re-serialize the decoded object to JSON text for the one consumer loader. `to_string`
        // over an owned `serde_json::Value` is effectively infallible; a failure maps to a
        // `load`-coded error, never a panic across napi.
        let json = serde_json::to_string(&definition).map_err(|e| {
            consumer_error_to_napi_err(prompting_press::ConsumerError::Load(e.to_string()))
        })?;
        self.inner
            .load_json(&json)
            .map(|_| ())
            .map_err(consumer_error_to_napi_err)
    }
}

impl Default for Registry {
    /// An empty registry — the same value [`Registry::new`] (the JS constructor) produces. Present
    /// so the `pub` napi constructor satisfies `clippy::new_without_default`; the JS surface always
    /// uses `new Registry()`.
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    /// Borrow the inner consumer registry, for the render/check/compose modules to resolve a
    /// prompt by name. A missing name is mapped to an `unknown_prompt` error at the call site
    /// (FR-008a) — this accessor itself never panics. Called from `render`/`getSource`
    /// (render.rs), `check` (check.rs), and `Composition::resolve` (compose.rs).
    pub(crate) fn inner(&self) -> &prompting_press::Registry {
        &self.inner
    }

    /// Build a `Registry` pre-populated with the given already-constructed
    /// [`PromptDefinition`]s, for the sibling modules' `#[cfg(test)]` render/check/compose tests.
    ///
    /// Test-only: the public JS path populates a registry via `insert` / the US2 loaders; this
    /// bypasses the napi decoding so a Rust test can seed kernel `PromptDefinition`s directly (the
    /// generated newtypes make a struct literal awkward, so tests build each def from JSON and hand
    /// them here).
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

    /// Build a `PromptDefinition` from JSON (the idiomatic in-test construction the consumer's
    /// own tests use — the generated newtypes validate, so a struct literal is awkward).
    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    /// `insert` of a constructed definition (as a decoded `serde_json::Value` matching the shape)
    /// round-trips: the inner consumer registry resolves it by name via the internal accessor.
    /// This exercises the Q3 path — the object is re-serialized → JSON → the consumer's `load_json`.
    #[test]
    fn insert_then_inner_get_round_trips() {
        let mut reg = Registry::new();
        reg.insert(serde_json::json!({
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}",
        }))
        .expect("valid shape");

        assert!(reg.inner().get("greet").is_some(), "present after insert");
        assert!(reg.inner().get("absent").is_none());
    }

    /// A malformed definition object throws (does not panic across napi) and inserts nothing
    /// (FR-007) — the constructed-object path shares the loader's accept/reject contract (Q3).
    #[test]
    fn insert_malformed_object_errors() {
        let mut reg = Registry::new();
        // missing required `role` / `body`
        let res = reg.insert(serde_json::json!({ "name": "greet" }));
        assert!(
            res.is_err(),
            "missing required fields must error, not panic"
        );
        let payload: serde_json::Value =
            serde_json::from_str(&res.unwrap_err().reason).expect("json payload");
        assert_eq!(payload["code"], "load", "a shape violation maps to load");
        // FR-007: nothing was inserted.
        assert!(reg.inner().get("greet").is_none(), "no partial load");
    }

    /// `loadJson(text)` of a valid JSON document inserts the definition, resolvable by name.
    #[test]
    fn load_json_inserts_and_resolves() {
        let mut reg = Registry::new();
        reg.load_json(r#"{"name": "greet", "role": "user", "body": "Hi {{ x }}"}"#.to_string())
            .expect("valid json document");
        assert!(reg.inner().get("greet").is_some());
    }

    /// `loadYaml(text)` of a valid YAML document inserts the definition, resolvable by name.
    #[test]
    fn load_yaml_inserts_and_resolves() {
        let mut reg = Registry::new();
        reg.load_yaml("name: greet\nrole: user\nbody: \"Hi {{ x }}\"\n".to_string())
            .expect("valid yaml document");
        assert!(reg.inner().get("greet").is_some());
    }

    /// YAML↔JSON parity is a structural property of the shared core (Q3 / Principle I): the
    /// equivalent JSON and YAML documents, loaded via the consumer through this binding, produce
    /// the **same** stored definition — byte-identical template source — so the binding adds no
    /// behavior. We drive both through the `#[napi]` methods, then compare the consumer's stored
    /// result.
    #[test]
    fn load_yaml_and_load_json_of_equivalent_docs_agree() {
        let json = r#"{"name": "greet", "role": "user", "body": "Hi {{ x }}"}"#;
        let yaml = "name: greet\nrole: user\nbody: \"Hi {{ x }}\"\n";

        let mut from_json = Registry::new();
        from_json.load_json(json.to_string()).expect("valid json");
        let mut from_yaml = Registry::new();
        from_yaml.load_yaml(yaml.to_string()).expect("valid yaml");

        let j = from_json.inner().get("greet").expect("json def present");
        let y = from_yaml.inner().get("greet").expect("yaml def present");

        // The consumer treats them identically: re-serializing each stored definition yields the
        // same JSON (a structural-equality check that does not depend on the binding).
        let j_json = serde_json::to_string(j).expect("serialize json-loaded def");
        let y_json = serde_json::to_string(y).expect("serialize yaml-loaded def");
        assert_eq!(
            j_json, y_json,
            "YAML and JSON loads must agree (structural parity)"
        );
    }

    /// A malformed JSON document throws a `load`-coded error and inserts nothing (FR-007).
    #[test]
    fn load_json_malformed_errors_and_inserts_nothing() {
        let mut reg = Registry::new();
        // missing required `body`
        let res = reg.load_json(r#"{"name": "greet", "role": "user"}"#.to_string());
        assert!(res.is_err(), "missing required field must error");
        let payload: serde_json::Value =
            serde_json::from_str(&res.unwrap_err().reason).expect("json payload");
        assert_eq!(payload["code"], "load", "malformed document maps to load");
        assert!(
            reg.inner().get("greet").is_none(),
            "no partial load (FR-007)"
        );
    }

    /// The test-only seeder builds a registry from constructed `PromptDefinition`s — used by the
    /// render/check/compose `#[cfg(test)]` modules; proven here to resolve by name.
    #[test]
    fn from_defs_for_test_seeds_by_name() {
        let def = def_from_json(r#"{ "name": "greet", "role": "user", "body": "hi" }"#);
        let reg = Registry::from_defs_for_test([def]);
        assert!(reg.inner().get("greet").is_some());
    }
}
