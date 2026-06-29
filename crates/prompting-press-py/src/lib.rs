//! # prompting-press-py
//!
//! The Python binding for Prompting Press, built with [PyO3]. This crate exposes the Rust
//! consumer surface ([`prompting_press`]) to Python as a native extension module.
//!
//! It is one of exactly two crates (the other being `prompting-press-node`) permitted to
//! depend on an FFI toolkit; the kernel and the Rust consumer stay FFI-free (constitution
//! Principle II / C-02). The binding adds **no** engine logic — it marshals to the shared
//! Rust core (Principle I / C-01).
//!
//! ## Module map
//!
//! Foundational phase (T004–T007) wires the blocking core:
//! - [`marshal`] — the one FFI value bridge (Python value → `minijinja::Value`).
//! - [`error`] — the exception hierarchy + `ConsumerError`/`KernelError` → `PyErr` translation
//!   (SEC-004 scrub preserved).
//!
//! Later phases add the render/check/compose paths:
//! - [`render`] — the `RenderResult` and `GuardConfig` pyclasses + the `validate_in_python`
//!   helper (used by `prompt.rs` and `compose.rs`).
//! - [`check`] — the `CheckReport` / `Finding` pyclasses; the lint is `prompt.check()` now.
//! - [`compose`] — the binding-owned `Composition` / `Message` pyclasses: eager-validate each
//!   entry (reusing the US1 validation path), then a kernel-direct resolve loop (US4).
//!
//! Phase 4 (spec 008 T035–T038) adds the immutable `Prompt` object:
//! - [`prompt`] — the `Prompt` pyclass: validating construction, `from_yaml`/`from_json`/
//!   `from_toml`, read-only properties, `render`/`get_source`/`check`, and `derive` (the sole
//!   mutator). Primary public type; replaces the former Registry-based split surface.
//!
//! [PyO3]: https://pyo3.rs

use pyo3::prelude::*;

pub mod check;
pub mod compose;
pub mod error;
pub mod marshal;
pub mod prompt;
pub mod render;

/// Returns the kernel version, reached through the Rust consumer surface.
///
/// Retained from the spec-001 stub so the extension module exports a trivial callable and the
/// dependency edge onto `prompting-press`/`prompting-press-core` stays load-bearing.
#[pyfunction]
fn core_version() -> &'static str {
    prompting_press::core_version()
}

/// The native extension module. CPython binds an extension by the `PyInit_<name>` symbol, and
/// PyO3 derives that symbol from this `#[pymodule]` function's name — so it MUST match maturin's
/// `module-name = "prompting_press"` (pyproject.toml), or `import prompting_press` fails with a
/// missing `PyInit_prompting_press`. The `#[pyo3(name = "prompting_press")]` attribute sets the
/// module name WITHOUT renaming the Rust `fn` — keeping the `fn prompting_press_py` identifier
/// (so the `prompting_press::core_version()` crate-path call above still resolves; renaming the fn
/// to `prompting_press` would shadow the crate).
#[pymodule]
#[pyo3(name = "prompting_press")]
fn prompting_press_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(core_version, m)?)?;

    // The exception hierarchy + the FieldError row class (T006).
    error::register(m)?;

    // The render output + guard config pyclasses (US1, T010/T011). GuardConfig is the opt-in
    // guard plumbed through to the kernel (FR-009). The module-level render/get_source free
    // functions are removed (spec 008 Phase 4) — use Prompt.render / Prompt.get_source.
    m.add_class::<render::RenderResult>()?;
    m.add_class::<render::GuardConfig>()?;

    // The lint report pyclasses (US3, T017). The module-level check(reg) free function is
    // removed (spec 008 Phase 4) — use prompt.check() per Prompt object.
    m.add_class::<check::CheckReport>()?;
    m.add_class::<check::Finding>()?;

    // Multi-message composition: the binding-owned Composition + Message pyclasses (US4, T020).
    // Composition aggregates Prompt objects (spec 008 Phase 4 reshape).
    m.add_class::<compose::Composition>()?;
    m.add_class::<compose::Message>()?;

    // The immutable Prompt object (spec 008 Phase 4, T035–T038). Primary public type.
    m.add_class::<prompt::Prompt>()?;

    Ok(())
}
