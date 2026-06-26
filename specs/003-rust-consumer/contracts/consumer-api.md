# Contract — `prompting-press` public API (spec 003)

The consumer crate's "interface" is its **public Rust API** (a library). This fixes the surface
applications depend on. Signatures are illustrative of the *contract* — exact identifiers/generics are
settled in implementation; the invariants are normative. Native types (garde `Report`, kernel
`KernelError`) MUST NOT appear on this surface (C-06).

## Module surface

```rust
// crates/prompting-press/src/lib.rs
pub use prompting_press_core::PromptDefinition;     // re-exported kernel shape (not redefined, FR-008)
pub use prompting_press_core::RenderResult;          // re-exported kernel result (or wrapped 1:1)

pub mod registry;     // Registry: name -> PromptDefinition + dual-input loaders
pub mod render;        // validate-then-render + get_source wrappers
pub mod check;         // the agreement + provenance lint
pub mod compose;       // multi-message composition (Vec + append_*, no .chain())
pub mod error;         // ConsumerError + FieldError (the normalized shape)
// (token-count hook dropped — F4; deferred to a later spec)

pub use registry::Registry;
pub use error::{ConsumerError, FieldError};
pub use check::{CheckReport, Finding};
pub use compose::{Composition, Message};
```

## Operations (the contract)

### 1. Registry + dual-input loader (FR-005..008a)

```rust
impl Registry {
    pub fn new() -> Self;
    pub fn load_yaml(&mut self, doc: &str) -> Result<&PromptDefinition, ConsumerError>;
    pub fn load_json(&mut self, doc: &str) -> Result<&PromptDefinition, ConsumerError>;
    pub fn insert(&mut self, def: PromptDefinition) -> &PromptDefinition; // constructed object
    pub fn get(&self, name: &str) -> Option<&PromptDefinition>;
}
```

**Contract**: YAML, JSON, and a constructed object normalize to the **same** internal
`PromptDefinition` (FR-005/006); malformed input → `ConsumerError`, nothing partially loaded (FR-007);
the crate consumes the kernel's `PromptDefinition`, defines no parallel shape (FR-008); a name absent
at render/check → `ConsumerError::UnknownPrompt` (FR-008a). The crate reads no files — the caller
hands in already-read text or an object (C-03 / FR-024).

### 2. Validate-then-render (FR-001..003a, FR-009)

```rust
pub fn render<V>(
    reg: &Registry,
    name: &str,
    vars: &V,
    variant: Option<&str>,
    guard: &GuardConfig,
) -> Result<RenderResult, ConsumerError>
where
    V: serde::Serialize + garde::Validate;
```

**Contract**: validates `vars` ONCE via garde before any templating (FR-002); on failure → normalized
`ConsumerError`, **no render performed** (FR-002); on success, serializes `vars` via
`minijinja::Value::from_serialize` (FR-003a) and delegates to kernel `render`, returning the kernel's
`RenderResult` (text + provenance). Caller passes prompt-name + vars together — no per-prompt type
registration (clarify Q3). The kernel receives only already-valid values (FR-003). `get_source(reg,
name, variant) -> Result<&str, ConsumerError>` delegates to the kernel (FR-010). The crate
reimplements no render/agreement/variant/hash logic (FR-011).

### 3. Agreement + provenance lint (FR-016..020)

```rust
pub fn check(reg: &Registry) -> CheckReport;   // CheckReport { findings: Vec<Finding> }, empty = pass
```

**Contract**: for each prompt + each variant, obtains referenced roots from the kernel's
`required_roots` (does not re-derive — FR-017) and reports any root not in the prompt definition's
`variables` set as `Finding::UndeclaredVariable` (the authoritative declared set is
`def.variables` — clarify Q1). Reports an `untrusted`/`external` field used outside a declared guard
covering it as `Finding::UntrustedWithoutGuard` (via the kernel's `provenance_view` — FR-018; reframed: a declared untrusted/external field with no guard configured). **Pure
analysis**: no render, no mutation, no side effects (FR-019). Each finding names prompt + variant +
offending variable/field (FR-020). Runnable as a CI pass.

### 4. Composition (FR-012/013)

```rust
impl Composition {
    pub fn new() -> Self;
    pub fn append<V: Serialize + Validate>(&mut self, name: &str, vars: V, variant: Option<&str>) -> &mut Self;
    pub fn resolve(&self, reg: &Registry) -> Result<Vec<Message>, ConsumerError>; // Message { role, text }
}
```

**Contract**: an explicit ordered sequence; `resolve` renders each entry (validate → render) in append
order, returning `Vec<Message>` in that order (FR-012). One entry's validation failure → a
`ConsumerError` naming the entry/field; no partial-as-success (US4 scenario 3). **No `.chain()` fluent
API** (FR-013). Fragment-by-composition: render a fragment, pass its text into a parent as a declared
variable (no template includes).

### 5. Error normalization (FR-014/015)

```rust
pub struct FieldError { pub field: String, pub code: String, pub message: String }
pub enum ConsumerError { Validation(Vec<FieldError>), UnknownPrompt(String), Kernel(Vec<FieldError>) }
```

**Contract**: garde `Report` → `Vec<FieldError>` via `Report::iter()` over `(Path, Error)` —
`field = path.to_string()`, `message = error.message()`, `code` synthesized (garde exposes no machine
code — D3). Kernel `KernelError` → `Vec<FieldError>` (one row per variant, mapped code). Native types
never leak (FR-014). `Parse`/`Render` kernel detail is **sanitized**, never copied raw into
message/logs (FR-015 / SEC-004).

### 6. ~~Token-count hook~~ — DROPPED (F4)

The token-count hook is removed from spec 003 (deferred to a later spec; analyze F4). The crate exposes
no token-counting seam and ships no counter.

## Cross-cutting invariants

| Invariant | FR / SC | Enforcement |
|---|---|---|
| FFI-free (no pyo3/napi) | SC-007, C-02 | `ci:check-ffi` gate; garde + serde_yaml_ng are pure-Rust (research D1/D2) |
| No render/agreement/variant/hash logic here | FR-011, C-01 | all such ops are kernel calls; review |
| Validation-blind kernel | FR-003 | only validated values reach the kernel; `Value::from_serialize` after `validate()` |
| Native error types don't leak | FR-014, C-06 | `ConsumerError` is the only public error type |
| No I/O / no output parsing | FR-024, C-03 | crate takes text/objects in; output_model carried as metadata only |
| `check()` is pure | FR-019 | no render, no mutation, deterministic `BTreeSet`/`BTreeMap` ordering |
