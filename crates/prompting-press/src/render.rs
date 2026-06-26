//! Validate-then-render + `get_source` wrappers (spec 003, T009/T010; FR-001..003a,
//! FR-009/FR-010/FR-011).
//!
//! This is the consumer's headline call path: it takes a prompt **name** (resolved against
//! the [`Registry`]) plus the caller's typed Vars value, validates the vars **once**, and —
//! only on success — bridges them to the kernel's value type and delegates rendering. The
//! consumer adds **no** rendering, agreement, variant-resolution, or hashing logic; those
//! live once in the kernel and are wrapped here (constitution Principle I / FR-011).
//!
//! ## The validate → serialize → kernel chain (FR-002/FR-003/FR-003a)
//!
//! 1. **Resolve** the prompt by name against the registry; absent ⇒
//!    [`ConsumerError::UnknownPrompt`] (FR-008a), never a panic.
//! 2. **Validate** the whole Vars set ONCE via garde, *before any templating* (FR-002). On
//!    failure the garde [`Report`](garde::Report) is normalized to
//!    [`ConsumerError::Validation`] and **no render is performed** — the kernel is never
//!    reached.
//! 3. **Bridge** the now-validated struct to the kernel's value type with
//!    [`minijinja::Value::from_serialize`] (FR-003a). The caller never hand-builds a value
//!    map; the same struct garde validated is serialized in.
//! 4. **Delegate** to [`prompting_press_core::render`], normalizing any
//!    [`KernelError`](prompting_press_core::KernelError) to [`ConsumerError::Kernel`].
//!
//! ## Three-sets invariant (spec Assumptions / critique E1)
//!
//! The caller's garde Vars *field names* must agree with the prompt's declared `variables`.
//! garde validates the struct's **values**; the agreement check (a CI lint, a later phase)
//! validates *template-roots ⊆ `variables`*; but the **struct ↔ `variables`** field-name
//! agreement is the caller's responsibility. A mismatch (e.g. a struct field `usrname` where
//! the template references `username`) is **not silent**: the serialized value lacks the
//! referenced root, so the kernel's strict-undefined fires and surfaces here as a normalized
//! [`ConsumerError::Kernel`] carrying an [`code::UNDEFINED_VARIABLE`](crate::error::code)
//! row — a loud error, never an empty render. Closing this in-library would require the
//! per-prompt type registration that clarify Q3 deliberately rejected for v1; it is pinned
//! by a test, not enforced by an extra check.

use garde::Validate;
use prompting_press_core::{GuardConfig, RenderResult};
use serde::Serialize;

use crate::{ConsumerError, Registry};

/// Validate `vars`, then render `name`'s resolved variant through the kernel (FR-009).
///
/// The caller passes the prompt **name** (resolved against `reg`) and the typed Vars value
/// **together** — there is no per-prompt type registration (clarify Q3). `variant`
/// selects an arm (`None` ⇒ the reserved `default` = root body); `guard` is plumbed
/// straight through to the kernel, whose [`RenderResult::guard`] field is surfaced
/// unchanged (guard *expansion* is the kernel's contract — spec 002 / F5; this crate only
/// plumbs and surfaces it).
///
/// `V::Context: Default` so the whole-struct [`Validate::validate`] convenience applies
/// (one validation pass over the entire input set, FR-002).
///
/// # Errors
/// - [`ConsumerError::UnknownPrompt`] — `name` is absent from `reg` (FR-008a). Returned
///   **before** validation; nothing is rendered.
/// - [`ConsumerError::Validation`] — garde rejected `vars`. Returned **before** any
///   templating (FR-002); the kernel is never reached. Every offending field is named.
/// - [`ConsumerError::Kernel`] — the kernel rejected the render (unknown variant, a
///   strict-undefined reference, a parse / render failure). `Parse` / `Render` detail is
///   scrubbed (FR-015).
pub fn render<V>(
    reg: &Registry,
    name: &str,
    vars: &V,
    variant: Option<&str>,
    guard: &GuardConfig,
) -> Result<RenderResult, ConsumerError>
where
    V: Serialize + Validate,
    V::Context: Default,
{
    // 1. Resolve the prompt by name (absent ⇒ structured error, never a panic).
    let def = reg
        .get(name)
        .ok_or_else(|| ConsumerError::UnknownPrompt(name.to_string()))?;

    // 2. Validate the WHOLE input set once, BEFORE any templating (FR-002). On failure the
    //    garde Report is normalized via the `From<garde::Report>` impl and the kernel is
    //    never reached — the returned `Validation` variant is itself the proof no render
    //    was attempted.
    vars.validate().map_err(ConsumerError::from)?;

    // 3. Bridge the validated struct to the kernel's value type (FR-003a). `vars: &V` and
    //    `&V: Serialize` (a reference to a Serialize type is Serialize), so no clone.
    let values = minijinja::Value::from_serialize(vars);

    // 4. Delegate rendering to the kernel; normalize KernelError → ConsumerError::Kernel.
    //    The kernel receives ONLY already-validated values (FR-003); the consumer adds no
    //    render/agreement/variant/hash logic of its own (FR-011).
    prompting_press_core::render(def, variant, values, guard).map_err(ConsumerError::from)
}

/// Return a prompt variant's **unrendered** template source, delegating to the kernel
/// (FR-010).
///
/// Pure source lookup: there are no vars to validate, so this performs no validation and no
/// bridging — it resolves the prompt by name and asks the kernel for the resolved variant's
/// source bytes (the exact string the kernel hashes into `template_hash`).
///
/// # Errors
/// - [`ConsumerError::UnknownPrompt`] — `name` is absent from `reg` (FR-008a).
/// - [`ConsumerError::Kernel`] — the kernel rejected the lookup (e.g. an unknown variant).
pub fn get_source<'a>(
    reg: &'a Registry,
    name: &str,
    variant: Option<&str>,
) -> Result<&'a str, ConsumerError> {
    let def = reg
        .get(name)
        .ok_or_else(|| ConsumerError::UnknownPrompt(name.to_string()))?;

    prompting_press_core::get_source(def, variant).map_err(ConsumerError::from)
}
