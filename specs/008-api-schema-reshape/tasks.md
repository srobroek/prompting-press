---
description: "Task list for spec 008 — Pre-publish API & schema reshape"
---

# Tasks: Pre-publish API & schema reshape

**Input**: Design documents from `specs/008-api-schema-reshape/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/prompt-api.md

**Tests**: This feature reuses the existing test suites + CI gates as its acceptance gates (no new test
framework). Test/verification tasks below are the existing gates, plus new unit/integration tests added to each
binding's existing suite.

## Format: `[ID] [P?] Description`

- **[P]**: parallelizable (different files, no dependency on an incomplete task)
- This reshape is a **coordinated outward-flow change** (schema → codegen → kernel → consumer → bindings →
  conformance); phases are strictly ordered and each ends in a CI gate. User stories (US1–US7) cut across
  phases, so each task notes the FR/SC/US it serves rather than being grouped under one story.

## Implementation-open items (decide during the phase that hits them — see autonomous-run-log A8-7/8/9)

- **A8-7**: Python `with` is a reserved keyword → pick the method name (`with_` / `derive` / `replace`) in P4.
- **A8-8**: `json-schema-to-zod` ships no formatter → add a deterministic Biome pass in the TS codegen (P1).
- **A8-9**: verify whether the Zod codegen needs the same `variants.propertyNames` `jq` strip as Rust typify (P1).

---

## Phase 1: Schema + codegen + Python floor (the source of truth — gates everything)

**Goal**: change the schema, regenerate all three shapes, move fixtures, raise the Python floor. **Gate**:
`moon run schemas:check-schema schemas:validate-fixtures schemas:codegen-check` green (incl. twice-run
determinism).

- [X] T001 Rename the per-variable trust tag `provenance` → `origin` in `schemas/jsonschema/prompt-definition.schema.json` (`$defs.VariableDecl.properties.origin` + `required: ["type","origin"]`); enum values + description unchanged. (FR-001, FR-002, FR-006; US6)
- [X] T002 Add the optional per-variable boolean `validation_required` (default `false`) to `$defs.VariableDecl.properties` in the schema, with the "declarative, per-language-enforced, kernel-blind" description. (FR-022; US7)
- [X] T003 Validate the edited schema is still Draft 2020-12 valid: `moon run schemas:check-schema`. (FR-003; SC-002)
- [X] T004 [P] Move `schemas/jsonschema/fixtures/valid/` → `schemas/jsonschema/tests/fixtures/valid/` (use `git mv`, not `rm`). (FR-007; SC-006)
- [X] T005 [P] Move `schemas/jsonschema/fixtures/invalid/` → `schemas/jsonschema/tests/fixtures/invalid/` (`git mv`). (FR-007; SC-006)
- [X] T006 Update the fixture path in `schemas/jsonschema/scripts/validate_fixtures.py` to the new `tests/fixtures/` location. (FR-008; SC-006)
- [X] T007 Update the `schemas:validate-fixtures` moon-task `inputs` glob in `schemas/moon.yml` (`jsonschema/fixtures/**/*` → `jsonschema/tests/fixtures/**/*`). (FR-008; SC-006)
- [X] T008 Update the fixture references in `conformance/schema/manifest.json` AND **preserve the documented `variant-named-default` loader-exclusion note** (architecture memory A1 — schema-invalid but loader-accepted). (FR-008, FR-009; SC-006)
- [X] T009 Rename the `bad-provenance.json` invalid fixture → `bad-origin.json` and update its content + any manifest reference to the `origin` field. (FR-001, FR-005; SC-002)
- [X] T010 Update any `origin`/`provenance` field usage inside the moved `valid/` fixtures (e.g. `multi-variant.json`, `single-body.json`, `variant-with-meta.json`) to the `origin` key. (FR-005; SC-002)
- [X] T011 Raise the Python floor: `packages/python/pyproject.toml` `requires-python = ">=3.12"` + update the `Programming Language :: Python` classifiers. (R5/A8-5; FR-013)
- [X] T012 [P] Bump pyo3 ABI: `crates/prompting-press-py/Cargo.toml` feature `abi3-py310` → `abi3-py312`. (R5/A8-5)
- [X] T013 [P] Update `packages/python/scripts/codegen.sh` `--target-python-version 3.10` → `3.12`. (R5/A8-5)
- [X] T014 [P] Update the CI Python matrix floor in `.github/workflows/ci.yml` and fix the STALE `abi3-py39`/`py310` comments to `abi3-py312`. (R5/A8-5)
- [X] T015 Regenerate the Rust shape: run `crates/prompting-press-core/scripts/codegen.sh` (cargo-typify@0.7.0); confirm the `origin` enum + `validation_required: Option<bool>` field appear and the `jq 'del(.properties.variants.propertyNames)'` strip still applies. (FR-003; SC-002)
- [X] T016 Regenerate the Python shape: run `packages/python/scripts/codegen.sh` (datamodel-codegen, target 3.12); confirm `origin` + `validation_required`. (FR-003; SC-002)
- [X] T017 ~~Rewrite codegen.mjs to json-schema-to-zod~~ **NO-OP — Q1 REVERTED** (A8-11, user-confirmed 2026-06-28): the TS generated prompt-definition shape STAYS a `json-schema-to-typescript` interface. Rationale: the document is runtime-validated by Rust serde at the napi boundary (TS never re-validates it); json-schema-to-zod collapses the `VariableDecl` `$ref` to `z.any()` so a generated Zod schema would be WEAKER than serde; and `validation_required` coverage rides on the CALLER's render-vars Zod validator (`.shape`), a separate schema. No dep added; codegen.mjs unchanged (just regenerated → T019). (FR-014 satisfied differently: TS runtime enforcer is the caller's Zod validator + Rust serde, not the generated shape.)
- [X] T018 ~~Verify propertyNames strip for Zod codegen~~ **MOOT** (T017 reverted; `json-schema-to-typescript` already handles the schema, unchanged since spec 005). (A8-9 closed.)
- [X] T019 Regenerate the TS shape: `pnpm -C packages/typescript codegen`; confirm a Zod schema + exported type with `origin` (`z.enum`) + `validation_required` (`z.boolean().default(false)`). (FR-003; SC-002)
- [X] T020 GATE: `moon run schemas:check-schema schemas:validate-fixtures schemas:codegen-check --force` all green (twice-run determinism for all 3 shapes). (FR-003, FR-025/FR-028; SC-006)

---

## Phase 2: Kernel rename (behavior identical — Principle I)

**Goal**: rename the kernel's provenance surface to `origin` vocabulary; ZERO behavior change. **Gate**:
`cargo test -p prompting-press-core` + `moon run ci:check-ffi`.

- [X] T021 Rename `crates/prompting-press-core/src/provenance.rs` → `origin.rs` (`git mv`) and update the `mod`/`pub use` in `lib.rs`. (FR-004)
- [X] T022 Rename the symbols in that module: `ProvenanceView` → `OriginView` (fields `untrusted`/`external` unchanged), `provenance_view` → `origin_view`, and switch to the regenerated `origin` enum from the generated shape. Logic/algorithm byte-for-byte identical; update doc comments from "provenance" → "origin" for the per-variable tag ONLY. (FR-004; SC-002)
- [X] T023 Update all kernel call sites + tests referencing the renamed symbols (`engine.rs`, `lib.rs`, `tests/*.rs`, `tests/fixtures/defs/*.json` using the `provenance` field). Keep render/guard/hash behavior unchanged. (FR-004, FR-027; SC-002, SC-003)
- [X] T024 Confirm the render-result provenance (`template_hash`/`render_hash` in `hashing.rs`/`engine.rs`) is NOT renamed. (FR-002; SC-002)
- [X] T025 GATE: `cargo test -p prompting-press-core` green; `moon run ci:check-ffi` green (no FFI dep crept into the kernel). (FR-027, FR-028)

---

## Phase 3: Rust consumer — the `Prompt` object (Facade)

**Goal**: introduce the immutable `Prompt`, migrate ops onto it, `with` as sole mutator, drop `Registry`.
**Gate**: `cargo test -p prompting-press`.

- [ ] T026 Create `crates/prompting-press/src/prompt.rs`: `Prompt` struct wrapping the generated `PromptDefinition`; `Prompt::new(shape) -> Result<Prompt, ConsumerError>` (validating: shape OK by type; parse + `required_roots` agreement → construction error on excluded-feature/syntax/undeclared). Read-only accessors. (FR-010, FR-011, FR-012, FR-020; US1, US3)
- [ ] T027 Add the text factories `Prompt::from_yaml` / `from_json` / `from_toml` (add `toml@1.1.2` to `crates/prompting-press/Cargo.toml` + workspace deps); fold the old `registry.rs` dual-input loaders into these. (FR-013; US1, SC-010)
- [ ] T028 Move `render` onto `Prompt`: `Prompt::render<V: Serialize + Validate>(&self, vars, variant, guard)` delegating to the unchanged kernel; byte-identical output. (FR-015, FR-016; US1, SC-003)
- [ ] T029 Move `get_source` + `check` onto `Prompt`: `Prompt::get_source(variant)` and `Prompt::check() -> CheckReport` (advisory: origin/guard finding; analysis-error arm unreachable post-construction). (FR-015, FR-020, FR-021; US3)
- [ ] T030 Implement `Prompt::with(overlay) -> Result<Prompt>`: shallow-replace per top-level field (incl. `name`), re-validate the merged whole through `Prompt::new`, original untouched. Define `PromptOverlay`. (FR-017; US2, SC-004)
- [ ] T031 Migrate `Composition` to aggregate `Prompt` objects (not names); `resolve()` over held objects, no registry. (FR-018; US4)
- [ ] T032 Remove `Registry` (`crates/prompting-press/src/registry.rs`) + the registry-keyed free fns; update `lib.rs` re-exports (`Prompt`/`Composition` in; `Registry`/`render`/`get_source`/`check` free-fns out). (FR-019; US5, SC-001)
- [ ] T033 Add Rust unit/integration tests for: construct valid/invalid (incl. excluded-feature → construction error), `with` immutability + merged-validation, `from_toml`, no-`Registry`. (SC-001, SC-004, SC-005, SC-009, SC-010)
- [ ] T034 GATE: `cargo test -p prompting-press` green. (FR-028)

---

## Phase 4: Python binding + facade

**Goal**: `Prompt` pyclass with validators bound at construction; drop `Registry`. **Gate**:
`cargo test -p prompting-press-py` + `pytest packages/python/tests`.

- [ ] T035 Add the `Prompt` pyclass (`crates/prompting-press-py/src/`): primary constructor `Prompt(shape, *, validators=None)` (raises `PromptValidationError` on invalid shape/parse/agreement); `from_yaml`/`from_json`/`from_toml` (stdlib `tomllib` — free at 3.12); read-only `@property` accessors. Kernel-direct render. (FR-010..013, FR-020; US1, US3, SC-010)
- [ ] T036 Bind validators at construction: accept a Pydantic-model-based validator map; check every `validation_required: true` variable is covered (introspect `model_fields`) and RAISE naming the uncovered variable if not. (FR-022, FR-023, FR-024; US7, SC-009)
- [ ] T037 Move `render`/`get_source`/`check` onto the pyclass (keyword-only optional tail — C-11); SEC-004 scrub on every new error path (route `KernelError` through the consumer `From` first; Pydantic mapper copies `msg`/`loc` only). (FR-015, FR-025; SC-008)
- [ ] T038 Implement the sole mutator on the pyclass — **A8-7: pick the non-reserved name** (`with_` recommended); validators carry forward + optional override; re-validate merged whole. (FR-017, FR-023; US2, SC-004)
- [ ] T039 Migrate `Composition` to aggregate `Prompt` objects; remove `Registry` from the pyclass module + the facade `__init__.py` `__all__`/imports (Registry out, Prompt in). (FR-018, FR-019; US4, US5, SC-001)
- [ ] T040 Update `pytest` tests for the object surface: construct valid/invalid, coverage-raise, `with_` immutability, `from_toml`, no-`Registry`. (SC-001, SC-004, SC-005, SC-009, SC-010)
- [ ] T041 GATE: `cargo test -p prompting-press-py` + `moon run ci:test-python` green (build maturin from `packages/python/`). (FR-028)

---

## Phase 5: TypeScript binding + facade

**Goal**: `Prompt` class (`new` throws), Zod-schema-backed, drop `Registry` + duck-typing. **Gate**:
`cargo test -p prompting-press-node` + `node --test`.

- [ ] T042 Add the `Prompt` class to `packages/typescript/src/index.ts`: primary constructor `new Prompt(shape, validators?)` that **throws** `PromptValidationError` (carrying `[{field,code,message}]`) on invalid shape/parse/agreement; static `fromYaml`/`fromJson`/`fromToml` (add `smol-toml@1.7.0` to package.json); read-only accessors. (FR-010..014, FR-020; US1, US3, US6, SC-010)
- [ ] T043 Bind validators at construction; check `validation_required` coverage via `ZodObject.shape` (`field in schema.shape`) and throw naming the uncovered variable; extend `ZodLikeSchema` with optional `shape` (absent → documented "cannot assert coverage"). (FR-022..024; US7, SC-009)
- [ ] T044 Move `render`/`getSource`/`check` onto the class (options-object tail — C-11); **remove the `isSchema()` duck-typing** (the named validators arg removes the ambiguity). (FR-015; US1)
- [ ] T045 Implement `with(overlay, validators?)` (throws on invalid merged; validators carry forward + override). (FR-017, FR-023; US2, SC-004)
- [ ] T046 Migrate `Composition` to aggregate `Prompt` objects; remove the `Registry` class + registry-keyed `render`/`getSource`/`check` exports from `index.ts`. (FR-018, FR-019; US4, US5, SC-001)
- [ ] T047 Update `node --test` suites for the object surface: construct valid/invalid (throw), coverage-throw, `with` immutability, `fromToml`, no-`Registry`. (SC-001, SC-004, SC-005, SC-009, SC-010)
- [ ] T048 GATE: `cargo test -p prompting-press-node` + `moon run ci:test-node` green. (FR-028)

---

## Phase 6: Conformance + docs + examples sweep

**Goal**: re-prove cross-binding parity over the renamed field/moved fixtures; eliminate every stale reference.
**Gate**: `moon run ci:conformance` + full CI.

- [ ] T049 Update the conformance corpus: the `origin` field in `conformance/schema/yaml/*.yaml` + `conformance/marshaling/*.json`, and any fixture paths in the runners; keep the canonical-serialized-form parity approach (decision memory D1 — do NOT build native objects). (FR-005; SC-003)
- [ ] T050 Update the conformance runners to drive the new `Prompt` object surface (construct + render) instead of the `Registry` + free-fn surface, in all three language runners. (FR-016, FR-019; SC-003, SC-001)
- [ ] T051 GATE: `moon run ci:conformance` green (byte-identical hashes across all three bindings over the renamed field). (SC-003)
- [ ] T052 [P] Sweep + reconcile docs for the OLD surface (mid-amendment lesson): every `README.md`, package README, quickstart, and merged-spec quickstart referencing `Registry`/`render(reg,name,…)`/the `provenance` field → update to the `Prompt`/`origin` surface. `rg -n 'Registry|provenance|render\(reg'` across `README.md packages/*/README.md` etc. (FR-005; SC-002, SC-007)
- [ ] T053 [P] Update `AGENTS.md`/`CLAUDE.md` only if they describe the old surface (NOT the APM-generated constitution block — leave APM-managed content alone, per A8-10). (FR-005)
- [ ] T054 GATE: full CI green — `moon run ci:check-ffi ci:conformance ci:test-python ci:test-node` + `cargo test --workspace`. (FR-028; SC-007)

---

## Dependencies & ordering

> **FR-026 (all-bindings parity)** is satisfied collectively by Phases 3 (Rust) + 4 (Python) + 5 (TS) — the
> `Prompt` object lands in all three. It is the umbrella requirement those three phases fulfill, not a single
> task.

- **Phase 1 → 2 → 3 → {4, 5} → 6.** Phases 1–3 are strictly serial (schema gates codegen gates kernel gates
  consumer). **Phases 4 and 5 are parallelizable with each other** (Python and TS bindings are independent
  crates/packages over the same consumer) — they may run concurrently once Phase 3 lands. Phase 6 needs all
  bindings done.
- Within Phase 1: T001–T003 (schema) before T015–T019 (codegen); T004–T010 (fixtures) independent of codegen;
  T011–T014 (Python floor) independent `[P]`. T020 gate closes the phase.
- The `[P]` markers indicate genuinely independent files; do not parallelize tasks touching the same file.

## Implementation strategy

- **MVP / first vertical slice**: Phases 1–3 (schema + kernel + Rust consumer `Prompt`) deliver the reshape in
  Rust end-to-end and prove the object model before the FFI bindings replicate it.
- **Incremental**: each phase ends green; the bindings (4, 5) mirror the Rust shape; conformance (6) is the
  final cross-language proof.
- **Commit checkpoints** after each phase gate (subject to signing availability — see autonomous-run-log [B1]).
