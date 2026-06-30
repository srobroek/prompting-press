//! # prompting-press-core
//!
//! The FFI-free engine kernel for Prompting Press — the single, shared source of
//! rendering behavior that every language binding (the Rust consumer, Python via PyO3,
//! Node via napi) sits on top of. Because rendering, agreement analysis, variant
//! resolution, and hashing all happen **once, here, in Rust**, cross-language output
//! equality is a *structural* property, not something each binding re-verifies
//! (constitution Principle I).
//!
//! It hosts the code-generated input-contract shape ([`PromptDefinition`] and supporting
//! types, FR-027) derived from the JSON Schema single source of truth; all bindings and
//! the Rust consumer crate source these types from here.
//!
//! ## What the kernel does
//!
//! Four capabilities, all pure and I/O-free:
//!
//! 1. **Render** ([`render`]) — turns a [`PromptDefinition`] + already-validated values
//!    into rendered text, using a MiniJinja environment restricted to **interpolation,
//!    conditionals, and loops** with **strict-undefined** handling (a missing variable is
//!    a loud [`KernelError::UndefinedVariable`], never a silent empty string). The six
//!    excluded features (`{% include %}`, `{% extends %}`, `{% import %}`,
//!    `{% from … import %}`, `{% macro %}`, `{% block %}`) are rejected at parse time as
//!    [`KernelError::ExcludedFeature`] / [`KernelError::Parse`] (FR-002).
//! 2. **Agreement analysis** ([`required_roots`]) — the headline differentiator
//!    (Principle IV): reports, per resolved variant, the set of **root** variable names a
//!    template references (via MiniJinja's stable `undeclared_variables(false)` minus the
//!    engine's own globals). The kernel only *returns* the set; the `referenced ⊆ declared`
//!    comparison is the consumer's lint (FR-019).
//! 3. **Variant resolution + provenance / hashing** ([`get_source`], [`RenderResult`]) —
//!    `None`/`"default"` resolves to the root `body`; a named arm resolves to that variant
//!    (unknown ⇒ [`KernelError::UnknownVariant`]). Each [`RenderResult`] carries
//!    `template_hash = SHA256(variant source)` and `render_hash = SHA256(rendered text)`,
//!    as plain data on the return value — no telemetry sink, and there is no `vars_hash`
//!    (Principle V).
//! 4. **Var-trust query + opt-in guard** ([`untrusted_fields`], [`GuardConfig`]) —
//!    reports which fields are declared `trusted: false` and, when `GuardConfig.enabled`
//!    is true, wraps untrusted interpolations in `<untrusted>…</untrusted>` in the
//!    rendered body and places a fixed advisory in [`RenderResult::guard`].
//!
//! ## Invariants
//!
//! - **FFI-free (Principle II / C-02).** This crate must never depend on `pyo3`, `napi`,
//!   or any FFI binding crate, directly or transitively (a CI `cargo tree -i` gate
//!   enforces it). FFI concerns live exclusively in the binding crates; the kernel stays a
//!   pure-Rust, portable library.
//! - **Validation-blind (FR-004).** The kernel receives *already-validated* values
//!   (a [`minijinja::Value`]) and performs no type validation, coercion, or constraint
//!   checking. It knows nothing of Pydantic / Zod / garde.
//! - **No I/O (Principle III / C-03).** No file, network, database, or environment access;
//!   no model/LLM call; no provider-request assembly; no token counting; no output
//!   parsing. The caller *pushes* data in.
//! - **Error normalization is the consumer's job, not the kernel's (C-06 / Principle VI).**
//!   The kernel returns its native [`KernelError`]; normalizing it to the common
//!   `[{field, code, message}]` shape happens at each binding boundary (spec 003+), never
//!   here. See [`error`].
//!
//! ## What the kernel does NOT do with what it returns (normative — critique X1 / SEC-002)
//!
//! These are guarantees a consumer **must not** mistake for protections the kernel
//! provides. The kernel is a renderer and analyzer, not a sanitizer or an enforcer:
//!
//! - **The guard advisory does NOT sanitize.** The `RenderResult::guard` string is a
//!   fixed advisory referencing the `<untrusted>…</untrusted>` markers. The actual value
//!   delimiting happens in the rendered body (when `GuardConfig.enabled` is true and
//!   untrusted fields exist), but the delimiters are a suggestion to the downstream model
//!   about where user-supplied data starts and ends — not a security barrier.
//! - **`trusted` is declarative metadata.** The `trusted: bool` flag on a field drives
//!   the opt-in guard delimiting and is queryable via [`untrusted_fields`], but the kernel
//!   never gates or blocks rendering based on it when the guard is off. With the guard
//!   disabled, a template interpolating a `trusted: false` field renders exactly as one
//!   interpolating a `trusted: true` field.
//! - **`output_model` is a reference that is never parsed.** The `output_model` field on a
//!   definition is carried as a metadata reference only; the kernel never parses, validates
//!   against, or coerces anything to it (Principle III).
//!
//! ## Example
//!
//! Deserialize a [`PromptDefinition`] from its canonical JSON form and render it. The
//! rendered text is byte-identical across runs and languages (Principle I), and the
//! result carries content-addressed provenance hashes.
//!
//! ```
//! use prompting_press_core::{render, GuardConfig, PromptDefinition};
//!
//! // Canonical JSON input (the same shape as the published YAML/JSON form).
//! let def: PromptDefinition = serde_json::from_str(
//!     r#"{
//!         "name": "greet",
//!         "role": "user",
//!         "body": "Hello {{ name }}",
//!         "variables": {
//!             "name": { "type": "string", "trusted": true }
//!         }
//!     }"#,
//! )
//! .expect("valid prompt definition");
//!
//! let values = minijinja::Value::from_serialize(serde_json::json!({ "name": "Ada" }));
//! let no_guard = GuardConfig::default(); // { enabled: false } — guard opt-out
//!
//! let result = render(&def, None, values, &no_guard).expect("render succeeds");
//!
//! assert_eq!(result.text, "Hello Ada");
//! assert_eq!(result.variant, "default"); // no variant named -> root body
//! assert_eq!(result.template_hash.len(), 64); // lowercase-hex SHA-256
//! assert!(result.guard.is_none()); // guard opt-out
//! ```

