//! Multi-message composition (spec 008, T031; FR-012/FR-013).
//!
//! A [`Composition`] is an **explicit, ordered** sequence of `(Prompt, vars, variant)`
//! entries that [`resolve`](Composition::resolve)s — in append order — to a `Vec<Message>`,
//! where each [`Message`] is that `Prompt` rendered with its own validated vars and tagged
//! with that prompt definition's role (FR-012). It is the few-shot / system+user sequence
//! builder (US4).
//!
//! ## Prompt-as-object, no Registry (spec 008 reshape)
//!
//! Pre-reshape, `Composition` aggregated `(name, vars, variant)` entries and resolved
//! names against a `Registry` passed to `resolve`. Post-reshape, each entry holds an owned
//! (or borrowed-by-clone) `Prompt` — construction invariants are already enforced on the
//! `Prompt`, so resolution never hits an "unknown prompt" path (the definition is right
//! there). No `Registry` is required.
//!
//! ## No `.chain()` (FR-013)
//!
//! Construction is `new()` + [`append`](Composition::append) only — a plain ordered builder.
//! There is deliberately **no** fluent `.chain()` API: it would collide with
//! [`Iterator::chain`] and cannot cross the PyO3 / napi FFI boundary the later bindings need
//! (constitution Principle VI). `append` takes `&mut self` and returns `Result<(), …>`, not
//! `Self`, so it is not chainable.
//!
//! ## Where validation happens — eager, at `append` (decision: option (a))
//!
//! The typing challenge is that each entry carries a *different* typed Vars type
//! (`V: Serialize + Validate`), yet all entries live in one homogeneous `Vec`. Two shapes
//! were possible:
//!
//! - **(a)** validate + serialize **eagerly at `append`**, storing the resulting
//!   type-erased [`minijinja::Value`] + the `Prompt` + variant; [`resolve`](Composition::resolve)
//!   then only calls `prompt.render` with the already-validated value.
//! - **(b)** store boxed trait objects / closures that defer validation to `resolve`.
//!
//! This module takes **(a)**. It needs no `dyn` machinery (the only erased thing is the
//! already-validated `minijinja::Value`, which the kernel takes anyway), and it keeps the
//! library's central guarantee intact: **validation always runs, and a partial result is
//! never returned as success**. Under (a) an invalid entry fails *at `append`* — the entry is
//! rejected and never enters the `Vec`, so `resolve` can only ever see fully-validated
//! entries.
//!
//! `append` is therefore **fallible**: a garde failure surfaces immediately as a normalized
//! [`ConsumerError::Validation`] naming the offending field (FR-014), and the composition is
//! left exactly as it was before the call (the bad entry is not stored).
//!
//! ## resolve: render in order, no Registry
//!
//! [`resolve`](Composition::resolve) walks the stored entries in append order. For each it
//! calls [`prompting_press_core::render`] directly on the entry's `Prompt`'s definition with
//! the pre-validated value — reusing the same kernel call path (no rendering logic is
//! duplicated here — FR-011 / C-01). Each result maps to
//! `Message { role: <def.role stringified>, text: result.text }`. One entry's failure
//! (unknown variant, a strict-undefined reference, a parse/render error) propagates as the
//! normalized [`ConsumerError`]; the partial result is **not** returned as success (US4
//! scenario 3). An empty composition resolves to `Ok(vec![])` (edge case F7).

use garde::Validate;
use minijinja::Value;
use prompting_press_core::GuardConfig;
use serde::Serialize;

use crate::prompt::Prompt;
use crate::ConsumerError;

/// One resolved message in a composition's output: a role-tagged rendered string (data-model
/// §Message). `role` is the prompt definition's role stringified (`"system"` / `"user"` /
/// `"assistant"`); `text` is that prompt rendered with the entry's own validated vars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The conversational role, taken from the prompt definition's `role`.
    pub role: String,
    /// The rendered body text for this entry.
    pub text: String,
}

/// One appended entry, captured after eager validation + serialization (option (a)).
///
/// The vars are already type-erased into a [`minijinja::Value`] (the same type the kernel
/// renders against), so the `Vec` of entries is homogeneous despite each entry's source Vars
/// type differing. The `Prompt` is cloned in at `append` and owns its definition.
#[derive(Debug, Clone)]
struct Entry {
    /// The validated, fully-constructed `Prompt` (owns the definition).
    prompt: Prompt,
    /// The pre-validated, serialized vars (the bridge value — FR-003a), ready for the kernel.
    values: Value,
    /// The selected variant (`None` ⇒ the reserved `default` / root body).
    variant: Option<String>,
}

