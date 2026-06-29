# Implementation Plan: Adversarial hardening & fuzzing

**Branch**: `009-adversarial-fuzzing` (stacked on `008-api-schema-reshape`) | **Date**: 2026-06-29 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/009-adversarial-fuzzing/spec.md`

## Summary

A **test-only** hardening pass: add robustness fuzzing, property-based fuzzing, an injection/guard demo, and
secret-scrub verification to the existing suites, proving the library never panics / always returns a structured
error / never leaks values / holds its invariants under hostile + generated input. No library behavior changes
(Principle I/III). Targets the post-008 `Prompt` surface. New dev-deps pinned exact: `proptest 1.11.0`,
`arbitrary 1.4.2` (Rust), `hypothesis 6.155.7` (Python), `fast-check 4.8.0` (TS).

## Technical Context

**Language/Version**: Rust 1.95, Python ≥3.12, TS 5.9 / Node 20+ — unchanged.
**Primary Dependencies (NEW, dev/test only)**: proptest 1.11.0 + arbitrary 1.4.2 (kernel + consumer
`[dev-dependencies]`); hypothesis 6.155.7 (`packages/python` dependency-group, test only); fast-check 4.8.0
(`packages/typescript` devDependencies). None enter the runtime dep set; none add FFI to kernel/consumer.
**Testing**: extends the existing `cargo test`, `pytest`, `node --test` suites; rides the existing
`ci:test-python` / `ci:test-node` / `cargo test --workspace` gates (no new moon task required — FR-008 met by
the property/robustness tests living in the gated suites).
**Project Type**: polyglot library monorepo. **Constraints**: bounded gate (capped case count + input size);
deterministic/replayable seeds; floating-version gate (exact pins); FFI isolation preserved.

## Constitution Check

*Constitution v1.2.0. Re-checked post-design below.*

| Principle | Gate | Verdict |
|-----------|------|---------|
| **I. Shared Core** | kernel unchanged | ✅ tests only; zero kernel edits |
| **II. FFI Isolation** | no pyo3/napi in kernel/consumer | ✅ proptest/arbitrary are pure-Rust dev-deps; `ci:check-ffi` stays green |
| **III. Minimal Boundary** | no I/O / LLM / boundary growth | ✅ the pass PROVES the boundary; adds no capability. The "no jailbreak" framing (FR-006) defends the boundary honestly |
| **IV. Typed Input** | agreement check sound | ✅ property tests assert validate-before-render + the agreement guarantee under generated input |
| **V. Repo Canonical** | no version axis | ✅ n/a |
| **VI. Per-Language Idiom** | native frameworks | ✅ proptest/hypothesis/fast-check are each ecosystem's idiomatic property tool |
| **VII. JSON Schema source** | shapes codegen'd | ✅ unchanged |
| **C-08 Scope Discipline** | no unearned surface | ✅ coverage-guided fuzzing (cargo fuzz) explicitly deferred until a real need |
| **C-09** | origin metadata + advisory guard | ✅ the injection demo asserts pass-through (never sanitize) + states advisory-not-enforcement |
| **SEC-003 floating versions** | exact pins | ✅ all 4 new deps exact-pinned |
| **SEC-004 scrub** | no value leakage | ✅ this pass adversarially VERIFIES the scrub |

**No violations.**

## Implementation phasing (informs tasks)

Each binding's adversarial suite is independent → **parallelizable** across the kernel/Rust, Python, and TS.

1. **Rust (kernel + consumer)** — `proptest` property tests (never-panic, validate-before-render via the
   consumer `render<V>`, hash-determinism) + enumerated hostile corpora over `Prompt::new`/factories/render/
   check + a secret-scrub proptest. Add proptest/arbitrary to `[dev-dependencies]`. Gate: `cargo test
   --workspace`.
2. **Python** — `hypothesis` strategies over `Prompt(...)`/`from_*`/`render`/`check`; secret-scrub test
   (secret-shaped value → error has no leak); injection/guard demo. Add hypothesis to the test dependency-group.
   Gate: `ci:test-python`.
3. **TypeScript** — `fast-check` arbitraries over `new Prompt`/`from*`/`render`/`check`; secret-scrub test;
   injection/guard demo. Add fast-check to devDependencies. Gate: `ci:test-node`.
4. **Shared injection/guard demo + secret-scrub** — each binding gets the worked demo (FR-005/006) + the scrub
   verification (FR-007); the kernel gets the render-pass-through + guard-naming proptest.
5. **CI wiring + docs note** — confirm the new tests ride the existing gates in bounded time (FR-008); add a
   short honest "what the guard is / isn't" note where the injection demo lives.

## Project Structure (areas touched — all test/dev only)

```text
crates/prompting-press-core/
├── Cargo.toml                 # + [dev-dependencies] proptest 1.11.0, arbitrary 1.4.2
└── tests/fuzz_*.rs            # NEW: kernel property + hostile-corpus + guard-passthrough tests
crates/prompting-press/
├── Cargo.toml                 # + [dev-dependencies] proptest 1.11.0
└── tests/fuzz_*.rs            # NEW: consumer Prompt never-panic / validate-before-render / determinism / scrub
packages/python/
├── pyproject.toml             # + [dependency-groups] test: hypothesis==6.155.7
└── tests/test_fuzz*.py        # NEW: hypothesis property + injection demo + secret-scrub
packages/typescript/
├── package.json               # + devDependencies fast-check 4.8.0
└── test/fuzz*.test.mjs        # NEW: fast-check property + injection demo + secret-scrub
```
No new moon task — the tests live in the gated suites. (If a separate fast/slow split is ever wanted, a
`ci:fuzz` task is the seam; not built now — Scope Discipline.)

## Complexity Tracking

*No constitution violations — empty.*