/// Code-generated shape modules, emitted from the JSON Schema single source of truth
/// by `cargo-typify` (FR-016 / roadmap decision C-07). Marked-generated, segregated, and
/// freshness-gated in CI; never hand-edited. Regenerate via
/// `crates/prompting-press-core/scripts/codegen.sh`.
pub mod generated;

/// Re-export the generated `PromptDefinition` shape and its supporting types so
/// consumers reach them through the kernel's public surface.
pub use generated::prompt_definition;
pub use generated::prompt_definition::PromptDefinition;

/// Structured kernel error type (`KernelError`); the consumer normalizes it to the
/// common `[{field, code, message}]` shape (roadmap decision C-06 (Principle VI)).
pub mod error;

/// Engine construction + the render path and variant resolution: the canonical
/// strict-undefined MiniJinja environment, `render`, and `get_source`
/// (research D1/D3, FR-001a/FR-002/FR-006..FR-013).
pub mod engine;

/// Content-addressed hashing helpers (`template_hash` / `render_hash`); pure-Rust
/// SHA-256 over the UTF-8 string content (research D8, FR-012/FR-013). No `vars_hash`.
mod hashing;

/// The sound agreement analysis (`required_roots`): the library's headline differentiator
/// (constitution Principle IV / C-04). Reports, per resolved variant, the set of root
/// variable names a template references via MiniJinja's stable `undeclared_variables(false)`
/// minus an env-derived globals allowlist (research D2, FR-016..FR-020).
pub mod agreement;

/// Var-trust query, opt-in guard instruction, and injection-resistant delimiting
/// (spec 015; `GuardConfig`, `untrusted_fields`, `build_guard_text`,
/// `apply_guard_prepass`, `guard_wrap_filter`). When `GuardConfig.enabled` is true
/// and the definition has untrusted fields (`trusted: false`), untrusted values are
/// wrapped in `<untrusted>…</untrusted>` in the rendered body, and a fixed advisory
/// is placed in `RenderResult::guard`. Pure analysis; never mutates the template,
/// values, or rendered body outside of the opt-in pre-pass (Principle III / C-03).
/// (The per-variable tag was `origin` enum through spec 014; spec 015 collapsed it
/// to a required `trusted: bool`.)
pub mod origin;

pub use agreement::{required_roots, Agreement};
pub use engine::{get_source, render, RenderResult};
pub use error::KernelError;
pub use origin::{untrusted_fields, GuardConfig, DEFAULT_GUARD_ADVISORY};

/// Returns the kernel's package version, sourced from Cargo at compile time.
///
/// A trivial placeholder so the crate exposes a public symbol the consumer and
/// binding crates can exercise to prove their dependency edges are real.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
