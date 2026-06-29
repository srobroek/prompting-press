# Memory Synthesis

_(Direct-read fallback: `speckit_memory_*` MCP tools + `.spec-kit-memory/` SQLite cache absent this session.
Sources read: `.specify/memory/{constitution.md@v1.2.0, roadmap.md, DECISIONS.md}`, `docs/memory/{INDEX, A1, D1}`,
the spec, and the feature `memory.md`. Token banner N/A — no cache.)_

## Current Scope

Spec 008 — pre-publish reshape of the public contract, ~174 files, schema-as-source-of-truth. Three bundles,
one object-model decision: (1) per-variable `provenance`→`origin` rename; (2) schema-fixture move into
`tests/`; (3) prompt-as-object redesign across Rust/Python/TS (immutable `Prompt`, validating constructor,
`with` sole mutator, `fromYaml/Json/Toml`, Registry dropped, per-variable `validation_required` validators
bound at construction). Kernel UNCHANGED (Principle I). Affected modules: `schemas/`, `crates/prompting-press-core`
(generated shape + `provenance.rs`→origin), `crates/prompting-press` (consumer: registry→Prompt, check, render),
`crates/prompting-press-py`, `crates/prompting-press-node` + `packages/{python,typescript}`, `conformance/`,
all fixtures + docs.

## Relevant Decisions

- **Constitution v1.2.0 — Principle VI expanded** (Reason: governs the validator model 008 implements; Status:
  active — amended THIS session; Source: `.specify/memory/DECISIONS.md` + constitution). Validators MAY bind at
  construction; per-variable `validation_required` orthogonal to `origin`; enforcement asymmetric (TS/Python
  throw at construction, Rust compile-time/structural + declarative). Kernel validation-blind.
- **C-11 / v1.1.0 — options-object call shape** (Reason: `render`/`getSource`/composition move onto `Prompt`
  and must keep the options-object tail; Status: active; Source: DECISIONS.md). Rust single `Option<T>` OK.
- **D1 — cross-binding parity via canonical serialized form** (Reason: conformance re-runs over moved fixtures;
  Status: active; Source: `docs/memory/decisions/2026-06-28-canonical-serialized-form-marshaling.md`). date/
  decimal pinned by serialized string, NOT native objects. Don't "fix" a runner to build native objects.

## Active Architecture Constraints

- **A1 — three validity layers are NOT equivalent** (Reason: `fromYaml/Json/Toml` = serde shape layer; the
  validating constructor adds the decidable agreement check; `check()` is the advisory residue; Source:
  `docs/memory/architecture/2026-06-28-loader-vs-schema-validation-layers.md`). `variant-named-default` is
  schema-invalid but loader-accepted → conformance manifest excludes it from the loader round-trip set; the
  fixture move MUST preserve that note.
- **Principle I — kernel is the single source of render/agreement/hash behavior** (Reason: 008 must not touch
  it; cross-binding parity is structural; Source: constitution). The generated `PromptDefinition` lives in
  `-core/src/generated/`; rename cascades through it via codegen, not hand-edits.
- **Principle II / III — FFI isolation + minimal boundary** (Reason: no `pyo3`/`napi` in core or consumer; no
  I/O / LLM / request-body assembly may sneak in with the reshape; Source: constitution + `ci:check-ffi`).
- **Principle VII / C-07 — JSON Schema single source; 3 shapes codegen'd** (Reason: the rename + the new
  `validation_required` field must regenerate, never hand-mirror; codegen-freshness gate; Source: constitution).
- **Principle IV — sound agreement check** (Reason: moves onto construction; un-analyzable bodies fail at
  construction; Source: constitution + `agreement.rs` parse-first short-circuit).

## Accepted Deviations

- None on record that relax 008. (The Rust compile-time vs TS/Python runtime asymmetry is now *codified* in
  Principle VI, so it is the standard, not a deviation.)

## Relevant Security Constraints

- **SEC-004 scrub** (Reason: the reshape re-routes errors through new construction/`with` paths; the scrub must
  hold — `KernelError` through the consumer `From` scrubber first; Pydantic mapper copies `msg`/`loc` only,
  never `input`/`ctx`; Source: spec-004 memory + spec 001 SEC-004). Native error types MUST NOT leak across FFI.
- **Origin tag is declarative, not enforcement (C-09)** (Reason: rename must not imply runtime trust
  enforcement; the guard stays advisory; Source: `provenance.rs` docs + roadmap C-09).

## Related Historical Lessons

- **Mid-cycle amendment → sweep all docs** (spec-005 IMP-001): the v1.2.0 amendment this session instantly
  stales any doc describing the old "validators at render only" model; reconcile during implementation, not
  just at the sync gate.
- **Subagent tool-channel glitch**: run read-only audit/verify gates main-thread or with an integrity-check
  preamble; discard `tool_uses:0`; re-verify load-bearing findings against the code.
- **maturin from `packages/python/`**, napi from package dir; commit via `git commit -F`; 1Password must run
  for signing; `dgit push`; `rm` blocked (use `git rm`/`git mv`).

## Conflict Warnings

- **No HARD conflicts.** The resolved 008 object model is consistent with the constitution (incl. the v1.2.0
  amendment authored to support it) and with A1/D1. The Registry drop is C-08-consistent (Scope Discipline).
- **Soft watch:** the `with(overlay)` + bound-validator carry-forward semantics are underspecified (logged as
  open design item A8-2); resolve explicitly in the plan's contracts, don't let it drift into implementation.

## Retrieval Notes

- Index entries considered: `docs/memory/INDEX.md` (2 entries: A1, D1) — both included. Governance layer read
  directly (constitution v1.2.0, roadmap 008 entry, DECISIONS.md top 2 amendments). Budget: well under limits
  (2 decisions, 5 constraints, 0 deviations, 2 security, 3 lessons). Full-memory-read not required.
