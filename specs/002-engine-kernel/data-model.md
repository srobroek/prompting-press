# Phase 1 Data Model — Spec 002 (Engine kernel)

The kernel adds **behavior** over the spec-001 data shape; it introduces only a few small output/option
types of its own. The input shape (`PromptDefinition` and friends) is **consumed, not redefined**
(FR-027) — it is the generated serde struct relocated into the kernel (research D6).

## Input types (from spec 001 — consumed, not defined here)

- **`PromptDefinition`** — `{ body: String, role, name, variables: HashMap<String, VariableDecl>,
  variants: HashMap<String, Variant>, meta, metadata, output_model }`. `body` is the **default arm**
  (reserved variant name `default`). The kernel reads `body`/`variants` (render + hash + analysis) and
  `variables[*].provenance` (guard expansion). It does **not** read/interpret `meta`, `metadata`,
  `output_model`, or `variables[*]` constraint keys (those are the consumer's / opaque).
- **`Variant`** — `{ body: String, meta }`. Only `body` is load-bearing in the kernel.
- **`VariableDecl`** — the kernel uses only `provenance ∈ {trusted, untrusted, external}`. Type and
  constraint keys are validation-layer concerns (out of scope, C-03).

## Kernel-defined types

### `ResolvedVariant` (internal)
The arm chosen for a render.

| Field | Type | Notes |
|---|---|---|
| `name` | `String` | The reserved `default`, or the explicit variant key. |
| `source` | `&str` (borrow of the def) | The template source string — the exact bytes `template_hash` is computed over (FR-012). |

**Resolution rule** (FR-007..FR-011, per research D6/contradiction resolution):
- `variant = None` → `name="default"`, `source = def.body`.
- `variant = Some("default")` → same as None (reserved name → root body, FR-011).
- `variant = Some(k)` where `k ∈ variants` → `name=k`, `source = variants[k].body`.
- `variant = Some(k)` where `k ∉ variants` (and `k != "default"`) → **`KernelError::UnknownVariant{ requested: k }`** (FR-009).
- There is **no "missing default" error path**: the root `body` is always the default (see research
  §resolved-contradiction; ratified 2026-06-26 via spec refine).

### `RenderResult` (output / provenance — FR-015)
Plain data returned to the caller; no telemetry coupling.

| Field | Type | FR | Notes |
|---|---|---|---|
| `text` | `String` | FR-001 | The rendered body. |
| `name` | `String` | FR-015 | Prompt name (from `def.name`). |
| `variant` | `String` | FR-015 | Resolved variant name. |
| `template_hash` | `String` | FR-012 | lowercase-hex `SHA256(variant source)`. |
| `render_hash` | `String` | FR-013 | lowercase-hex `SHA256(text)`. |
| `guard` | `Option<String>` | FR-022 | Present only when guard expansion was opted in; the guard instruction text. **Never** concatenated into `text`. |

Invariants: no `vars_hash` field exists (FR-014). With guard opt-out, `guard = None` and `text` is
byte-identical to a plain render (FR-022, SC-005).

### `Agreement` (analysis output — FR-016..FR-019)
Per resolved variant.

| Field | Type | Notes |
|---|---|---|
| `variant` | `String` | Which arm was analysed. |
| `required_roots` | `BTreeSet<String>` | Root variable names the template references, minus local bindings and minus the env-derived globals allowlist (research D2). Sorted ⇒ deterministic. |

The kernel does **not** compare `required_roots` against declared `variables` (FR-019 — that ⊆ check is
the consumer's lint in spec 003).

### `GuardConfig` (render option — FR-022..FR-025)
Opt-in, per-render.

| Field | Type | Notes |
|---|---|---|
| `enabled` | `bool` | When false, no guard field is produced. |
| `template` | `Option<String>` | Caller override of the guard instruction text; `None` ⇒ kernel default template (FR-024). |

The guard text names the prompt's `untrusted`/`external` fields (FR-021/FR-022). Producing it never
mutates the template, values, or `text` (FR-023, FR-025). The default template is a kernel constant.

### `ProvenanceView` (exposure — FR-021)
Derived from `def.variables`; lets the consumer query tags without re-reading the shape.

| Field | Type | Notes |
|---|---|---|
| `untrusted` | `BTreeSet<String>` | Field names tagged `untrusted`. |
| `external` | `BTreeSet<String>` | Field names tagged `external`. |
| (`trusted` is the complement; not separately stored.) | | Sorted ⇒ deterministic guard text. |

### `KernelError` (structured errors — FR-028)
Enum; the consumer normalizes to `[{field, code, message}]` (FR not in kernel scope).

| Variant | Trigger | FR |
|---|---|---|
| `UnknownVariant { requested }` | Render of a non-existent variant name | FR-009 |
| `ExcludedFeature { detail }` | Template uses include/import/extends/macro/inheritance (surfaces from the parse error under feature-disabled engine) | FR-002, FR-008(edge) |
| `Parse { detail }` | Template fails to parse (syntax) | FR-028 |
| `UndefinedVariable { name }` | Strict-undefined hit at render (FR-001a) | FR-001a |
| `Render { detail }` | Other render-time failure (e.g. non-iterable in a loop) | FR-028 |

(`ExcludedFeature` vs `Parse`: if MiniJinja's disabled-feature error is distinguishable by `ErrorKind`,
the kernel labels it `ExcludedFeature`; otherwise it falls back to `Parse` — both loud, research D4.)

## State / lifecycle

No persistent state. Each kernel operation is pure over its inputs:
- **Analyse**: `(&PromptDefinition, variant) → Agreement` — never renders, never mutates (FR-018).
- **Get source**: `(&PromptDefinition, variant) → &str` — the unrendered arm (FR-006).
- **Render**: `(&PromptDefinition, variant, Value, GuardConfig) → RenderResult | KernelError`.
- **Provenance view**: `(&PromptDefinition) → ProvenanceView`.

All four are referentially transparent: no I/O, no globals, no time/random (Principle III / C-03).
