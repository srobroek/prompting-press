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

/// Returns the kernel's package version, sourced from Cargo at compile time.
///
/// A trivial placeholder so the crate exposes a public symbol the consumer and
/// binding crates can exercise to prove their dependency edges are real.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
