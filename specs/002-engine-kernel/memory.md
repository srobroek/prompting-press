# Feature Memory — Spec 002 (Engine kernel)

Feature-local working notes and open questions for the `prompting-press-core` kernel. Durable
decisions live in the governance layer (constitution + roadmap C-01…C-10); this file is transient.

## What 002 owns (the single behavior site)

`prompting-press-core` is the ONE place rendering, agreement analysis, variant resolution, and
hashing are implemented (C-01). It is binding-agnostic and validation-blind (C-02/C-03): inputs are
already-validated values + a `PromptDefinition` (the generated Rust shape from 001). Output is
rendered text + provenance data. No FFI, no I/O, no LLM/request-body/token/output logic.

## Kernel input contract (from 001's generated shape)

- `PromptDefinition { body, role, name, variables, variants, meta, metadata, output_model }`.
- `body` = the **default arm's** template source (surfaced as reserved variant name `default`,
  `is_default=true`). `variants: HashMap<String, Variant>` where `Variant { body, meta }`.
- `variables: HashMap<String, VariableDecl>` where `VariableDecl { type, provenance (trusted|
  untrusted|external), + optional JSON-Schema constraints }`. The kernel reads `provenance` for the
  guard expansion (data, not enforcement).
- Reserved-`default` rule is NOT in the type (typify stripped `propertyNames`) — kernel logic must
  enforce variant-naming/default semantics itself.

## Clarified (Session 2026-06-26)

- **Strict undefined** (FR-001a): undefined var at render = loud error, not empty string. Optional
  refs use explicit `is defined`.
- **Agreement-check granularity** (FR-016): **per resolved variant** (one template source at a time);
  consumer aggregates. Likely a `HashSet<String>` per variant.
- **Guard expansion** (FR-022..024): guard text is a **separate, caller-configurable field on the
  render result** with a provided default — NOT concatenated into the body. Additive, non-mutating.
- **Excluded features rejected loudly** (FR-002): kernel actively rejects include/import/extends/
  macros/inheritance (not incidental engine behavior).

## Open questions (resolve in plan)

1. **MiniJinja version pin + allowlist** (roadmap Q3): pin a concrete MiniJinja version; re-confirm
   `Template::undeclared_variables(nested=false)` stable-API soundness and derive the globals/filters
   allowlist from THAT version (2.21 was the reference). Allowlist contents (`range`, `namespace`,
   `dict`, … + default filters). Allowlist is a fixed kernel constant (no new pluggable seam — C-08),
   not configurable.
2. **Strict-undefined mechanism**: MiniJinja `UndefinedBehavior::Strict` is the obvious vehicle —
   confirm it errors on bare-print AND that `is defined` still works for optional refs under Strict.
3. **Excluded-feature rejection mechanism**: parse-time detection vs. configuring MiniJinja with no
   loader / disabled features. Whatever the mechanism, it must fail loud AND keep them out of the
   agreement analysis (C-04 soundness).
4. **"values" wire type at the kernel boundary**: serde-compatible value map (e.g. `serde_json::Value`
   / MiniJinja `Value`) — settled in plan; must round-trip the FFI conformance cases later (007).
5. **Error taxonomy**: kernel error enum variants — unknown-variant, multi-variant-without-default,
   render error, parse error, excluded-feature-used, strict-undefined. (Normalization to
   `[{field,code,message}]` is the consumer's job; the kernel needs a structured enum.)
6. **Guard default wording**: exact default guard-template text/format (the field shape is decided;
   the default string is a plan detail).

## Non-negotiables to keep green

- Kernel stays FFI-free — spec-001 `scripts/ci/check-ffi.sh` gate. New deps (MiniJinja, sha2) must
  pull no `pyo3`/`napi`.
- Agreement check + provenance lint are **pure analysis, never mutate** (C-04/C-09).
- No `vars_hash`; provenance is data on the return value (C-05).
- Don't hand-edit generated files; the kernel *consumes* the generated shape, doesn't regenerate it.