/// An explicit, ordered sequence of `(Prompt, vars, variant)` entries that resolves to a
/// `Vec<Message>` in append order (FR-012). Built with [`new`](Self::new) +
/// [`append`](Self::append); there is no fluent `.chain()` (FR-013). No `Registry` needed.
#[derive(Debug, Clone, Default)]
pub struct Composition {
    /// Entries in append order — the resolved-message order (FR-012).
    entries: Vec<Entry>,
}

impl Composition {
    /// Create an empty composition. An empty composition resolves to `Ok(vec![])` (F7).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// `true` iff no entries have been appended.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The number of appended entries (== the resolved-message count on success).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Append one `(prompt, vars, variant)` entry, **validating + serializing `vars` eagerly**
    /// (option (a) — see module docs).
    ///
    /// `vars` is validated **once** via garde *now*; on success it is serialized to the
    /// kernel's value type ([`minijinja::Value::from_serialize`], FR-003a) and the entry is
    /// stored (alongside a clone of `prompt`). On failure the garde report is normalized to
    /// [`ConsumerError::Validation`] (FR-014) and **nothing is stored** — the composition is
    /// unchanged, so a later [`resolve`](Self::resolve) never sees a half-validated entry.
    ///
    /// Takes `&mut self` and returns `Result<(), ConsumerError>` (not `Self`): the builder is
    /// intentionally **not** fluent/chainable (FR-013).
    ///
    /// `V::Context: Default` so the whole-struct [`Validate::validate`] convenience applies
    /// (one validation pass over the entry's entire input set). Context-carrying validation
    /// is intentionally out of v1 scope (scope discipline — TY-4 / one concrete path per
    /// concern).
    ///
    /// # Errors
    ///
    /// [`ConsumerError::Validation`] — garde rejected `vars`. The entry is not appended.
    pub fn append<V>(
        &mut self,
        prompt: &Prompt,
        vars: &V,
        variant: Option<&str>,
    ) -> Result<(), ConsumerError>
    where
        V: Serialize + Validate,
        V::Context: Default,
    {
        // Validate the entry's whole input set ONCE, eagerly (FR-002 semantics at append).
        // On failure the entry is NOT stored — the composition is left untouched.
        vars.validate().map_err(ConsumerError::from)?;

        // Bridge the now-validated struct to the kernel's value type (FR-003a).
        // `from_serialize` is infallible (ER-2): a custom-Serialize failure surfaces
        // downstream as a strict-undefined kernel error, never silently here.
        let values = Value::from_serialize(vars);

        self.entries.push(Entry {
            prompt: prompt.clone(),
            values,
            variant: variant.map(str::to_string),
        });
        Ok(())
    }

    /// Resolve the composition to an ordered `Vec<Message>` (FR-012), rendering each entry —
    /// in append order — through the kernel.
    ///
    /// For each entry, in order: call [`prompting_press_core::render`] on the entry's
    /// `Prompt`'s definition with the entry's **pre-validated** value (vars were validated at
    /// [`append`](Self::append)). The render result becomes
    /// `Message { role: <def.role stringified>, text: result.text }`. Composition uses no
    /// guard expansion — a default [`GuardConfig`] is passed (guard text is never
    /// concatenated into `text`; spec 002).
    ///
    /// One entry's render failure (unknown variant, strict-undefined reference,
    /// parse/render error) propagates as the normalized [`ConsumerError`]; the partial
    /// result built so far is **discarded**, never returned as success (US4 scenario 3).
    /// An empty composition returns `Ok(vec![])` (F7).
    ///
    /// `resolve` does not mutate `self` (it takes `&self`); it reuses the kernel's render
    /// path rather than duplicating any rendering logic (FR-011 / C-01).
    ///
    /// # Errors
    ///
    /// [`ConsumerError::Kernel`] — the kernel rejected an entry's render (unknown variant,
    /// strict-undefined reference, parse/render failure). `Parse`/`Render` detail is
    /// scrubbed (FR-015).
    pub fn resolve(&self) -> Result<Vec<Message>, ConsumerError> {
        let mut messages = Vec::with_capacity(self.entries.len());

        for entry in &self.entries {
            let def = entry.prompt.definition();

            // Delegate rendering to the kernel with the already-validated value (FR-011).
            // Composition does no guard expansion; a default GuardConfig leaves `text` as-is.
            // `?` propagates a kernel failure as the normalized error and DISCARDS `messages`
            // built so far — no partial-as-success (US4 scenario 3).
            let result = prompting_press_core::render(
                def,
                entry.variant.as_deref(),
                entry.values.clone(),
                &GuardConfig::default(),
            )
            .map_err(ConsumerError::from)?;

            messages.push(Message {
                // The prompt definition's role, stringified via its Display impl.
                role: def.role.to_string(),
                text: result.text,
            });
        }

        Ok(messages)
    }
}
