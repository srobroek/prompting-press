//! # prompting-press-core
//!
//! The FFI-free engine kernel for Prompting Press — the shared core that holds the
//! template model, versioning, and variant-resolution logic. Every language binding
//! (Rust consumer, Python via PyO3, Node via napi) sits on top of this crate.
//!
//! This crate hosts the code-generated input-contract shape (`PromptDefinition` and
//! supporting types, FR-027) derived from the JSON Schema single source of truth.
//! All language bindings and the Rust consumer crate source these types from here.
//!
//! **Isolation invariant (constitution Principle II / C-02):** this crate must never
//! depend on `pyo3` or `napi`, directly or transitively. FFI concerns live exclusively
//! in the binding crates; the kernel stays a pure-Rust, portable library.

/// Code-generated shape modules, emitted from the JSON Schema single source of truth
/// by `cargo-typify` (FR-016 / constitution C-07). Marked-generated, segregated, and
/// freshness-gated in CI; never hand-edited. Regenerate via
/// `crates/prompting-press-core/scripts/codegen.sh`.
pub mod generated;

/// Re-export the generated `PromptDefinition` shape and its supporting types so
/// consumers reach them through the kernel's public surface.
pub use generated::prompt_definition;
pub use generated::prompt_definition::PromptDefinition;

/// Structured kernel error type (`KernelError`); the consumer normalizes it to the
/// common `[{field, code, message}]` shape (constitution C-06 / Principle VI).
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

/// Provenance exposure + the opt-in, additive guard expansion (`provenance_view`,
/// `ProvenanceView`, `GuardConfig`): surfaces the `untrusted`/`external` field tags and,
/// when opted in per render, names them in a separate guard instruction (constitution
/// Principle IV / C-09; research F5, FR-021..FR-025). Pure analysis; never mutates the
/// template, values, or rendered body, and never inspects or sanitizes a value.
pub mod provenance;

pub use agreement::{required_roots, Agreement};
pub use engine::{get_source, render, RenderResult};
pub use error::KernelError;
pub use provenance::{provenance_view, GuardConfig, ProvenanceView};

/// Returns the kernel's package version, sourced from Cargo at compile time.
///
/// A trivial placeholder so the crate exposes a public symbol the consumer and
/// binding crates can exercise to prove their dependency edges are real.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
