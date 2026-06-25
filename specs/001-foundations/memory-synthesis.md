# Memory Synthesis

> Generated markdown-first (MCP `speckit-memory-hub` not configured). Durable memory
> (`docs/memory/decisions|bugs|architecture/`) is freshly initialized and empty, so this synthesis
> draws on the **governance layer** (constitution v1.0.0 Principles I–VII + roadmap decisions C-01…C-09) and
> `docs/memory/PROJECT_CONTEXT.md`. Phase: **Plan** → prioritizes boundary definitions, module
> ownership, and architectural-drift risks.

## Current Scope

Spec 001 — Foundations. The structural spine: crate/package layout (engine kernel + Rust/Python/TS
consumer-or-binding crates + reserved Go), the prompt-definition **JSON Schema** (single source of
truth), the **schema→shape codegen** pipeline, and **CI guardrails** enforcing the constitution.
**No rendering, validation, or engine behavior** — affected modules are workspace structure,
`schemas/jsonschema/`, the codegen pipeline, and CI. Roadmap Q1 (per-language codegen tooling) must
be resolved in the plan.

## Relevant Decisions

- **C-01 Shared core / structural parity** (Status: active, Source: constitution): one Rust engine
  bound into each language. Shapes the crate layout 001 must create.
- **C-02 FFI isolation** (Status: active, Source: constitution): `pyo3`/`napi` only in binding
  crates, never in `prompting-press-core` / `prompting-press`. **001 must add the CI guardrail that
  enforces this** (FR-018).
- **C-07 JSON Schema single source of truth** (Status: active, Source: constitution): per-language
  shapes codegen'd from one schema. **001 authors the schema + codegen + freshness CI** (FR-008..019).
- **C-08 Scope discipline** (Status: active, Source: constitution): no speculative pluggable
  interfaces. 001 ships one concrete path; introduces no seams.
- **Generate-then-extend Vars model** (Status: active, Source: spec 001 Clarifications): the schema's
  `variables` block carries name + type + provenance + JSON-Schema constraints, rich enough to
  generate typed Vars later. 001 must make the schema this expressive (FR-010a).

## Active Architecture Constraints

- **Minimal boundary** (Source: constitution Principle III / PROJECT_CONTEXT): no I/O, no LLM calls,
  no request-body assembly, no token counting, no output parsing. 001 introduces none of these
  (FR-022); the negative-scope review (SC-007) verifies it.
- **Crate dependency direction** (Source: C-01/C-02): kernel ← Rust consumer ← bindings; kernel
  depends on neither binding nor FFI. The layout (FR-001..003) must encode this and CI must guard it.
- **Codegen artifacts committed + freshness-gated** (Source: C-07): generated shapes are checked in,
  marked generated, segregated from hand-written code, and CI fails on staleness (FR-016, FR-019).

## Accepted Deviations

- None. (No `accepted-deviation` entries in durable memory.)

## Relevant Security Constraints

- None specific to 001 (no untrusted input is processed yet — provenance tags are only *declared* in
  the schema this spec, plumbed/enforced in later specs). The 3-way provenance tag
  (`trusted|untrusted|external`) shape is fixed here (FR-010a) so the later guard/lint can rely on it.

## Related Historical Lessons

- None (fresh project; no `bugs/` or prior-feature lessons). Carry-forward from the grilling session
  is already encoded in the constitution and `docs/research/feature-scope.md`.

## Conflict Warnings

- **No hard conflicts.** Spec 001 is fully consistent with the constitution — it *is* the spec that
  makes C-01/C-02/C-07 executable. One thing to watch in the plan (soft): the codegen tooling choice
  (Q1) must produce **deterministic** output (FR-015) and support all three languages; if a chosen
  generator can't guarantee byte-stable output, the freshness CI gate (FR-019) is undermined —
  resolve tooling against this constraint, not just availability.

## Retrieval Notes

- Index entries considered: governance layer (constitution, roadmap ledger) + PROJECT_CONTEXT;
  durable `decisions/bugs/architecture/` empty (fresh init). Within budget (5 decisions, 3
  architecture, 0 security/bugs/worklog). No feature `memory.md` present. MCP unavailable →
  markdown-first; SQLite cache not built. Full-memory read not required.
