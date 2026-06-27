# Data Model — TypeScript binding (`prompting-press-node`)

Phase 1. The binding's types: the `#[napi]` classes the Rust crate exposes, and the TS-facing surface the
`packages/typescript` facade adds. All wrap the shared-core / consumer types 1:1 — no new domain logic.

## napi-exposed classes (Rust `crates/prompting-press-node/src/`)

| Class | Wraps | Surface (camelCase on JS side) | Notes |
|---|---|---|---|
| `Registry` | `prompting_press::Registry` | `new()`, `loadYaml(text)`, `loadJson(text)`, `insert(def)` | `&mut self` loaders; absent name at render/check → `UnknownPromptError`. Marshals YAML/JSON **text** to the consumer loader (Q3). |
| `RenderResult` | kernel `RenderResult` | getters: `text`, `name`, `variant`, `templateHash`, `renderHash`, `guard?` | read-only; surfaced 1:1 from the kernel. `guard` separate from `text` (system-prompt addendum doctrine, carried from 004). |
| `CheckReport` | `prompting_press::CheckReport` | `findings: Finding[]`, `passed(): boolean`, `isEmpty()` | deterministic order preserved (BTreeMap/BTreeSet). |
| `Finding` | `prompting_press::Finding` | getters: `prompt`, `variant?`, `kind`, `detail` | `kind` ∈ `undeclared_variable` / `untrusted_without_guard` / `reserved_variant_name` / `analysis_error`. |
| `Composition` | binding-owned `Vec<Entry>` | `new()`, static `fromMessages(entries)`, `append(name, vars, variant?)`, `resolve(reg): Message[]` | NOT the consumer's generic `Composition<V>` (no garde type) — binding-owned ordered marshaled entries; no `.chain()`. |
| `Message` | binding-owned | getters: `role`, `text` | composition output element. |

Free functions: `render(reg, name, value, variant?, guard?) -> RenderResult`,
`getSource(reg, name, variant?) -> string`, `check(reg) -> CheckReport`.

> Validation is **not** in the napi layer — the napi `render`/`Composition.append` receive an
> **already-Zod-validated** plain JS value. The Zod `safeParse` happens in the TS facade (D4), because Zod
> is a TS library.

## TS facade surface (`packages/typescript/src/index.ts`)

| Symbol | Kind | Role |
|---|---|---|
| `render(reg, name, schema, data, opts?)` | fn | `schema.safeParse(data)` → on success delegate to the napi `render`; on failure throw `PromptValidationError`. (Also accepts already-typed data with no schema — Q4.) |
| `getSource`, `check`, `Registry`, `RenderResult`, `CheckReport`, `Finding`, `Composition`, `Message` | re-export | from the napi addon. |
| `PromptingPressError` | `class extends Error` | base; `readonly errors: FieldError[]`. |
| `PromptValidationError` / `PromptRenderError` / `UnknownPromptError` / `LoadError` | `class extends PromptingPressError` | 1:1 with the Rust `ConsumerError` variants; `instanceof`-branchable. |
| `FieldError` | type | `{ field: string; code: string; message: string }`. |
| `PromptDefinition` | type (re-export) | from `./generated/prompt-definition` (codegen'd, C-07). |

## Key types (carried, not redefined)

- **FieldError** `{field, code, message}` — the single cross-binding error row. `code` from the closed
  vocabulary: `validation`, `unknown_prompt`, `unknown_variant`, `undefined_variable`, `parse`, `render`,
  `excluded_feature`, `load`.
- **Prompt definition** — the spec-001 shape (`name`, `role`, `body`, `variables` with provenance tags,
  `variants`, `meta`/`metadata`, `outputModel`). TS shape codegen'd from the JSON Schema; consumed, not
  redefined.
- **Provenance** — `templateHash`/`renderHash` (SHA-256 hex), byte-identical to the Python binding + Rust
  consumer for the same logical prompt+inputs (structural; Principle I). No `varsHash`.

## Marshaling rules (FR-003a / Q6)

- `undefined` / absent object field → **field not present** (kernel strict-undefined fires if referenced).
- explicit `null` → JSON `null`.
- `number` → i64/f64 by JS value; `bigint` → lossless integer (test-pinned); nested objects/arrays →
  recursive; dates → stringified consistently with the kernel serde model. Matched to the Python binding's
  `None`/absent handling for spec-006 parity.
