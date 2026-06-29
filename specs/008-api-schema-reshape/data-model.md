# Phase 1 Data Model — Pre-publish API & schema reshape

## 1. JSON Schema deltas (`schemas/jsonschema/prompt-definition.schema.json`)

The schema is the single source of truth (C-07); all three shapes regenerate from these deltas.

### 1a. Rename `provenance` → `origin` (per-variable trust tag)

Inside `$defs.VariableDecl`:
- `properties.provenance` → **`properties.origin`** (key renamed; the `enum [trusted, untrusted, external]`
  and description are unchanged except the field name).
- `required: ["type", "provenance"]` → **`required: ["type", "origin"]`**.

**Scope boundary (FR-002):** this is the ONLY `provenance`→`origin` change in the schema. The render-result
provenance (the `template_hash`/`render_hash` on the render return value) is a *runtime output*, not a schema
field, and is **not** renamed.

### 1b. Add `validation_required` (per-variable, optional)

Inside `$defs.VariableDecl.properties`, add:
```json
"validation_required": {
  "type": "boolean",
  "default": false,
  "description": "When true, a validator covering this variable MUST be supplied when the Prompt is constructed. Orthogonal to `origin`. Declarative metadata; enforcement is per-language (TS/Python throw at construction if uncovered; Rust guarantees coverage at compile time). The kernel never reads this field (validation-blind)."
}
```
- Optional (not in `required`), defaults `false` → existing documents stay valid.
- `additionalProperties: false` on `VariableDecl` already holds; the new key is explicitly allowed by being
  declared.

### 1c. Codegen impact per language

| Shape | Tool | Effect of 1a+1b |
|-------|------|-----------------|
| Rust struct | `cargo-typify@0.7.0` | `VariableDeclProvenance` enum → regenerated as the `origin` enum (typify names it from the field, so the generated type identifier changes); new `validation_required: Option<bool>` field. The existing `jq 'del(.properties.variants.propertyNames)'` strip is unchanged. |
| Python Pydantic | `datamodel-code-generator` | field `provenance` → `origin`; new optional `validation_required: bool = False`. `--target-python-version 3.12` (R5). |
| TS Zod schema | `json-schema-to-zod@2.8.1` | replaces the `interface`; `origin` is a `z.enum([...])`; `validation_required` is `z.boolean().default(false)`. `z.infer` gives the static type. |

## 2. The `Prompt` object (new — all three bindings)

A first-class **immutable** value object (Facade over the codegen'd shape). Same capability everywhere; native
idiom per language (Principle VI).

### 2a. Fields (read-only accessors; NO setters)

Mirrors the generated shape: `name`, `role`, `body`, `variables` (each: `type`, `origin`,
`validation_required?`, + the carried JSON-Schema constraint metadata), `variants` (name → `{body, meta?}`),
`output_model?`, `metadata?`, `meta?`. Plus the binding-held **validator(s)** (TS/Python: the bound runtime
validator object; Rust: carried at the type level via the generic `V`).

### 2b. Construction (validating; never panics)

| Path | Rust | Python | TypeScript |
|------|------|--------|------------|
| From shape object (PRIMARY) | `Prompt::new(shape, validators?) -> Result<Prompt, Error>` | `Prompt(shape, *, validators=...)` raises on invalid | `new Prompt(shape, validators?)` **throws** on invalid |
| From YAML text | `Prompt::from_yaml(text, validators?) -> Result` | `Prompt.from_yaml(text, *, validators=...)` | `Prompt.fromYaml(text, validators?)` |
| From JSON text | `Prompt::from_json(text, …)` | `Prompt.from_json(...)` | `Prompt.fromJson(...)` |
| From TOML text | `Prompt::from_toml(text, …)` (`toml@1.1.2`) | `Prompt.from_toml(...)` (`tomllib`) | `Prompt.fromToml(...)` (`smol-toml@1.7.0`) |

**Construction invariants (every path, FR-011/020/024):**
1. **Shape-validate** the document/object (serde shape layer / Pydantic / Zod parse) → structured load error on
   malformed input.
2. **Parse the template** + run the kernel's `required_roots`; an excluded feature / syntax error → construction
   failure (R7/Q4). Referenced vars ⊄ declared → construction failure (agreement, Principle IV).
3. **Validator coverage:** every variable with `validation_required: true` must be covered by a supplied
   validator. TS/Python introspect (`schema.shape` / Pydantic `model_fields`) and **throw/raise** if not; Rust
   enforces at compile time (the generic `V` is the coverage).
4. On success → an **immutable** `Prompt`. Errors normalized to `[{field, code, message}]`; no native error
   type crosses FFI (C-06).

### 2c. Operations (single-prompt, moved onto the object)

- **`render(...)`** — validate values (bound validator) → kernel render → `RenderResult` (`text`,
  `template_hash`, `render_hash`, `guard?`). Byte-identical to the old `render(reg, name, …)` (FR-016).
  Optional/config tail (`variant`, `guard`) per the options-object/keyword/Rust-`Option` shape (C-11).
- **`getSource({variant?})`** — the unrendered template source.
- **`check()`** — pure analysis; post-reshape its hard arms are construction-enforced, so it returns only the
  origin/guard advisory finding (a prompt with `untrusted`/`external` vars and no guard is valid-but-flagged).
- **`with(overlay) -> Result<Prompt>`** — the SOLE mutator. Shallow-replace per top-level field; `name`
  overlayable; validators carry forward (TS/Python) with optional override, or are re-named via the generic `V`
  (Rust); re-validates the merged whole through the same construction path; original untouched (R6).

### 2d. Removed

- **`Registry`** and all name-keyed lookup free functions (`render(reg, name, …)`, `get_source(reg, …)`,
  `check(reg)`) — removed from every binding's public surface (FR-019). The dual-input loaders fold into the
  `from_yaml`/`from_json`/`from_toml` factories.
- The TS `isSchema()` duck-typing (the named `schema`/validators argument removes the ambiguity).

## 3. Composition (reshaped)

`Composition` aggregates **`Prompt` objects** (each with its variables/variant), resolving to an ordered
`[{role, text}]`. No registry; no name resolution. Native construction sugar per language (array/list of
entries). Eager-validates each entry; a failing entry → structured error identifying it (no partial output).

## 4. Entity relationships

```
Prompt (immutable)
 ├── shape: generated PromptDefinition (name, role, body, variables[], variants{}, meta, …)
 │     └── VariableDecl: { type, origin (renamed), validation_required?, …constraints }
 ├── validator(s): native (Zod/Pydantic bound; garde = type-level in Rust)
 ├── render() → RenderResult { text, template_hash, render_hash, guard? }   [kernel, unchanged]
 ├── getSource() → string
 ├── check() → CheckReport { findings: [origin/guard advisory] }            [pure analysis]
 └── with(overlay) → Result<Prompt>   [shallow-replace, re-validate merged, original untouched]

Composition  →  [Prompt, vars, variant?]*  →  resolve()  →  [{role, text}]*
```

## 5. State & invariants (testable)

- **Immutability:** no public setter exists on `Prompt` in any binding; after `with`, the source's accessors
  return original values (SC-004).
- **Construction totality:** an analyzable-and-covered prompt constructs; any decidable violation (shape /
  parse / agreement / missing required validator) fails construction with a structured error (SC-005/009).
- **Hash parity:** `render` output + hashes are byte-identical to the pre-reshape path and across bindings
  (SC-003) — proven structurally by the unchanged kernel + the conformance corpus.
- **Field name:** zero `provenance` occurrences remain for the per-variable tag; render-result provenance
  retained (SC-002).
