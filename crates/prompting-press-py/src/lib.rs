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
//! Later phases add `render` (US1), `check` (US3), and `compose` (US4) — see the placeholders
//! in [`prompting_press_py`].
//!
//! [PyO3]: https://pyo3.rs

use pyo3::prelude::*;

pub mod error;
pub mod marshal;
pub mod registry;

// T0NN (US1): pub mod render;   — validate → marshal → kernel-direct render + get_source.
// T0NN (US3): pub mod check;    — `check(registry)` + CheckReport / Finding pyclasses.
// T0NN (US4): pub mod compose;  — Composition / Message; eager-validate append; resolve loop.

/// Returns the kernel version, reached through the Rust consumer surface.
///
/// Retained from the spec-001 stub so the extension module exports a trivial callable and the
/// dependency edge onto `prompting-press`/`prompting-press-core` stays load-bearing.
#[pyfunction]
fn core_version() -> &'static str {
    prompting_press::core_version()
}

/// The native module Python imports as `prompting_press_py` (re-exported to callers as
/// `prompting_press` by the maturin `module-name` setting).
#[pymodule]
fn prompting_press_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(core_version, m)?)?;

    // The Registry pyclass (T007).
    m.add_class::<registry::Registry>()?;

    // The exception hierarchy + the FieldError row class (T006).
    error::register(m)?;

    // T0NN (US1): m.add_function(wrap_pyfunction!(render::render, m)?)?;
    //             m.add_function(wrap_pyfunction!(render::get_source, m)?)?;
    // T0NN (US3): m.add_function(wrap_pyfunction!(check::check, m)?)?;
    //             m.add_class::<check::CheckReport>()?; m.add_class::<check::Finding>()?;
    // T0NN (US4): m.add_class::<compose::Composition>()?; m.add_class::<compose::Message>()?;

    Ok(())
}
