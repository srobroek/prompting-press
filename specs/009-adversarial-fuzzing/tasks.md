---
description: "Task list for spec 009 — Adversarial hardening & fuzzing"
---

# Tasks: Adversarial hardening & fuzzing

**Input**: Design documents from `specs/009-adversarial-fuzzing/`

**Prerequisites**: plan.md, spec.md

**Tests**: This spec IS tests — every task adds adversarial coverage to an existing suite. No new framework
beyond the pinned property-testing libs. No library behavior changes (Principle I/III).

## Format: `[ID] [P?] Description`

- **[P]**: parallelizable — the three language suites are independent test additions.

## Phase 1: Rust (kernel + consumer)

**Gate**: `cargo test --workspace` green; `ci:check-ffi` + `ci:check-floating-versions` green.

- [X] T001 Add `[dev-dependencies]` `proptest = "1.11.0"` + `arbitrary = "1.4.2"` to `crates/prompting-press-core/Cargo.toml`, and `proptest = "1.11.0"` to `crates/prompting-press/Cargo.toml`. Dev-only (must NOT enter runtime deps; must NOT trip `ci:check-ffi`). (FR-009)
- [X] T002 [P] Kernel robustness corpus `crates/prompting-press-core/tests/fuzz_robustness.rs`: feed malformed / oversized / deeply-nested / Unicode / control-char bodies + values to `render`/`required_roots`/`origin_view`; assert each returns `Ok`/`Err(KernelError)` — never panics. (FR-001; US1; SC-001)
- [X] T003 [P] Kernel property tests `crates/prompting-press-core/tests/fuzz_properties.rs` (proptest): hash-determinism (same def+values → identical `template_hash`/`render_hash` across two renders); never-panic across a generated value space; bounded cases + fixed seed. (FR-003, FR-004; US2; SC-004)
- [X] T004 [P] Kernel guard-passthrough proptest: an `untrusted` field with generated injection-shaped values renders **verbatim** (output contains the value byte-for-byte) and guard-on body == guard-off body. (FR-005; US3; SC-006)
- [X] T005 Consumer `crates/prompting-press/tests/fuzz_prompt.rs` (proptest): over `Prompt::new`/`from_yaml`/`from_json`/`from_toml`/`render::<V>`/`check`/`with` — never-panic; validate-before-render (an invalid `V` never reaches a kernel render); construction rejects hostile/un-analyzable docs with a structured `ConsumerError`. (FR-001, FR-003; US1, US2; SC-002, SC-005)
- [X] T006 Consumer secret-scrub proptest `crates/prompting-press/tests/fuzz_scrub.rs`: a secret-shaped value triggering a parse/render error → the secret substring is absent from `ConsumerError`'s Display, its `[{field,code,message}]` rows, and the error chain. (FR-002, FR-007; US4; SC-003)
- [X] T007 GATE: `cargo test --workspace` green; `ci:check-ffi` green (proptest/arbitrary stayed out of kernel/consumer runtime); `ci:check-floating-versions` green (exact pins). (FR-008, FR-009, FR-010)

## Phase 2: Python (hypothesis)

**Gate**: `ci:test-python` green.

- [X] T008 Add `hypothesis==6.155.7` to the test dependency-group in `packages/python/pyproject.toml` (+ lock). Test-only. (FR-009)
- [X] T009 [P] `packages/python/tests/test_fuzz_robustness.py` (hypothesis): strategies generating hostile shapes/values fed to `Prompt(...)`/`from_yaml`/`from_json`/`from_toml`/`render`/`check`; assert a structured `PromptingPressError` subtype or a valid result — never an unraised crash / interpreter abort. (FR-001, FR-003; US1, US2; SC-001, SC-002, SC-005)
- [X] T010 [P] `packages/python/tests/test_fuzz_scrub.py`: secret-shaped value → error; assert the secret substring is absent from `str(err)`, `err.errors` rows, and the traceback (never surfaces the raw `pydantic.ValidationError` input — the spec-004 M-1 lesson). (FR-002, FR-007; US4; SC-003)
- [X] T011 [P] `packages/python/tests/test_fuzz_injection_demo.py`: construct a prompt with an `untrusted` field; assert `prompt.check()` flags it unguarded; the injection value renders verbatim; the opt-in guard names the field with a byte-identical body. Include an explicit comment: advisory text, NOT enforcement; no LLM. (FR-005, FR-006; US3; SC-006)
- [X] T012 GATE: `ci:test-python` green (hypothesis tests ride the existing gate). (FR-008)

## Phase 3: TypeScript (fast-check)

**Gate**: `ci:test-node` green.

- [X] T013 Add `fast-check@4.8.0` to `devDependencies` in `packages/typescript/package.json` (+ lock). Test-only. (FR-009)
- [X] T014 [P] `packages/typescript/test/fuzz.robustness.test.mjs` (fast-check): arbitraries over `new Prompt`/`fromYaml`/`fromJson`/`fromToml`/`render`/`check`; assert each throws a `PromptingPressError` subclass or returns a value — never an unhandled throw / process crash; hash-determinism property. (FR-001, FR-003, FR-004; US1, US2; SC-001, SC-004, SC-005)
- [X] T015 [P] `packages/typescript/test/fuzz.scrub.test.mjs`: secret-shaped value → error; assert the substring is absent from `err.message`, `err.errors` rows, and `err.stack` (the Zod mapper copies message+path only). (FR-002, FR-007; US4; SC-003)
- [X] T016 [P] `packages/typescript/test/fuzz.injection-demo.test.mjs`: `untrusted` field; `prompt.check()` flags it; injection value renders verbatim; opt-in guard names the field, body byte-identical. Explicit advisory-not-enforcement comment. (FR-005, FR-006; US3; SC-006)
- [X] T017 GATE: `ci:test-node` green. (FR-008)

## Phase 4: Cross-cutting verification

- [X] T018 Confirm the published runtime dep sets are UNCHANGED (the 4 frameworks are dev/test only): inspect each manifest's runtime vs dev sections. (FR-009; SC-008)
- [X] T019 GATE: full CI green — `cargo test --workspace` + `ci:check-ffi ci:check-floating-versions ci:test-python ci:test-node ci:conformance`. Confirm the adversarial suites run in bounded time (capped cases). (FR-008, FR-010; SC-007)

## Dependencies & ordering

- Phases 1 / 2 / 3 are **independent and parallelizable** (separate suites). Phase 4 needs all three.
- Within each phase: add the dep first, then the test files, then the gate.
- No kernel/consumer/binding *source* changes — tests + manifests only.

## Implementation strategy

Delegate the three language phases to parallel `coder` agents (separate file sets; the proven 008 pattern), each
with the integrity-check preamble + its own gate. Re-verify each agent's diff + gate independently (the
fabrication lesson). Bound every property suite (case count + size) so the CI gate stays fast.
