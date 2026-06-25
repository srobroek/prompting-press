# Project Context

Last reviewed: 2026-06-25

## Product / Service

**Prompting Press** — a typed, versioned, variant-aware **prompt-template library** (the prompt
analogue of a typed config system). It parses, validates, and renders LLM prompt text and stamps
provenance. Polyglot: one shared Rust engine (`prompting-press-core`) bound into Python and
TypeScript (Go deferred), so a prompt renders byte-identically across languages by construction.
Reusable standalone; Bellwether (`claudebroker`) is consumer #1, not the owner.

The differentiator: typed prompt **input** + a **sound agreement check** (a template's referenced
variables must be a subset of its declared, typed Vars fields — caught at CI, not silently
mis-rendered). This is the BAML-equivalent static guarantee no file-based library provides.

## Key Constraints

- **Minimal boundary (constitution Principle III, non-negotiable)**: NO I/O, NO LLM calls, NO
  provider request-body assembly, NO token counting (hook only), NO output parsing. The user pushes
  prompt data in; the library returns text + provenance.
- **Shared core, structural parity (C-01)**: one Rust engine; cross-language rendering is identical
  by construction, not by test. No per-language reimplementation.
- **FFI isolation (C-02)**: `pyo3`/`napi` live only in binding crates, never in
  `prompting-press-core` or the `prompting-press` Rust consumer crate. CI-enforced.
- **Repo-canonical; git owns versioning (C-05)**: no managed version axis; variants are named,
  caller-selected alternatives; provenance = per-variant `template_hash` + `render_hash`.
- **JSON Schema is the single source of truth (C-07)**: per-language prompt-definition shapes are
  codegen'd from one schema; YAML+JSON push input.
- **Scope discipline (C-08)**: all five of the original brief's pluggable interfaces eliminated; no
  new pluggable interface until a second concrete implementation exists.

## Important Domains

- Prompt templating (MiniJinja, Jinja-family; interpolation/conditionals/loops only — no
  includes/macros/inheritance in v1)
- Typed input validation (Pydantic / Zod / garde, generated-then-extended from the schema)
- Cross-language FFI marshaling (PyO3, napi-rs) + the conformance corpus that guards it
- Content-addressed provenance (template_hash, render_hash)

## Current Priorities

- **Spec 001 (Foundations)**: crate layout + prompt-definition JSON Schema + codegen pipeline + CI
  guardrails. The structural spine; no rendering yet.
- Then specs 002 (engine kernel) → 003 (Rust consumer) → 004 (Python) → 005 (TypeScript) → 006
  (conformance corpus) → 007 (v1 release). See `.specify/memory/roadmap.md`.

## Keep Here

- durable product constraints (the boundary; the shared-core invariant)
- domain language and invariants (what "variant", "provenance", "agreement check" mean here)
- project-wide priorities that shape feature tradeoffs

## Never Store Here

- feature-specific acceptance criteria
- task lists
- transient implementation notes
- changelog entries

Update the review date when constraints or priorities materially change.
