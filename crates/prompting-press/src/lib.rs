//! # prompting-press
//!
//! The public Rust consumer surface for Prompting Press. Rust applications depend on
//! this crate (not the kernel directly) for a stable, idiomatic API; it re-exports and
//! wraps [`prompting_press_core`].
//!
//! Like the kernel, this crate is FFI-free: it must never pull in `pyo3` or `napi`
//! (constitution Principle II / C-02).
//!
//! This is a spec-001 stub: the dependency edge onto the kernel is real, but the public
//! API is not yet built out.

/// Re-export of the kernel, so consumers can reach core types through one entry point.
pub use prompting_press_core as core;

/// Re-export the generated `PromptDefinition` shape and its supporting types from the
/// kernel, so consumers reach them through this crate's public surface rather than
/// depending on the kernel directly. This crate re-exports but NEVER hand-edits the
/// generated module (which lives in `prompting-press-core`).
pub use prompting_press_core::generated::prompt_definition;
pub use prompting_press_core::generated::prompt_definition::PromptDefinition;

/// Returns the underlying kernel version.
///
/// Trivial placeholder that calls into the kernel, making the dependency edge
/// load-bearing rather than declarative-only.
#[must_use]
pub fn core_version() -> &'static str {
    prompting_press_core::version()
}
