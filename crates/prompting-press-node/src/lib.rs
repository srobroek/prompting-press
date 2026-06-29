//! # prompting-press-node
//!
//! The Node.js binding for Prompting Press, built with [napi-rs] (Node-API). This crate exposes
//! the Rust consumer surface ([`prompting_press`]) to Node.js as a native addon.
//!
//! It is one of exactly two crates (the other being `prompting-press-py`) permitted to depend on
//! an FFI toolkit; the kernel and the Rust consumer stay FFI-free (constitution Principle II /
//! C-02). The binding adds **no** engine logic — it marshals to the shared Rust core
//! (Principle I / C-01).
//!
//! ## Module map
//!
//! Foundational phase (T004–T007) wires the blocking core:
//! - [`marshal`] — the one FFI value bridge (JS value → `minijinja::Value`).
//! - [`error`] — the `ConsumerError`/`KernelError` → `napi::Error` translation carrying a
//!   structured, already-scrubbed payload the TS facade decodes (SEC-004 scrub preserved).
//! - [`registry`] — the `Registry` `#[napi]` class (construct + insert + the US2 loaders).
//!
//! Later phases add the render/check/compose paths:
//! - [`render`] — marshal → kernel-direct render + `getSource` (US1; validation is the TS facade's).
//! - [`check`] — `check(registry)` + the `CheckReport` / `Finding` types (US3).
//! - [`compose`] — the binding-owned `Composition` / `Message` types: marshal each (already
//!   TS-validated) entry, then a kernel-direct resolve loop (US4).
//!
//! ## Registration
//!
//! napi-rs auto-registers every `#[napi]` item through its module-register machinery (the `ctor`
//! constructor the macro emits) — there is **no** manual class/function registration step (unlike
//! PyO3's `#[pymodule]`). Declaring the modules below and annotating the items with `#[napi]` is
//! sufficient for them to appear on the generated addon + its `index.d.ts`.
//!
//! [napi-rs]: https://napi.rs

use napi_derive::napi;

pub mod check;
pub mod compose;
pub mod error;
pub mod marshal;
pub mod prompt;
pub mod registry;
pub mod render;

/// Returns the kernel version, reached through the Rust consumer surface.
///
/// Retained from the spec-001 stub so the addon exports a trivial callable and the dependency
/// edges onto `prompting-press`/`prompting-press-core` stay load-bearing. Surfaces as
/// `coreVersion` in JS (napi renames snake_case → camelCase).
#[napi]
#[must_use]
pub fn core_version() -> &'static str {
    prompting_press::core_version()
}
