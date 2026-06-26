# Contract — `prompting-press-core` public API (spec 002)

The kernel's "interface" is its **public Rust API** (this is a library, not a service). This contract
fixes the signatures the Rust consumer (003) and the binding crates (004/005) depend on. Types are from
the data model. Signatures are illustrative of the *contract*, not final names — the invariants and
behaviors are normative; exact identifiers are settled in implementation.

## Module surface

```rust
// crates/prompting-press-core/src/lib.rs
pub mod generated;          // relocated spec-001 shape (research D6); pub use generated::prompt_definition::PromptDefinition;
pub mod engine;             // Environment construction + render
pub mod agreement;          // undeclared-variables analysis
pub mod provenance;         // tag exposure + guard expansion
pub mod hashing;            // sha256 helpers
pub mod error;              // KernelError

pub use generated::prompt_definition::PromptDefinition;
pub use error::KernelError;
pub use engine::{RenderResult, GuardConfig};
pub use agreement::Agreement;
pub use provenance::ProvenanceView;
```

## Operations (the contract)

### 1. Render

```rust
pub fn render(
    def: &PromptDefinition,
    variant: Option<&str>,
    values: minijinja::Value,
    guard: &GuardConfig,
) -> Result<RenderResult, KernelError>;
```

**Contract**:
- Resolves the variant per the data-model rule (None/`"default"` → root `body`; named → that arm;
  unknown → `Err(UnknownVariant)`). [FR-007, FR-008, FR-009, FR-011]
- Renders with an `Environment` configured `UndefinedBehavior::Strict` and **`macros`/`multi_template`
  disabled**; a template using an excluded feature returns `Err(ExcludedFeature|Parse)` and a
  strict-undefined hit returns `Err(UndefinedVariable)`. [FR-001a, FR-002]
- Deterministic: identical `(def, variant, values)` ⇒ byte-identical `text`, `template_hash`,
  `render_hash`. [FR-003, SC-001]
- `RenderResult.text` is the rendered body **only**; `guard` is a separate field, populated iff
  `guard.enabled` (never concatenated into `text`). [FR-022, FR-023, SC-005]
- Emits `template_hash = hex(SHA256(resolved source))`, `render_hash = hex(SHA256(text))`; **no
  vars_hash**. [FR-012, FR-013, FR-014]
- Performs no I/O, no model call, no request-body assembly, no token counting, no output parsing.
  [FR-005]

### 2. Get source (unrendered)

```rust
pub fn get_source<'a>(
    def: &'a PromptDefinition,
    variant: Option<&str>,
) -> Result<&'a str, KernelError>;
```

**Contract**: returns the exact unrendered source string of the resolved arm — the same bytes
`template_hash` is computed over. Same resolution + `UnknownVariant` rule as render. [FR-006, FR-012]

### 3. Agreement analysis (required roots)

```rust
pub fn required_roots(
    def: &PromptDefinition,
    variant: Option<&str>,
) -> Result<Agreement, KernelError>;
```

**Contract**:
- Returns the **per-variant** set of referenced **root** variable names via
  `Template::undeclared_variables(false)`, minus the env-derived globals allowlist. [FR-016, FR-017]
- Excludes loop variables, `{% set %}` targets, and block/with locals (guaranteed by the engine
  analysis). [FR-017, SC-002]
- **Pure**: does not render, does not mutate `def`, values, or any output. [FR-018, SC-006]
- Does **not** compare against declared `variables` (consumer's job). [FR-019]
- A template that fails to parse (incl. an excluded feature) returns `Err`, never an empty/successful
  analysis (guards the `undeclared_variables` "empty-set-on-parse-error" footgun, research D2). [FR-028]

### 4. Provenance view

```rust
pub fn provenance_view(def: &PromptDefinition) -> ProvenanceView;
```

**Contract**: returns the `untrusted` / `external` field-name sets from `def.variables[*].provenance`.
Pure, no mutation. [FR-021]

### 5. Guard expansion (internal to render; surfaced via `GuardConfig` + `RenderResult.guard`)

**Contract**: when `guard.enabled`, the kernel produces guard instruction text from
`guard.template` (or the kernel default) naming the union of untrusted+external fields, and returns it
in `RenderResult.guard`. Additive and non-mutating: template, values, and `text` are unchanged; no
value is sanitized/stripped/escaped-away. [FR-022, FR-023, FR-024, FR-025, SC-005]

## Cross-cutting invariants (apply to all operations)

| Invariant | FR / SC | Enforcement |
|---|---|---|
| Kernel has zero `pyo3`/`napi`/FFI deps | SC-007, C-02 | spec-001 `check-ffi` gate (`cargo tree -i`) stays green; new deps `minijinja`,`sha2` are pure-Rust (research D7) |
| No I/O, LLM, request-body, token-count, output-parse | FR-005, C-03 | no such APIs exist in the kernel; negative review |
| Validation-blind | FR-004 | kernel never inspects value types/constraints |
| Consumes (not redefines) the 001 shape | FR-027 | `pub mod generated` is the relocated codegen output, freshness-gated |
| Determinism / structural parity | FR-003, C-01 | `BTreeSet` ordering, `sha2`, no time/random |

## Error contract

`KernelError` variants: `UnknownVariant { requested }`, `ExcludedFeature { detail }`,
`Parse { detail }`, `UndefinedVariable { name }`, `Render { detail }`. The kernel returns native
`KernelError`; normalization to the common `[{field, code, message}]` shape happens in the **consumer**
(spec 003) — native error types MUST NOT leak across FFI (C-06). [FR-028]

> **Info-leakage note (security SEC-004):** `Parse`/`Render` `detail` strings may embed bound-value
> content (which can be untrusted/PII). The kernel holding this in-process is fine, but the spec-003
> normalization layer MUST scrub/avoid logging raw `detail` to prevent value content leaking into
> consumer logs. Tracked for spec 003; no kernel change required.
