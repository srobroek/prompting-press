//! # prompting-press-core
//!
//! The FFI-free engine kernel for Prompting Press — the shared core that holds the
//! template model, versioning, and variant-resolution logic. Every language binding
//! (Rust consumer, Python via PyO3, Node via napi) sits on top of this crate.
//!
//! **Isolation invariant (constitution Principle II / C-02):** this crate must never
//! depend on `pyo3` or `napi`, directly or transitively. FFI concerns live exclusively
//! in the binding crates; the kernel stays a pure-Rust, portable library.
//!
//! This is a spec-001 stub: it compiles and pins the dependency shape, but carries no
//! real engine logic yet.

/// Returns the kernel's package version, sourced from Cargo at compile time.
///
/// A trivial placeholder so the crate exposes a public symbol the consumer and
/// binding crates can exercise to prove their dependency edges are real.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
