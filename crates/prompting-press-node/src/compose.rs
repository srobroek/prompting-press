//! Composition types and the `Message` napi object (spec 004, US4; T022; FR-012/FR-013).
//!
//! Post-spec-008 reshape:
//! - The TS facade owns its own `Composition` class that stores `NapiPrompt` handles and
//!   calls `NapiPrompt::render_prompt` directly — no registry needed, no `#[napi] Composition`.
//! - The napi `Composition` class and its `append`/`resolve`/`fromMessages` methods are
//!   **demoted to plain Rust** (no `#[napi]`) so they no longer appear on the JS surface
//!   (SC-001 / T046).
//! - [`Message`] is kept as a `#[napi(object)]` because the TS facade imports it as a type
//!   from the addon's generated `index.d.ts`.
//! - [`MessageEntry`] is demoted to plain Rust (the TS facade no longer uses it as a napi type;
//!   the facade's own `CompositionEntry` interface takes a `Prompt` object, not a name string).
//! - The `#[cfg(test)]` suites below are kept in Rust so `cargo test -p prompting-press-node`
//!   exercises the kernel-direct resolve path without a Node runtime.

use napi_derive::napi;

use prompting_press::ConsumerError;
use prompting_press_core::GuardConfig as KernelGuardConfig;

use crate::error::{consumer_error_to_napi_err, kernel_error_to_napi_err};
use crate::marshal::to_kernel_value;
use crate::registry::Registry;

/// One resolved message in a composition's output: a role-tagged rendered string.
///
/// The Node mirror of the consumer's `Message` (data-model §Message). `role` is the prompt
/// definition's role stringified (`"system"` / `"user"` / `"assistant"`); `text` is that prompt
/// rendered with the entry's own validated value. A `#[napi(object)]` so it crosses as a plain JS
/// object `{ role, text }`; a message is produced by [`Composition::resolve`], never constructed
/// from JS.
#[derive(Clone, Debug)]
#[napi(object)]
pub struct Message {
    /// The conversational role, taken from the prompt definition's `role`.
    pub role: String,
    /// The rendered body text for this entry. The guard text is never concatenated here.
    pub text: String,
}

/// One input entry for the Rust `Composition::from_messages` test helper.
///
/// Plain Rust — no longer a `#[napi(object)]` (SC-001 / T046). The TS facade's `CompositionEntry`
/// interface takes a `Prompt` object directly; this struct survives only for `#[cfg(test)]` use.
pub struct MessageEntry {
    /// The prompt's registry name (resolved at `resolve`, not at construction).
    pub name: String,
    /// The already-validated value for this entry (validated in the TS facade — Q1).
    pub value: serde_json::Value,
    /// The selected variant (absent ⇒ the reserved `default` / root body).
    pub variant: Option<String>,
}

/// One appended entry, captured after marshaling.
///
/// The value is already marshaled into a [`minijinja::Value`] (the same type the kernel renders
/// against), so the `Vec` of entries is homogeneous despite each entry's source value differing.
/// The entry holds only the data the kernel needs at render time.
struct Entry {
    /// The prompt's registry name (resolved at `resolve`, not at `append`).
    name: String,
    /// The pre-marshaled value (the FFI bridge value — FR-003a), ready for the kernel.
    values: minijinja::Value,
    /// The selected variant (`None` ⇒ the reserved `default` / root body).
    variant: Option<String>,
}

/// An explicit, ordered sequence of `(prompt-name, value, variant)` entries.
///
/// Plain Rust — no longer a `#[napi]` class (SC-001 / T046). The TS facade owns its own
/// `Composition` class. This struct survives only for `#[cfg(test)]` use.
pub struct Composition {
    /// Entries in append order — the resolved-message order (FR-012).
    entries: Vec<Entry>,
}

