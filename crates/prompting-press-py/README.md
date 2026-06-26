# prompting-press-py

The **PyO3 binding crate** (cdylib) — one of exactly two crates permitted to
depend on an FFI toolkit (`pyo3`); the kernel and Rust consumer stay FFI-free
(Principle II / C-02). It exposes the Rust consumer surface to Python as a native
extension module; `packages/python/` wraps it into a `maturin`-built wheel.

`build.rs` adds `-undefined dynamic_lookup` on macOS only (PyO3 `extension-module`
leaves CPython symbols for load-time resolution); scoped to this crate so it never
enters `RUSTFLAGS` (keeps the codegen-determinism and FFI-isolation gates clean).

Spec 001 ships this as a **stub** (a trivial `#[pymodule]`). Marshaling + the
Pydantic facade land in spec 004.
