# Phase 1 Data Model — Python binding (`prompting-press-py`)

The binding defines **no new domain data** — it re-surfaces the spec-002/003 model in Python idiom.
Below: the Python-facing types (what a caller imports/sees) and the Rust-side `#[pyclass]`/marshaling
types that back them. Render/agreement/variant/hash logic is **not** here — it lives in the kernel
(Principle I / C-02).

## Python-facing surface (what the caller sees)

### Typed Vars model (application-authored)
- A caller-defined **Pydantic v2 `BaseModel`** subclass with field validators (`@field_validator`,
  constrained types). Field names must agree with the prompt's declared `variables` (the three-sets
  invariant — caller responsibility; a mismatch surfaces as an `undefined_variable` exception, never a
  silent empty render).
- **Not** codegen'd; authored per application. Passed to `render`/`from_messages` alongside the prompt
  name. The binding owns validation (clarified Q1): it calls `model_validate(data)` (or re-validates a
  passed instance) before any templating.

### `PromptDefinition` (generated — `prompting_press.generated`)
- The Pydantic shape **code-generated from the JSON Schema** (present; freshness-gated). Fields: `name`,
  `role` (`system`/`user`/`assistant`), `body`, `variables: dict[str, VariableDecl]` (each with a
  `provenance` tag — the authoritative declared set for the agreement check), `variants: dict[str,
  Variant]`, `output_model` (opaque ref), `meta`/`metadata` (opaque maps). Used for the
  constructed-object input path; never hand-edited.

### `Registry`  (`#[pyclass]`)
- A library-owned map of prompt **name → loaded definition**, wrapping the consumer's `Registry`
  (`BTreeMap`-backed → deterministic `check` order). Methods:
  - `load_yaml(text: str) -> None` — marshal text to the consumer's YAML loader.
  - `load_json(text: str) -> None` — marshal text to the consumer's JSON loader.
  - `insert(definition: PromptDefinition) -> None` — `model_dump_json` → consumer `load_json`.
  - A name absent at render/check raises `UnknownPromptError` (never a crash).

### `RenderResult`  (`#[pyclass]`, read-only getters)
- `text: str`, `name: str`, `variant: str`, `template_hash: str` (lowercase SHA-256 hex),
  `render_hash: str`, `guard: str | None`. Surfaced 1:1 from the kernel's `RenderResult`; not redefined.

### `Message`  (`#[pyclass]` or plain) — composition output
- `role: str`, `text: str`. A `Composition` resolves to an ordered `list[Message]`.

### `Composition`  (`#[pyclass]`)
- `Composition()` + `append(name, vars, variant=None)`, or the constructor
  `Composition.from_messages([(name, vars, variant?), ...])`. `resolve(registry) -> list[Message]`.
  Validates **eagerly at append/from_messages** (option (a), spec-003 parity); `resolve` never returns
  a partial result as success. No `.chain()` (FR-013).

### `CheckReport` / `Finding`  (`#[pyclass]`, read-only)
- `CheckReport.findings: list[Finding]`; `CheckReport.passed() -> bool`.
- `Finding`: `prompt: str`, `variant: str | None`, `kind: str`, `detail: str`. `kind` ∈
  `{undeclared_variable, untrusted_without_guard, reserved_variant_name, analysis_error}` (the spec-003
  `FindingKind` discriminants, stringified). Deterministic order preserved from the consumer.

### Exception hierarchy (`prompting_press` namespace)
- **`PromptingPressError(Exception)`** — base; `.errors: list[FieldError]` where each is
  `{field: str, code: str, message: str}`.
- **`PromptValidationError`** — Pydantic/garde-class validation (`code="validation"`); rows name every
  offending field.
- **`PromptRenderError`** — kernel failures (`code` ∈ `unknown_variant`, `undefined_variable`, `parse`,
  `render`, `excluded_feature`); `parse`/`render`/`excluded_feature` messages are **scrubbed** (SEC-004).
- **`UnknownPromptError`** — `code="unknown_prompt"`; carries the requested name.
- **`LoadError`** — `code="load"`; malformed YAML/JSON or shape violation; nothing partially loaded.
- The `code` vocabulary is identical to the Rust consumer's `error::code` (cross-binding contract).

## Rust-side (binding internals — `crates/prompting-press-py/src/`)

| Module | Type / fn | Role |
|---|---|---|
| `lib.rs` | `#[pymodule] prompting_press_py` | register classes + functions + exceptions |
| `registry.rs` | `#[pyclass] Registry(prompting_press::Registry)` | load_yaml/load_json/insert |
| `render.rs` | `#[pyfn] render`, `#[pyfn] get_source` | validate (Q1) → marshal → `prompting_press_core::render` (kernel-direct, E1); `get_source` reuses the consumer; `KernelError` via consumer scrubber |
| `marshal.rs` | `to_kernel_value(obj) -> minijinja::Value` | `depythonize` bridge (D2); lossless None/int/float/nested |
| `check.rs` | `#[pyfn] check`, `#[pyclass] CheckReport`, `Finding` | over `prompting_press::check`; deterministic order |
| `compose.rs` | `#[pyclass] Composition`, `Message` | binding-owned ordered entries (NOT the consumer's generic `Composition<V>`, E1); eager-validate append; resolve loop calls kernel-direct render |
| `error.rs` | `create_exception!` base + subtypes; `From<ConsumerError>` translation; Pydantic `ValidationError` map | normalize → raise; SEC-004 scrub preserved |

## Validation & invariants

- **Validate-then-render** (FR-002): `model_validate` before any kernel call; on failure raise
  `PromptValidationError`, never reach the kernel.
- **Only validated values cross FFI** (FR-003): the binding marshals already-validated data.
- **No engine logic** (FR-011/C-02): every render/agreement/variant/hash is a consumer/kernel FFI call.
- **Native types never leak** (FR-014/C-06): Pydantic `ValidationError` + Rust errors → the exception
  hierarchy; raw kernel detail scrubbed (FR-015).
- **`check` is pure** (FR-019): no mutation, no render, deterministic order.
- **Generated shape is codegen'd** (FR-008/024): never hand-edited; freshness-gated.
