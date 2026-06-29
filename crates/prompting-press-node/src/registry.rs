//! A plain Rust `BTreeMap`-backed store of loaded prompt definitions — the internal
//! definition cache used by the `#[cfg(test)]` suites in render/check/compose.
//!
//! Post-spec-008 reshape:
//! - The consumer's `Registry` type was removed; the consumer works with `Prompt` objects.
//! - The **public JS surface** no longer exposes any registry concept: `Registry`, the
//!   registry-keyed `render(reg, name, …)`, `check(reg)`, and `getSource(reg, …)` napi
//!   functions are all gone from the addon (SC-001 / T046).
//! - This struct is therefore a **plain Rust type with no `#[napi]` annotations**. It is
//!   only instantiated by the sibling modules' `#[cfg(test)]` helpers via
//!   [`Registry::from_defs_for_test`], so the test bodies still compile without invasive
//!   rewrites.
//!
//! No code outside `#[cfg(test)]` blocks uses this type in the napi crate.

use std::collections::BTreeMap;

use prompting_press::{ConsumerError, PromptDefinition};

use crate::error::consumer_error_to_napi_err;

/// Internal definition cache — plain Rust, no `#[napi]`.
///
/// Holds a `BTreeMap<String, PromptDefinition>` populated via the consumer's validated
/// `Prompt::from_yaml` / `Prompt::from_json` loaders. Exists only to support the
/// `#[cfg(test)]` suites in `render`, `check`, and `compose` that were written against
/// a registry-shaped seeder; they are preserved in Rust so `cargo test` still exercises
/// the kernel-direct render/check/compose paths without a Node runtime.
pub struct Registry {
    defs: BTreeMap<String, PromptDefinition>,
}

impl Registry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            defs: BTreeMap::new(),
        }
    }

    /// Load a prompt from an already-read **YAML** document, keyed by name.
    ///
    /// Delegates to [`prompting_press::Prompt::from_yaml`]. A malformed document throws a
    /// `load`-coded napi error and inserts **nothing** (FR-007).
    pub fn load_yaml(&mut self, text: String) -> napi::Result<()> {
        let prompt =
            prompting_press::Prompt::from_yaml(&text).map_err(consumer_error_to_napi_err)?;
        let name = prompt.name().to_owned();
        self.defs.insert(name, prompt.definition().clone());
        Ok(())
    }

    /// Load a prompt from an already-read **JSON** document, keyed by name.
    pub fn load_json(&mut self, text: String) -> napi::Result<()> {
        let prompt =
            prompting_press::Prompt::from_json(&text).map_err(consumer_error_to_napi_err)?;
        let name = prompt.name().to_owned();
        self.defs.insert(name, prompt.definition().clone());
        Ok(())
    }

    /// Insert a prompt-definition object, keyed by name.
    pub fn insert(&mut self, definition: serde_json::Value) -> napi::Result<()> {
        let json = serde_json::to_string(&definition)
            .map_err(|e| consumer_error_to_napi_err(ConsumerError::Load(e.to_string())))?;
        let prompt =
            prompting_press::Prompt::from_json(&json).map_err(consumer_error_to_napi_err)?;
        let name = prompt.name().to_owned();
        self.defs.insert(name, prompt.definition().clone());
        Ok(())
    }

    /// Borrow a definition by name.
    pub(crate) fn get(&self, name: &str) -> Option<&PromptDefinition> {
        self.defs.get(name)
    }

    /// Iterate over all definitions in deterministic (BTreeMap) order.
    pub(crate) fn definitions(&self) -> impl Iterator<Item = (&str, &PromptDefinition)> {
        self.defs.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// Build a `Registry` pre-populated with already-constructed [`PromptDefinition`]s.
    ///
    /// Test-only helper: bypasses napi decoding so a Rust test can seed definitions directly.
    #[cfg(test)]
    pub(crate) fn from_defs_for_test(defs: impl IntoIterator<Item = PromptDefinition>) -> Self {
        let mut reg = Self::new();
        for def in defs {
            let name = def.name.to_string();
            reg.defs.insert(name, def);
        }
        reg
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    #[test]
    fn insert_then_get_round_trips() {
        let mut reg = Registry::new();
        reg.insert(serde_json::json!({
            "name": "greet",
            "role": "user",
            "body": "Hi {{ name }}",
            "variables": { "name": { "type": "string", "origin": "trusted" } },
        }))
        .expect("valid shape");

        assert!(reg.get("greet").is_some(), "present after insert");
        assert!(reg.get("absent").is_none());
    }

    #[test]
    fn insert_malformed_object_errors() {
        let mut reg = Registry::new();
        let res = reg.insert(serde_json::json!({ "name": "greet" }));
        assert!(res.is_err(), "missing required fields must error, not panic");
        assert!(reg.get("greet").is_none(), "no partial load");
    }

    #[test]
    fn load_json_inserts_and_resolves() {
        let mut reg = Registry::new();
        reg.load_json(
            r#"{"name":"greet","role":"user","body":"Hi {{ name }}","variables":{"name":{"type":"string","origin":"trusted"}}}"#
                .to_string(),
        )
        .expect("valid json document");
        assert!(reg.get("greet").is_some());
    }

    #[test]
    fn load_yaml_inserts_and_resolves() {
        let mut reg = Registry::new();
        reg.load_yaml(
            "name: greet\nrole: user\nbody: \"Hi {{ name }}\"\nvariables:\n  name:\n    type: string\n    origin: trusted\n"
                .to_string(),
        )
        .expect("valid yaml document");
        assert!(reg.get("greet").is_some());
    }

    #[test]
    fn load_json_malformed_errors_and_inserts_nothing() {
        let mut reg = Registry::new();
        let res = reg.load_json(r#"{"name":"greet","role":"user"}"#.to_string());
        assert!(res.is_err(), "missing required field must error");
        assert!(reg.get("greet").is_none(), "no partial load (FR-007)");
    }

    #[test]
    fn from_defs_for_test_seeds_by_name() {
        let def =
            def_from_json(r#"{"name":"greet","role":"user","body":"hi","variables":{}}"#);
        let reg = Registry::from_defs_for_test([def]);
        assert!(reg.get("greet").is_some());
    }
}
