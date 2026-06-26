# Feature Memory — Spec 003 (Rust consumer `prompting-press`)

Feature-local working notes + open questions for the Rust consumer crate. Durable decisions live in
the governance layer (constitution + roadmap C-01…C-10); this file is transient.

## What 003 owns (the consumer layer the kernel deliberately omits)

`prompting-press` is the public, idiomatic Rust API over the FFI-free kernel. It adds exactly the
things the kernel left out (because they're language-native, not shared-core): typed-Vars validation,
the dual-input loader, the `check(registry)` lint, error normalization, composition sugar, and the
token-count hook interface. It WRAPS the kernel's render/agreement/variant/hash — never reimplements.

## Kernel API it wraps (as-built, spec 002)

- `prompting_press_core::render(&PromptDefinition, variant: Option<&str>, values: minijinja::Value, &GuardConfig) -> Result<RenderResult, KernelError>`
- `get_source(&def, variant) -> Result<&str, KernelError>`
- `required_roots(&def, variant) -> Result<Agreement, KernelError>` — per-variant referenced-roots set.
  **003 owns** the `referenced ⊆ declared` comparison vs the garde Vars fields (kernel only returns the set).
- `provenance_view(&def) -> ProvenanceView { untrusted, external }` — 003's provenance lint uses this.
- `RenderResult { text, name, variant, template_hash, render_hash, guard: Option<String> }`,
  `GuardConfig { enabled, template }` (has `Default`), closed `KernelError` (5 variants).
- Generated `PromptDefinition` shape lives in the kernel; consumer re-exports it (already wired in the
  spec-001 stub `crates/prompting-press/src/lib.rs`).

## Clarified (Session 2026-06-26)

- **Declared-vars authority** (FR-017): the prompt definition's **`variables` block** — NOT the garde
  struct. `check()` is pure data, CI-portable, no Rust-type introspection.
- **Registry** (FR-008a): a library-owned **map name → `PromptDefinition`**; render resolves against
  it; `check(registry)` lints over it.
- **Vars binding** (FR-009): caller passes **`render(prompt, vars)`** — no per-prompt type
  registration.
- **Vars→kernel bridge** (FR-003a): **serialize the validated struct** into the kernel value type
  (serde+garde pairing); caller doesn't hand-build a map.

## Open questions (resolve in plan)

1. **garde version + API** (verify-at-spec-time): roadmap says garde 0.23 — confirm the current
   version and the `Validate` derive / `#[garde(custom(...))]` / `Report` API at plan time. Does garde
   0.x expose what we need (custom field validators, a structured `Report` we can normalize)?
2. **YAML parser choice** for the dual-input loader (serde_yaml is archived — need a maintained,
   pure-Rust successor, e.g. `serde_yaml_ng` or `serde_norway`). Verify maintained + FFI-free at plan.
3. **Composition API**: the ordered `Vec` of `(prompt-ref, vars)` → `[{role, text}]`. The `append_*`
   methods + the resolved message type (`{role, text}` per the kernel's role metadata). No `.chain()`.
4. **Error-normalization Rust type**: `Vec<FieldError { field, code, message }>`; map garde `Report`
   paths → `field`, each `KernelError` variant → a `{field, code, message}` row. Scrub bound-value
   content from `Parse`/`Render` detail (SEC-004).
5. **`count_tokens` hook signature**: `Fn(text: &str, model: &str) -> usize`? trait vs boxed closure,
   and where it attaches (registry/render call). C-03 — hook only, no built-in.
6. **The kernel value type for the serialize bridge**: kernel `render` takes `minijinja::Value`;
   confirm `Value::from_serialize` (or equivalent) is the bridge from the validated struct.

## Non-negotiables to keep green

- Consumer stays FFI-free — `ci:check-ffi` covers `prompting-press`. New deps (garde, a YAML crate)
  must pull no pyo3/napi.
- NO rendering/agreement/variant/hashing logic here — wrap the kernel (C-01).
- Validation lives here, kernel stays validation-blind. garde `Report` / `KernelError` never leak.
- `check(registry)` + validators are PURE (no mutation). Token-count is a hook (no built-in).
- Don't redefine the generated `PromptDefinition` — re-export from the kernel.
