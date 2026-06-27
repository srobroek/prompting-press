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
//! - [`registry`] — the `Registry` `#[pyclass]` (construct + insert; loaders are US2).
//!
//! Later phases add the render/check/compose paths:
//! - [`render`] — validate-in-Python → marshal → kernel-direct render + `get_source` (US1).
//! - [`check`] — `check(registry)` + the `CheckReport` / `Finding` pyclasses (US3).
//! - [`compose`] — the binding-owned `Composition` / `Message` pyclasses: eager-validate each
//!   entry (reusing the US1 validation path), then a kernel-direct resolve loop (US4).
//!
//! [PyO3]: https://pyo3.rs

use pyo3::prelude::*;

pub mod check;
pub mod compose;
pub mod error;
pub mod marshal;
pub mod registry;
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

    // The Registry pyclass (T007).
    m.add_class::<registry::Registry>()?;

    // The exception hierarchy + the FieldError row class (T006).
    error::register(m)?;

    // The render path: render / get_source + the RenderResult and GuardConfig pyclasses
    // (US1, T010/T011). GuardConfig is the opt-in guard plumbed through to the kernel (FR-009).
    m.add_class::<render::RenderResult>()?;
    m.add_class::<render::GuardConfig>()?;
    m.add_function(wrap_pyfunction!(render::render, m)?)?;
    m.add_function(wrap_pyfunction!(render::get_source, m)?)?;

    // The agreement + provenance lint: check(registry) + the CheckReport / Finding pyclasses
    // (US3, T017). Pure CI lint marshaled to the consumer's `check` (C-01); nothing is re-derived.
    m.add_class::<check::CheckReport>()?;
    m.add_class::<check::Finding>()?;
    m.add_function(wrap_pyfunction!(check::check, m)?)?;

    // Multi-message composition: the binding-owned Composition + Message pyclasses (US4, T020).
    // Composition eager-validates each entry via the US1 Python-validation path, then resolves
    // through the kernel directly (critique E1 / C-01) — no engine logic in the binding.
    m.add_class::<compose::Composition>()?;
    m.add_class::<compose::Message>()?;

    Ok(())
}
