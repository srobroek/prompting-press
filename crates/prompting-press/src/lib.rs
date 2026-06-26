//! # prompting-press
//!
//! The public Rust consumer surface for Prompting Press. Rust applications depend on
//! this crate (not the kernel directly) for a stable, idiomatic API; it re-exports and
//! wraps [`prompting_press_core`].
//!
//! Like the kernel, this crate is FFI-free: it must never pull in `pyo3` or `napi`
//! (constitution Principle II / C-02).
//!
//! Spec-003 build-out is in progress: the dependency edge onto the kernel is real, and the
//! normalized error surface ([`error`]), prompt [`registry`], and the validate-then-render
//! [`render`] path are now in place. The `check` / `compose` modules arrive in later phases
//! (their module declarations are added as each file is created).

/// Re-export of the kernel, so consumers can reach core types through one entry point.
pub use prompting_press_core as core;

/// Re-export the generated `PromptDefinition` shape and its supporting types from the
/// kernel, so consumers reach them through this crate's public surface rather than
/// depending on the kernel directly. This crate re-exports but NEVER hand-edits the
/// generated module (which lives in `prompting-press-core`).
pub use prompting_press_core::generated::prompt_definition;
pub use prompting_press_core::generated::prompt_definition::PromptDefinition;

/// Re-export the kernel's `RenderResult` (library-owned render output; FR-009). The
/// consumer surfaces it 1:1 rather than redefining a parallel shape (C-01).
pub use prompting_press_core::RenderResult;

/// The normalized error surface: [`ConsumerError`] + [`FieldError`], the ONLY error type on
/// this crate's public API. garde `Report` / kernel `KernelError` are normalized here and
/// never leak (Principle VI / C-06; FR-014/FR-015).
pub mod error;

/// The prompt [`Registry`]: name → `PromptDefinition`. Backed by a `BTreeMap` for
/// deterministic `check()` ordering (FR-008a).
pub mod registry;

/// Validate-then-render + `get_source` wrappers over the kernel (FR-001..003a, FR-009/010).
pub mod render;

pub use error::{ConsumerError, FieldError};
pub use registry::Registry;

/// Re-export the validate-then-render entry points at the crate root so applications reach
/// them as `prompting_press::render` / `prompting_press::get_source`.
pub use render::{get_source, render};

/// Returns the underlying kernel version.
///
/// Trivial placeholder that calls into the kernel, making the dependency edge
/// load-bearing rather than declarative-only.
#[must_use]
pub fn core_version() -> &'static str {
    prompting_press_core::version()
}