impl Composition {
    /// Create an empty composition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Build a composition from an ordered array of `(name, value, variant?)` entries.
    #[must_use]
    pub fn from_messages(entries: Vec<MessageEntry>) -> Self {
        let mut composition = Self::new();
        for entry in entries {
            composition.append_entry(&entry.name, entry.value, entry.variant);
        }
        composition
    }

    /// Marshal + store one entry.
    pub fn append(&mut self, name: String, value: serde_json::Value, variant: Option<String>) {
        self.append_entry(&name, value, variant);
    }

    /// The number of appended entries.
    #[must_use]
    pub fn length(&self) -> u32 {
        u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
    }

    /// Resolve the composition to an ordered `Message[]` by rendering each entry through the kernel.
    ///
    /// Used by `#[cfg(test)]` suites; the public JS path is the TS facade's own `Composition`
    /// class (which calls `NapiPrompt::render_prompt` directly, no registry needed).
    ///
    /// # Errors
    /// - `load` — an entry's name is absent from `reg`.
    /// - kernel codes — the kernel rejected an entry's render.
    pub fn resolve(&self, reg: &Registry) -> napi::Result<Vec<Message>> {
        let mut messages = Vec::with_capacity(self.entries.len());

        for entry in &self.entries {
            // Resolve the prompt by name (absent ⇒ structured error, never a panic).
            let Some(def) = reg.get(&entry.name) else {
                return Err(consumer_error_to_napi_err(ConsumerError::Load(format!(
                    "unknown prompt: `{}`",
                    entry.name
                ))));
            };

            // Render by calling the KERNEL DIRECTLY (critique E1 / C-01) with the already-validated
            // value. Composition does no guard expansion; a default GuardConfig leaves `text`
            // unchanged. A kernel failure propagates here, discarding the partial `messages` built
            // so far — no partial-as-success.
            let result = prompting_press_core::render(
                def,
                entry.variant.as_deref(),
                entry.values.clone(),
                &KernelGuardConfig::default(),
            )
            .map_err(kernel_error_to_napi_err)?;

            messages.push(Message {
                // The prompt definition's role, stringified via its `Display` impl.
                role: def.role.to_string(),
                text: result.text,
            });
        }

        Ok(messages)
    }
}

impl Default for Composition {
    /// An empty composition — the same value [`Composition::new`] (the JS constructor) produces.
    /// Present so the `pub` napi constructor satisfies `clippy::new_without_default`; the JS surface
    /// always uses `new Composition()`.
    fn default() -> Self {
        Self::new()
    }
}

impl Composition {
    /// The shared marshal + store step behind both [`append`](Self::append) and
    /// [`from_messages`](Self::from_messages).
    ///
    /// Marshals the (already-validated, in the TS facade) `value` to the kernel's value type and
    /// pushes the entry. Marshaling is infallible (the value already crossed napi), so this cannot
    /// leave a half-built entry.
    fn append_entry(&mut self, name: &str, value: serde_json::Value, variant: Option<String>) {
        let values = to_kernel_value(value);
        self.entries.push(Entry {
            name: name.to_string(),
            values,
            variant,
        });
    }
}

#[cfg(test)]
mod tests {
    //! Composition coverage that is drivable in Rust WITHOUT the TS facade.
    //!
    //! The validate-at-facade behavior (a real Zod schema, the error subclass on invalid vars,
    //! "nothing resolved on failure") lives TS-side in T021. Here we exercise the parts that need
    //! no JS runtime: an empty composition resolves to `[]`; `append` then `resolve` renders in
    //! order with roles; an unknown-name entry surfaces as a `load`-coded error at `resolve`;
    //! and one entry's failure discards the partial result.

    use super::*;
    use prompting_press::error::code;
    use prompting_press::PromptDefinition;

    fn def_from_json(json: &str) -> PromptDefinition {
        serde_json::from_str(json).expect("valid prompt definition")
    }

    fn payload_of(err: &napi::Error) -> serde_json::Value {
        serde_json::from_str(&err.reason).expect("napi error reason is the JSON payload")
    }

