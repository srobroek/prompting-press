# Implementation Plan: Foundations — Layout, Schema, Codegen, CI Guardrails

**Branch**: `001-foundations` | **Date**: 2026-06-25 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-foundations/spec.md`; memory synthesis from
`specs/001-foundations/memory-synthesis.md`; governing decisions C-01/C-02/C-07/C-08
(`.specify/memory/constitution.md` v1.0.0).

## Summary

Build the structural spine for Prompting Press: the load-bearing crate/package layout, the
prompt-definition **JSON Schema** (single source of truth), the schema→shape **codegen** pipeline
(Pydantic v2 / TS / Rust serde), and the **CI guardrails** enforcing FFI-isolation (C-02) and
codegen-freshness (C-07). No engine, rendering, validation, or binding logic — purely the governed
skeleton. Codegen tooling resolved in Phase 0 (best-in-class per language; determinism is the
make-or-break property gating the freshness check).

## Technical Context

**Language/Version**: Rust (workspace; toolchain pinned via `rust-toolchain.toml` for codegen
determinism) · Python 3.10+ (Pydantic v2 target) · TypeScript/Node (TS target). Go: reserved only.

**Primary Dependencies** (tooling, not runtime — 001 ships no runtime logic): `datamodel-code-generator`
0.65.1, `json-schema-to-typescript` 15.0.4, `cargo-typify` 0.7.0; Cargo workspace + moon
orchestration; maturin (Py packaging) / napi-rs CLI (TS packaging) — wired, not exercised.

**Storage**: N/A (the library does no I/O — Principle III).

**Testing**: schema meta-validation + accept/reject fixtures (FR-013); codegen determinism (re-run
no-diff); CI-gate behavior tests (forbidden-dep fails; stale-codegen fails). `cargo test` for stub
crates' build.

**Target Platform**: developer + CI workstations (Linux/macOS); published artifacts target
crates.io/PyPI/npm later (spec 007, not here).

**Project Type**: polyglot library — shared Rust core + per-language consumer/binding crates.

**Performance Goals**: N/A for 001 (no runtime path). Codegen + CI gates should run in seconds.

**Constraints**: codegen output MUST be byte-deterministic (freshness gate, FR-019); FFI deps MUST
be absent from kernel/consumer crates (FR-018); no rendering/validation/IO introduced (FR-021/022).

**Scale/Scope**: 4 active crates + 2 package wrappers + 1 reserved placeholder; 1 schema; 3 generated
shapes; 2 CI gates; ~8 validation fixtures.

## Constitution Check

*GATE: must pass before Phase 0 (done) and re-checked after Phase 1 design (below).*

| Principle | Relevance to 001 | Status |
|---|---|---|
| **I — Shared core / structural parity** | Layout creates the single Rust kernel + consumer layers; no per-language reimpl. | PASS — layout encodes it (FR-001..004). |
| **II — FFI isolation** | `pyo3`/`napi` only in binding crates; kernel/consumer FFI-free. | PASS — FR-002/003 + the FFI-isolation CI gate (FR-018, D3) *enforces* it. |
| **III — Minimal boundary** | No I/O / LLM / request-body / token / output-parsing. | PASS — 001 introduces none (FR-022); negative-scope review (SC-007). |
| **IV — Typed input / sound agreement check** | Not built in 001, but the schema's `variables` block must be expressive enough to generate-then-extend later. | PASS — FR-010a; no agreement logic yet (correctly deferred). |
| **V — Repo-canonical / git owns versioning** | No managed version axis; provenance fields shaped, not consumed. | PASS — schema has no version axis; variants modeled per Clarifications. |
| **VI — Per-language idiom** | Codegen targets each language's native shape (Pydantic/TS/serde). | PASS — D1 picks per-language generators. |
| **VII — JSON Schema single source of truth** | The whole spec. | PASS — schema + codegen + freshness gate (FR-008..019). |
| **Scope Discipline (C-08)** | No speculative pluggable interfaces. | PASS — 001 introduces none; one concrete path each. |

**No violations.** Complexity Tracking table omitted (nothing to justify).

## Project Structure

### Documentation (this feature)

```text
specs/001-foundations/
├── plan.md              # this file
├── research.md          # Phase 0 — codegen tooling (Q1 resolved)
├── data-model.md        # Phase 1 — prompt-definition shape
├── contracts/
│   └── prompt-definition.schema.json   # Phase 1 — the schema contract
├── quickstart.md        # Phase 1 — validation guide
├── memory-synthesis.md  # plan-with-memory output
└── tasks.md             # Phase 2 — /speckit.tasks (NOT created here)
```

### Source Code (repository root)

```text
crates/
├── prompting-press-core/      # engine kernel — NO pyo3/napi. Stub in 001.
│   └── Cargo.toml, src/lib.rs
├── prompting-press/           # Rust consumer crate (public Rust API). Stub in 001.
│   └── Cargo.toml, src/lib.rs
├── prompting-press-py/        # PyO3 binding crate (cdylib). Stub in 001; only crate that may dep pyo3.
│   └── Cargo.toml, src/lib.rs
└── prompting-press-node/      # napi binding crate (cdylib). Stub in 001; only crate that may dep napi.
    └── Cargo.toml, src/lib.rs

packages/
├── python/                    # published Python package wrapper (maturin) — skeleton
├── typescript/                # published npm package wrapper (napi-rs) — skeleton
└── go/                        # RESERVED placeholder (no go.mod, excluded from build/CI)

schemas/jsonschema/
└── prompt-definition.schema.json    # the single source of truth (impl copy of the contract)

schemas/jsonschema/fixtures/         # accept/reject validation fixtures (FR-013)
├── valid/*.{yaml,json}
└── invalid/*.{yaml,json}

<generated, committed, marked generated, segregated per language>:
  packages/python/.../prompt_definition.py        (Pydantic v2)
  packages/typescript/.../prompt-definition.ts    (TS types)
  crates/prompting-press/src/generated/...rs       (serde struct)  # exact paths set at task time

# Cargo workspace manifest (root Cargo.toml: members = crates/*; excludes packages/go)
# moon tasks: :build, :codegen, :check-ffi-isolation, :check-codegen-fresh
```

**Structure Decision**: the symmetric kernel + per-language-consumer layout (constitution C-01),
verified idiomatic in Phase 0. Reorg replaces the bootstrap's flat `packages/{python,typescript,go,rust}`
(FR-007). Generated-shape paths are segregated (`generated/` dirs) so they're never hand-edited and
the freshness gate catches drift.

## Phase 1 re-check (post-design)

Re-evaluated after writing the schema contract + data model: **still no constitution violations.**
The schema (`contracts/prompt-definition.schema.json`) is sealed (`additionalProperties:false`),
models the default-as-root + reserved-`default`-rejection + per-variant-sealing rules, declares
provenance/output-ref without consuming them, and has no version axis — consistent with every
principle. One Phase-0 watchpoint carried to tasks: confirm `typify`'s `const`/enum/serde-derive
output on a sample (it was not source-quotable) before relying on it; avoid `anyOf` in the schema
(typify weak area) — the contract uses `oneOf`/sealed objects only. ✓

## Complexity Tracking

No violations; section intentionally empty.
