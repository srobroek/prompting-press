//! # prompting-press-py
//!
//! The Python binding for Prompting Press, built with [PyO3]. This crate exposes the
//! Rust consumer surface ([`prompting_press`]) to Python as a native extension module.
//!
//! It is one of exactly two crates (the other being `prompting-press-node`) permitted
//! to depend on an FFI toolkit; the kernel and the Rust consumer stay FFI-free
//! (constitution Principle II / C-02).
//!
//! This is a spec-001 stub: it registers a single trivial function to prove the PyO3
//! module wiring works end to end.
//!
//! [PyO3]: https://pyo3.rs

use pyo3::prelude::*;

/// Returns the kernel version, reached through the Rust consumer surface.
///
/// Placeholder so the extension module exports something callable from Python and so
/// the dependency edges onto `prompting-press`/`prompting-press-core` are exercised.
#[pyfunction]
fn core_version() -> &'static str {
    prompting_press::core_version()
}

/// The native module Python imports as `prompting_press_py`.
#[pymodule]
fn prompting_press_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(core_version, m)?)?;
    Ok(())
}