    /// An empty composition resolves to an empty list (the `[]` edge case), with no registry
    /// lookups performed.
    #[test]
    fn empty_composition_resolves_to_empty() {
        let comp = Composition::new();
        assert_eq!(comp.length(), 0, "a fresh composition has no entries");

        let reg = Registry::from_defs_for_test([]);
        let messages = comp.resolve(&reg).expect("empty resolve");
        assert!(messages.is_empty(), "empty composition ⇒ empty list");
    }

    /// `append` + `resolve` renders each entry in append order through the kernel and tags it with
    /// the prompt's role. Drives the kernel-direct resolve loop + the `role` stringification + the
    /// message ORDER without the TS facade. (Full validate-at-facade behavior is covered TS-side in
    /// T021.)
    #[test]
    fn append_then_resolve_renders_in_order_with_roles() {
        let system = def_from_json(
            r#"{ "name": "sys", "role": "system", "body": "You are {{ persona }}." }"#,
        );
        let user =
            def_from_json(r#"{ "name": "ask", "role": "user", "body": "Question: {{ q }}" }"#);
        let reg = Registry::from_defs_for_test([system, user]);

        let mut comp = Composition::new();
        comp.append(
            "sys".to_string(),
            serde_json::json!({ "persona": "a helpful assistant" }),
            None,
        );
        comp.append("ask".to_string(), serde_json::json!({ "q": "why?" }), None);
        assert_eq!(comp.length(), 2, "two appended entries");

        let messages = comp.resolve(&reg).expect("resolve succeeds");
        assert_eq!(messages.len(), 2, "one message per entry");

        // Append order is preserved (FR-012).
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[0].text, "You are a helpful assistant.");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].text, "Question: why?");
    }

    /// `fromMessages` builds the same ordered composition from an entry array (the bulk
    /// constructor), resolving identically.
    #[test]
    fn from_messages_builds_ordered_composition() {
        let user =
            def_from_json(r#"{ "name": "ask", "role": "user", "body": "Question: {{ q }}" }"#);
        let reg = Registry::from_defs_for_test([user]);

        let comp = Composition::from_messages(vec![
            MessageEntry {
                name: "ask".to_string(),
                value: serde_json::json!({ "q": "first?" }),
                variant: None,
            },
            MessageEntry {
                name: "ask".to_string(),
                value: serde_json::json!({ "q": "second?" }),
                variant: None,
            },
        ]);
        assert_eq!(comp.length(), 2);

        let messages = comp.resolve(&reg).expect("resolve succeeds");
        assert_eq!(messages[0].text, "Question: first?");
        assert_eq!(messages[1].text, "Question: second?");
    }

    /// A later entry's render failure propagates as the mapped error and DISCARDS the partial
    /// result — `resolve` returns `Err`, never a truncated `Message[]` (no partial-as-success). The
    /// second entry references a prompt whose body needs a root the value lacks, driving the
    /// kernel's strict-undefined path; the first entry alone would have succeeded.
    #[test]
    fn one_entry_failure_discards_partial_result() {
        let ok = def_from_json(r#"{ "name": "ok", "role": "user", "body": "fine" }"#);
        let needs =
            def_from_json(r#"{ "name": "needs", "role": "user", "body": "Hello {{ missing }}!" }"#);
        let reg = Registry::from_defs_for_test([ok, needs]);

        let mut comp = Composition::new();
        comp.append("ok".to_string(), serde_json::json!({}), None);
        // No `missing` in the value ⇒ strict-undefined kernel error on the second entry.
        comp.append("needs".to_string(), serde_json::json!({}), None);

        let err = comp
            .resolve(&reg)
            .expect_err("the second entry's strict-undefined render must fail the whole resolve");
        let payload = payload_of(&err);
        assert_eq!(
            payload["code"],
            code::UNDEFINED_VARIABLE,
            "a strict-undefined render is loud, and the partial first-entry result is discarded"
        );
    }
}
