//! Multi-message composition for the Node binding (spec 004, US4; T022; FR-012/FR-013).
//!
//! A [`Composition`] is an **explicit, ordered** sequence of `(prompt-name, value, variant)`
//! entries that [`resolve`](Composition::resolve)s — in append order — to a `Message[]`, where each
//! [`Message`] is the named prompt rendered with its own (already-validated) value and tagged with
//! that prompt definition's `role` (FR-012). It is the few-shot / system+user sequence builder.
//! Construction is `new Composition()` + [`append`](Composition::append), or the
//! [`from_messages`](Composition::from_messages) (`fromMessages`) bulk constructor — there is
//! deliberately **no** fluent `.chain()` API (FR-013; it cannot cross the napi boundary and
//! collides with `Iterator::chain`).
//!
//! ## Why a binding-OWNED `Composition` (critique E1 / C-01)
//!
//! The Rust consumer's [`prompting_press::Composition`] is generic over `V: Serialize + Validate`
//! — a **garde** type. This binding has no such type: validation is owned in **TypeScript** (the
//! facade's `safeParse` — Q1), so there is no `V` to instantiate the consumer's `Composition` with.
//! Therefore this module owns its **own** `Composition` `#[napi]` class holding already-marshaled
//! entries, and [`resolve`](Composition::resolve) calls the **kernel directly** per entry —
//! exactly mirroring how the binding's [`render`](crate::render::render) works (US1). This is still
//! **zero engine logic** (Principle I): the kernel renders; the binding only marshals (the value
//! the TS facade already validated) and surfaces results. Render byte-parity with the Rust/Python
//! bindings stays structural because each entry's value is built by the same
//! [`to_kernel_value`](crate::marshal::to_kernel_value) path single-render uses.
//!
//! ## Eager marshaling at `append` — no partial state
//!
//! Each entry's value is **already validated** in the TS facade before the addon's `append` is
//! called (mirroring US1's `safeParse`-at-boundary). `append` marshals that validated value to the
//! kernel's [`minijinja::Value`] and stores the entry. The prompt `name` is **not** resolved at
//! `append` — an unknown name surfaces at [`resolve`](Composition::resolve). Because marshaling is
//! infallible (the value already crossed napi), an `append` cannot leave a half-built entry.
//!
//! ## resolve: prompt resolution + render, in order
//!
//! [`resolve`](Composition::resolve) walks the stored entries in append order. For each it resolves
//! the prompt by name against the [`Registry`](crate::registry::Registry) (absent ⇒ an
//! `unknown_prompt` error, never a panic) and delegates rendering to the kernel via
//! [`prompting_press_core::render`] with the entry's pre-marshaled value. Each result becomes
//! `Message { role: <def.role stringified>, text: result.text }`. One entry's failure (unknown
//! prompt, unknown variant, a strict-undefined reference) propagates as the mapped napi error and
//! the partial result built so far is **discarded** — never returned as success. An empty
//! composition resolves to `[]`.

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

/// One input entry for [`Composition::from_messages`]: an explicit `(name, value, variant?)`.
///
/// A `#[napi(object)]` so the TS facade passes a plain array of `{ name, value, variant? }`
/// objects (the idiomatic JS shape for an ordered `(prompt-ref, vars)` array — FR-012). `value` is
/// the already-Zod-validated payload for that entry; `variant` is optional (absent ⇒ the reserved
/// `default` arm).
#[napi(object)]
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

/// An explicit, ordered sequence of `(prompt-name, value, variant)` entries that resolves to a
/// `Message[]` in append order (FR-012). Built with `new Composition()` +
/// [`append`](Self::append) or [`from_messages`](Self::from_messages); there is **no** fluent
/// `.chain()` (FR-013).
#[napi]
pub struct Composition {
    /// Entries in append order — the resolved-message order (FR-012).
    entries: Vec<Entry>,
}

