# Prompting Press

A typed, versioned, variant-aware **prompt-template library** with one shared engine across
languages. One prompt definition renders byte-identically in Python, TypeScript, and Rust — by
construction (a single compiled Rust core), not by per-language reimplementation.

> **Status:** Foundations (spec 001) implemented. This is the structural spine — no prompt renders
> yet, by design. The engine, typed-input validation, and bindings land in specs 002–007.

## What it is (and isn't)

It parses/validates/renders prompt text and stamps content-addressed provenance. It deliberately does
**no** I/O, no LLM calls, no request-body assembly, no token counting, and no output parsing — it stays
a drop-in alongside any call layer (constitution Principle III).

## Architecture

```
crates/
├── prompting-press-core/   # FFI-free engine kernel (the shared core)
├── prompting-press/        # public Rust consumer API (re-exports generated types)
├── prompting-press-py/     # PyO3 binding (cdylib) — the only crate that may use pyo3
└── prompting-press-node/   # napi-rs binding (cdylib) — the only crate that may use napi
packages/
├── python/                 # maturin-built wheel wrapper
├── typescript/             # napi-rs npm package wrapper
└── go/                     # reserved placeholder (deferred; no toolchain)
schemas/jsonschema/         # prompt-definition.schema.json — the SINGLE SOURCE OF TRUTH
```

The JSON Schema is authoritative; the per-language shapes (Pydantic v2 / TS types / Rust serde structs)
are **code-generated** from it and committed, then freshness-gated in CI. FFI deps (`pyo3`/`napi`) are
mechanically kept out of the kernel and consumer crates.

## Develop

Toolchain is pinned via [`mise`](https://mise.jdx.dev) (`mise install`) and orchestrated with
[`moon`](https://moonrepo.dev). Run tasks with `mise exec -- moon run <task>`:

| Task | What it does |
|------|--------------|
| `:build` | build all 4 crates |
| `:codegen` | regenerate the 3 language shapes from the schema |
| `schemas:check-schema` | meta-validate the schema (Draft 2020-12) |
| `schemas:validate-fixtures` | accept/reject fixture suite |
| `schemas:codegen-check` | codegen-freshness gate (regenerate → no diff) |
| `ci:check-ffi` | FFI-isolation gate (no pyo3/napi in kernel/consumer) |
| `ci:check-floating-versions` | reject floating dependency versions (SEC-003) |

Generated files under `**/generated/` are **never hand-edited** — regenerate and commit.

## Governance

Development follows a spec-driven workflow. The project [constitution](.specify/memory/constitution.md)
(Principles I–VII) and [spec roadmap](.specify/memory/roadmap.md) (specs 001–007, decisions C-01..C-10)
are the artifacts of record. Licensed under Apache-2.0.
