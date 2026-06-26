# prompting-press-core

The **FFI-free engine kernel** of Prompting Press — the shared Rust core that
all language bindings sit on top of (constitution Principle I / C-01).

**Isolation invariant (Principle II / C-02):** this crate must never depend on
`pyo3` or `napi`, directly or transitively. CI enforces it
(`moon run ci:check-ffi`).

Spec 002 moves the code-generated `PromptDefinition` input-contract shape (FR-027)
into this crate (`src/generated/`, schema-derived — regenerate via
`bash crates/prompting-press-core/scripts/codegen.sh` or
`moon run prompting-press-core:codegen`; do not hand-edit). The render path,
agreement check, variant resolution, and hashing also arrive in spec 002.