#[napi]
impl Composition {
    /// `new Composition()` — create an empty composition. An empty composition resolves to `[]`.
    #[napi(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// `Composition.fromMessages(entries)` — build a composition from an ordered array of
    /// `(name, value, variant?)` entries, marshaling each in order.
    ///
    /// `entries` is an array of [`MessageEntry`] objects (`{ name, value, variant? }`). Each
    /// `value` is the already-Zod-validated payload (validation runs in the TS facade — Q1); this
    /// marshals + stores each in append order. Because marshaling is infallible, the whole
    /// construction succeeds and returns the populated `Composition`.
    #[napi(factory)]
    #[must_use]
    pub fn from_messages(entries: Vec<MessageEntry>) -> Self {
        let mut composition = Self::new();
        for entry in entries {
            composition.append_entry(&entry.name, entry.value, entry.variant);
        }
        composition
    }

    /// `composition.append(name, value, variant?)` — marshal + store one entry.
    ///
    /// `value` is the already-Zod-validated payload (validation runs in the TS facade — Q1); it is
    /// marshaled to the kernel's value type and the entry is stored. The prompt `name` is **not**
    /// resolved here — an unknown name surfaces at [`resolve`](Self::resolve) as an
    /// `unknown_prompt` error.
    ///
    /// Returns `void` (not `this`): the builder is intentionally **not** fluent/chainable (FR-013).
    #[napi]
    pub fn append(&mut self, name: String, value: serde_json::Value, variant: Option<String>) {
        self.append_entry(&name, value, variant);
    }

    /// `composition.length` — the number of appended entries (== the resolved-message count on
    /// success). Surfaces as a `length` getter on the JS class.
    #[napi(getter)]
    #[must_use]
    pub fn length(&self) -> u32 {
        // entry counts are tiny; a saturating cast keeps the JS-side `number` honest.
        u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
    }

    /// `composition.resolve(registry)` — resolve the composition to an ordered `Message[]`
    /// (FR-012), rendering each entry — in append order — through the kernel.
    ///
    /// For each entry, in order: resolve the prompt by name against `reg` (absent ⇒ an
    /// `unknown_prompt` error, never a panic), then delegate rendering to
    /// [`prompting_press_core::render`] **directly** (critique E1 / C-01) with the entry's
    /// **pre-marshaled** value. The render result becomes
    /// `Message { role: <def.role stringified>, text: result.text }`. Composition uses no guard
    /// expansion — a default [`GuardConfig`](prompting_press_core::GuardConfig) is passed, which
    /// leaves `text` unchanged.
    ///
    /// One entry's render failure (unknown prompt, unknown variant, a strict-undefined reference, a
    /// parse/render error) propagates as the mapped napi error and the partial result built so far
    /// is **discarded** — never returned as success. An empty composition resolves to `[]`.
    ///
    /// # Errors
    /// - `unknown_prompt` — an entry's name is absent from `reg`.
    /// - a kernel code (`unknown_variant` / `undefined_variable` / `parse` / `render` /
    ///   `excluded_feature`) — the kernel rejected an entry's render. `parse`/`render` detail is
    ///   scrubbed (SEC-004).
    #[napi]
    pub fn resolve(&self, reg: &Registry) -> napi::Result<Vec<Message>> {
        let mut messages = Vec::with_capacity(self.entries.len());

        for entry in &self.entries {
            // Resolve the prompt by name (absent ⇒ structured error, never a panic).
            let Some(def) = reg.inner().get(&entry.name) else {
                return Err(consumer_error_to_napi_err(ConsumerError::UnknownPrompt(
                    entry.name.clone(),
                )));
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
    //! order with roles; an unknown-name entry surfaces as an `unknown_prompt` error at `resolve`;
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

    /// An entry naming a prompt absent from the registry surfaces as an `unknown_prompt`-coded
    /// error at `resolve` (FR-008a) — a loud, structured error, never a panic across napi.
    #[test]
    fn unknown_name_entry_is_loud_at_resolve() {
        let present = def_from_json(r#"{ "name": "present", "role": "user", "body": "hi" }"#);
        let reg = Registry::from_defs_for_test([present]);

        let mut comp = Composition::new();
        comp.append("absent".to_string(), serde_json::json!({}), None);

        let err = comp
            .resolve(&reg)
            .expect_err("an unknown-name entry must error at resolve");
        let payload = payload_of(&err);
        assert_eq!(
            payload["code"],
            code::UNKNOWN_PROMPT,
            "an absent prompt name maps to unknown_prompt"
        );
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
