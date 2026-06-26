# prompting-press

The **public Rust consumer surface** for Prompting Press — the crate Rust
applications depend on (not the kernel directly). It layers an idiomatic API over
[`prompting-press-core`] and re-exports the code-generated prompt-definition types
(generated in `prompting-press-core/src/generated/`, schema-derived — do not hand-edit).

Like the kernel, this crate is **FFI-free**: no `pyo3`, no `napi` (Principle II /
C-02; CI-enforced).

Spec 001 ships this as a **stub**: the generated shape and the kernel dependency
edge are real; the typed-Vars facade, loader, and `render()`/`check()` API land in
spec 003.

[`prompting-press-core`]: ../prompting-press-core
