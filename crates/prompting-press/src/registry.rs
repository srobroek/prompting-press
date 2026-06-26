//! The prompt [`Registry`] — a library-owned map of prompt name → loaded
//! [`PromptDefinition`] (FR-008a, clarify Q2).
//!
//! Backed by a [`BTreeMap`](std::collections::BTreeMap) so iteration order is
//! **deterministic** — `check()` (a later
//! phase) walks the registry and must produce stable, reproducible findings ordering for a
//! CI gate.
//!
//! Population is by three equal-footing paths (FR-005): the dual-input loaders
//! [`load_yaml`](Registry::load_yaml) / [`load_json`](Registry::load_json), which deserialize
//! already-read text into the kernel's [`PromptDefinition`], and [`insert`](Registry::insert)
//! for a constructed object. All three normalize into the **same** `PromptDefinition` — there
//! is no parallel shape (FR-008). The crate does no I/O — the caller hands in already-read
//! text or a constructed object (C-03 / FR-024).

use std::collections::BTreeMap;

use prompting_press_core::PromptDefinition;

use crate::ConsumerError;

/// A name → [`PromptDefinition`] map. The single in-memory home for loaded prompts;
/// `render` / `get_source` / `check` resolve a prompt by name against it (absent ⇒
/// [`crate::ConsumerError::UnknownPrompt`], wired in a later phase).
#[derive(Debug, Clone, Default)]
pub struct Registry {
    /// BTreeMap keyed by [`PromptDefinition::name`] → deterministic iteration for `check()`.
    prompts: BTreeMap<String, PromptDefinition>,
}

impl Registry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a constructed [`PromptDefinition`], keyed by its `name`.
    ///
    /// The key is the prompt's [`name`](PromptDefinition::name) (a `#[serde(transparent)]`
    /// newtype that derefs to `String`). An existing entry with the same name is replaced.
    pub fn insert(&mut self, def: PromptDefinition) {
        let key = def.name.to_string();
        self.prompts.insert(key, def);
    }

    /// Look up a prompt by name. Returns `None` when absent — callers that need a hard error
    /// map the absence to [`crate::ConsumerError::UnknownPrompt`] (FR-008a).
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PromptDefinition> {
        self.prompts.get(name)
    }

    /// Iterate over `(name, definition)` pairs in **deterministic** (sorted-by-name) order.
    ///
    /// The registry is backed by a [`BTreeMap`], so iteration is sorted by prompt name. This
    /// is the accessor [`check`](crate::check::check) walks to lint the whole collection
    /// (FR-016): a stable iteration order is what makes the [`CheckReport`](crate::CheckReport)
    /// findings order reproducible for a CI gate. It hands out only shared borrows, so a
    /// caller cannot mutate the registry through it (supporting `check()`'s purity — FR-019).
    pub fn iter(&self) -> impl Iterator<Item = (&str, &PromptDefinition)> {
        self.prompts.iter().map(|(name, def)| (name.as_str(), def))
    }

    /// Load a prompt definition from an already-read **JSON** document (FR-005), insert it
    /// keyed by `name`, and return a reference to the inserted definition.
    ///
    /// JSON is deserialized into the kernel's [`PromptDefinition`] — the single internal
    /// representation; this defines no parallel shape (FR-008). A document loaded from JSON
    /// and the equivalent loaded from YAML produce structurally identical definitions
    /// (FR-006).
    ///
    /// On a deserialize failure (malformed JSON, or data that violates the
    /// `PromptDefinition` shape — the generated struct uses `#[serde(deny_unknown_fields)]`
    /// and rejects missing required fields), this returns [`ConsumerError::Load`] and
    /// **inserts nothing** — the registry is left untouched (FR-007: no partial/coerced load).
    ///
    /// The `Load` message carries the serde error string. That is a *parse-location*
    /// description (line/column / "missing field `body`"), not bound-value content — unlike
    /// the kernel's `Parse`/`Render` detail (which is scrubbed, SEC-004), it is safe to
    /// surface here.
    ///
    /// The crate reads no files; the caller hands in already-read text (C-03 / FR-024).
    ///
    /// # Errors
    ///
    /// Returns [`ConsumerError::Load`] if `doc` is not valid JSON or does not match the
    /// `PromptDefinition` shape.
    pub fn load_json(&mut self, doc: &str) -> Result<&PromptDefinition, ConsumerError> {
        let def: PromptDefinition =
            serde_json::from_str(doc).map_err(|e| ConsumerError::Load(e.to_string()))?;
        Ok(self.insert_and_get(def))
    }

    /// Load a prompt definition from an already-read **YAML** document (FR-005), insert it
    /// keyed by `name`, and return a reference to the inserted definition.
    ///
    /// YAML is deserialized via `serde_yaml_ng` (the maintained `serde_yaml` successor,
    /// backed by the pure-Rust YAML-1.2 parser `yaml-rust2` — research D2) into the **same**
    /// [`PromptDefinition`] as [`load_json`](Registry::load_json) (FR-006/008). Because the
    /// backing parser is YAML 1.2, the "Norway problem" does not apply: bare `no` / `yes` /
    /// `off` are plain-scalar **strings**, not booleans.
    ///
    /// Error and no-partial-load semantics are identical to [`load_json`](Registry::load_json):
    /// a deserialize failure returns [`ConsumerError::Load`] and inserts nothing (FR-007).
    ///
    /// The crate reads no files; the caller hands in already-read text (C-03 / FR-024).
    ///
    /// # Errors
    ///
    /// Returns [`ConsumerError::Load`] if `doc` is not valid YAML or does not match the
    /// `PromptDefinition` shape.
    pub fn load_yaml(&mut self, doc: &str) -> Result<&PromptDefinition, ConsumerError> {
        let def: PromptDefinition =
            serde_yaml_ng::from_str(doc).map_err(|e| ConsumerError::Load(e.to_string()))?;
        Ok(self.insert_and_get(def))
    }

    /// Insert `def` (keyed by its `name`) and return a reference to the just-inserted entry.
    ///
    /// Shared tail of the loaders. The re-`get` after `insert` sidesteps the borrow-checker
    /// friction of returning a reference produced by a `&mut self` insert: we capture the
    /// owned key, insert, then borrow it back immutably. The key is guaranteed present.
    fn insert_and_get(&mut self, def: PromptDefinition) -> &PromptDefinition {
        let key = def.name.to_string();
        self.prompts.insert(key.clone(), def);
        self.prompts
            .get(&key)
            .expect("just inserted under this key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(name: &str) -> PromptDefinition {
        serde_json::from_str(&format!(
            r#"{{ "name": "{name}", "role": "user", "body": "Hello {{{{ x }}}}" }}"#
        ))
        .expect("valid prompt definition")
    }

    #[test]
    fn insert_then_get_round_trips_by_name() {
        let mut reg = Registry::new();
        reg.insert(def("greet"));

        let got = reg.get("greet").expect("present after insert");
        assert_eq!(got.name.to_string(), "greet");
        assert!(reg.get("absent").is_none());
    }

    #[test]
    fn insert_replaces_same_name() {
        let mut reg = Registry::new();
        reg.insert(def("greet"));
        reg.insert(def("greet"));
        // Still exactly one logical entry under that name.
        assert!(reg.get("greet").is_some());
    }

    #[test]
    fn empty_registry_resolves_nothing() {
        let reg = Registry::new();
        assert!(reg.get("anything").is_none());
    }
}
