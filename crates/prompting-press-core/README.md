# prompting-press-core

The **FFI-free engine kernel** of Prompting Press — the shared Rust core that
all language bindings sit on top of (constitution Principle I / C-01).

**Isolation invariant (Principle II / C-02):** this crate must never depend on
`pyo3` or `napi`, directly or transitively. CI enforces it
(`moon run ci:check-ffi`).

Spec 001 ships this as a **stub** (it compiles and pins the dependency shape; no
engine logic yet). The render path, agreement check, variant resolution, and
hashing arrive in spec 002.
