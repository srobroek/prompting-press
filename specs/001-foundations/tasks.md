---
description: "Task list for spec 001 — Foundations"
---

# Tasks: Foundations — Layout, Schema, Codegen, CI Guardrails

**Input**: Design documents from `specs/001-foundations/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/prompt-definition.schema.json, quickstart.md

**Tests**: This spec's "tests" are its acceptance mechanism — schema accept/reject fixtures (FR-013),
codegen determinism (re-run no-diff), and CI-gate behavior checks. They are deliverables, included as
implementation tasks (not optional TDD tests). No runtime logic exists to unit-test in 001.

**Organization**: by user story. NOTE — unlike a typical feature, 001's stories are **sequential, not
parallel**: US1 (layout) is the foundation everything lives in; US2 needs US1; US3 needs US2+US1; US4's
gates attach to US1 (FFI) and US3 (freshness). Cross-story `[P]` is therefore rare; within-story `[P]`
(different files) is common.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no incomplete-task dependency)
- **[Story]**: US1 (layout) · US2 (schema) · US3 (codegen) · US4 (CI guardrails)
- Exact file paths included.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Root-level scaffolding every later phase needs. No crate logic yet.

- [X] T001 Create the root Cargo virtual workspace manifest `Cargo.toml` with `[workspace]` `members = ["crates/*"]`, `resolver = "2"`, and `[workspace.package]`/`[workspace.dependencies]` shared-metadata stanzas (no members exist yet — added in US1).
- [X] T002 [P] Add `rust-toolchain.toml` pinning an explicit stable channel (not floating `stable`) — required for `cargo-typify`/rustfmt codegen determinism (research D1/D4).
- [X] T003 [P] Update `.gitignore` for the new layout: ensure Rust `target/`, Python build/`.venv`, Node `node_modules/`, and generated-artifact build dirs are handled (the committed generated shapes are NOT ignored — they're tracked + freshness-gated).
- [X] T004 Remove the bootstrap's flat `packages/{python,typescript,go,rust}` skeleton (FR-007), leaving no orphaned duplicate; record the removal so the reorg is auditable.

**Checkpoint**: empty workspace manifest + pinned toolchain ready.

---

## Phase 2: User Story 1 — Buildable polyglot workspace (Priority: P1) 🎯 MVP

**Goal**: the load-bearing crate/package layout, all members building as stubs, correct dependency
direction (kernel ← consumer ← bindings; kernel FFI-free), Go reserved-only.

**Independent Test**: `moon run :build` (or the orchestrated build) builds all 4 crates as stubs;
`cargo tree -p prompting-press-core` shows no binding/FFI crate; `packages/go` is excluded from the build.

- [X] T005 [P] [US1] Create `crates/prompting-press-core/` stub crate (`Cargo.toml` + `src/lib.rs`), library, **no** `pyo3`/`napi` deps — the engine kernel (FR-001).
- [X] T006 [US1] Create `crates/prompting-press/` stub crate depending on `prompting-press-core`; **no** FFI deps; this is the Rust consumer/public surface (FR-002). (depends on T005)
- [X] T007 [P] [US1] Create `crates/prompting-press-py/` stub crate: `crate-type = ["cdylib"]`, depends on core/consumer + `pyo3` — the only crate that may dep `pyo3` (FR-003).
- [X] T008 [P] [US1] Create `crates/prompting-press-node/` stub crate: `crate-type = ["cdylib"]`, depends on core/consumer + `napi`/`napi-derive` — the only crate that may dep `napi` (FR-003).
- [X] T009 [P] [US1] Create `packages/python/` published-package skeleton (pyproject.toml + maturin config pointing at `prompting-press-py`; no logic) (FR-004). **Verify `maturin` is available** before relying on it (critique E3 / CHK026).
- [X] T010 [P] [US1] Create `packages/typescript/` published-package skeleton (package.json + napi-rs CLI config pointing at `prompting-press-node`; no logic) (FR-004). **Verify the `napi-rs` CLI is available** before relying on it (critique E3 / CHK026).
- [X] T011 [P] [US1] Create `packages/go/` reserved placeholder (a marker README only; **no** `go.mod`, no toolchain), excluded from the workspace and build (FR-005).
- [X] T012 [US1] Add the 4 crates to the root `Cargo.toml` workspace members; confirm `packages/go` is NOT a member. (depends on T005–T008)
- [X] T013 [US1] Wire moon projects/tasks for the active members (`:build`, `:test`); one orchestrated build/test command (FR-006). **Replace the bootstrap `.moon/workspace.yml` globs** (`apps/*`…`packages/*`…`tools/*` — which don't match `crates/*` and would sweep in `packages/go`) with explicit/enumerated membership: include `crates/*`, exclude `packages/go`, so no crate falls outside the gates by glob accident (security SEC-005). (depends on T012)
- [X] T014 [US1] Verify the layout: `moon run :build` builds all 4 stub crates; `cargo tree -p prompting-press-core` and `-p prompting-press` show no `pyo3`/`napi` (manual confirmation of the invariant US4 will then automate). (depends on T013) — satisfies SC-001, acceptance US1.

**Checkpoint**: buildable polyglot workspace; dependency direction correct; Go reserved. **MVP increment.**

---

## Phase 3: User Story 2 — Prompt-definition JSON Schema (Priority: P2)

**Goal**: the authoritative JSON Schema (single source of truth) + accept/reject fixtures.

**Independent Test**: the schema meta-validates as Draft 2020-12; 100% of well-formed fixtures accept,
100% of malformed reject (incl. named-`default` and per-variant-extras rejections).

- [X] T015 [US2] Promote the design contract to the implementation location: author `schemas/jsonschema/prompt-definition.schema.json` from `specs/001-foundations/contracts/prompt-definition.schema.json` — Draft 2020-12, stable `$id`, sealed root, `role`/`provenance` enums, `variables` block (type+provenance+constraints, FR-010a), root `body`=default, `variants` map with named-`default` rejection (FR-011b) and per-variant sealing (FR-011a), opaque `meta`/`metadata`. Avoid `anyOf` (typify weak area, research D1/D5). (FR-008..012)
- [X] T016 [US2] Add a schema meta-validation check (assert the schema is itself a valid Draft 2020-12 document — FR-008), runnable as a moon task.
- [X] T017 [P] [US2] Create well-formed (`accept`) fixtures under `schemas/jsonschema/fixtures/valid/`: single-body (no variants), multi-variant, variant-with-`meta` (per data-model.md matrix). (FR-013)
- [X] T018 [P] [US2] Create malformed (`reject`) fixtures under `schemas/jsonschema/fixtures/invalid/`: invalid role, invalid provenance tag, `variants` entry named `default`, variant redefining role/variables, extra root key, non-parseable doc. (FR-013)
- [X] T019 [US2] Add a fixtures-validation task (validate every `valid/` doc → accept, every `invalid/` doc → reject) as a moon task; this is the US2 acceptance gate. (depends on T015–T018) — satisfies SC-002, SC-006.

**Checkpoint**: schema is the single source of truth and proven by fixtures.

---

## Phase 4: User Story 3 — Codegen pipeline (Priority: P3)

**Goal**: deterministic schema→shape generation for Python/TS/Rust; artifacts committed + marked generated.

**Independent Test**: `moon run :codegen` produces 3 shapes; re-running yields zero diff; a schema field
change propagates to all 3.

- [ ] T020 [P] [US3] Pin Python codegen: add `datamodel-code-generator` 0.65.1 to the Python tool deps with determinism flags (`--disable-timestamp`, `--formatters builtin`, version-header off) — research D1. **Hash-pin** the codegen toolchain (`requirements.txt --hash=` + `--require-hashes`, or a committed `uv.lock`) and install hashed in CI — not a bare version string (security SEC-001/002).
- [ ] T021 [P] [US3] Pin TS codegen: add `json-schema-to-typescript` 15.0.4 + pinned Prettier to `packages/typescript` dev deps, `--bannerComment ''` — research D1. **Commit the lockfile** (`package-lock.json`/`pnpm-lock.yaml`, which carries integrity hashes) and install with `npm ci`/`--frozen-lockfile` in CI (security SEC-001/002).
- [ ] T022 [P] [US3] Pin Rust codegen: `cargo-typify` 0.7.0 installed `--locked`, CLI mode (not the macro); rustfmt pinned via the T002 toolchain — research D1. **Verify on a sample** that typify's `const`/string-enum/serde-derive output is as expected (research residual unknown) before wiring.
- [ ] T023 [US3] Generate the Python Pydantic v2 shape from `schemas/jsonschema/prompt-definition.schema.json` into a marked-generated, segregated path under `packages/python/` (e.g. `.../generated/prompt_definition.py`). (depends on T015, T020) (FR-014/016)
- [ ] T024 [US3] Generate the TS type shape into a marked-generated path under `packages/typescript/` (e.g. `.../generated/prompt-definition.ts`). (depends on T015, T021) (FR-014/016)
- [ ] T025 [US3] Generate the Rust serde struct into a marked-generated `generated/` module (e.g. `crates/prompting-press/src/generated/prompt_definition.rs`) that `lib.rs` re-exports but never hand-edits (critique E1); confirm `metadata`/`meta`→`serde_json::Value`, sealed objects→`deny_unknown_fields`. (depends on T015, T022) (FR-014/016)
- [ ] T026 [US3] Add a single `moon run :codegen` task orchestrating T023–T025; **wire the moon task graph so `:codegen` runs before `:build`** (the consumer crate won't compile until its generated module exists — critique E1); commit the generated artifacts. (depends on T023–T025) (FR-017)
- [ ] T027 [US3] Verify determinism: run `:codegen` twice, assert `git diff --exit-code` clean; edit one schema field, regenerate, confirm all 3 shapes change. (depends on T026) — satisfies SC-003.

**Checkpoint**: one schema → three deterministic, committed shapes.

---

## Phase 5: User Story 4 — CI guardrails (Priority: P1)

**Goal**: mechanically enforce FFI-isolation (C-02) and codegen-freshness (C-07). NOTE: the FFI gate
(T028) only needs US1; the freshness gate (T029) needs US3 — hence US4 lands last despite P1.

**Independent Test**: clean tree passes both; adding `pyo3` to the kernel fails the FFI gate; a stale
generated file fails the freshness gate; each failure names the invariant + location.

- [ ] T028 [US4] Add the FFI-isolation CI gate: a check (moon task + `.github/workflows/`) that runs `cargo tree -p <crate> -i pyo3`/`napi` and fails if found, citing Principle II / C-02 with the offending crate. **Drive it from an explicit, reviewable covered-crate list** (currently `prompting-press-core`, `prompting-press`) so a future FFI-free crate cannot silently escape the gate (critique E2 / CHK006). (depends on US1; research D3) (FR-018, FR-020)
- [ ] T029 [US4] Add the codegen-freshness CI gate: regenerate via `:codegen`, then `datamodel-codegen --check` (Python) + `git add -N . && git diff --exit-code` (TS/Rust), failing on any drift incl. partial regeneration, with a clear message. (depends on US3; research D2) (FR-019, FR-020)
- [ ] T030 [US4] Add the schema + fixtures checks (T016, T019) to the CI workflow so the schema contract is gated too.
- [ ] T030a [P] [US4] Add a floating-version lint (CI): reject `^`/`~`/`"latest"`/`"*"` in the codegen toolchain manifests; also pin `mise.toml`'s `jq` (currently `"latest"`) (security SEC-003).
- [ ] T031 [US4] Verify gate behavior in a scratch branch: add `pyo3` to `prompting-press-core` → FFI gate fails; hand-edit a generated shape → freshness gate fails; revert → both green. (depends on T028, T029) — satisfies SC-004, SC-005.

**Checkpoint**: the constitution's structural invariants are mechanically enforced.

---

## Phase 6: Polish & Cross-Cutting

- [ ] T032 [P] Add per-package README skeletons (crates + packages) noting "generated — do not edit" for generated dirs (FR-016 clarity). *(Registry-name reservation is spec 007, NOT here.)*
- [ ] T033 Run the `quickstart.md` validation end-to-end (US1–US4 commands) and confirm SC-001..SC-007. The negative-scope review (SC-007 / FR-021 / FR-022) MUST be an **auditable checklist**, asserting each forbidden capability is absent individually: no template-engine integration, no `render`/rendering path, no typed-Vars validation runtime, no agreement-check/variant-resolution/hashing logic, no I/O (file/DB/network), no LLM call, no request-body assembly, no token counting, no output parsing.
- [ ] T034 [P] Update `docs/research/roadmap.md` / `.specify/memory/roadmap.md` ledger: mark spec 001 `in-progress` → `implemented` once T033 passes.

---

## Dependencies & Execution Order

### Phase / story order (sequential — this is a foundational spec)

1. **Setup (Phase 1)** → no deps.
2. **US1 layout (Phase 2)** → after Setup. **The foundation; blocks US2/US3/US4.** = MVP.
3. **US2 schema (Phase 3)** → after US1 (needs `schemas/` + fixtures dirs).
4. **US3 codegen (Phase 4)** → after US2 (needs the schema) + US1 (needs package dirs).
5. **US4 guardrails (Phase 5)** → FFI gate (T028) after US1; freshness gate (T029) after US3.
6. **Polish (Phase 6)** → after US4.

### Within-story parallel opportunities

- Setup: T002, T003 [P].
- US1: the 4 crate stubs (T005, T007, T008) + package skeletons (T009–T011) are [P] (different dirs); T006 waits on T005; T012–T014 serialize the wiring.
- US2: fixtures T017, T018 [P] after the schema (T015).
- US3: tool pins T020–T022 [P]; the 3 generations T023–T025 [P] after the schema + their pins.

### Critical path

T001 → T005 → T006 → T012 → T013 → T014 (US1 MVP) → T015 → T019 (US2) → T023–T026 → T027 (US3) → T029 → T031 (US4) → T033.

---

## Implementation Strategy

### MVP (US1 only)
Setup → US1 → **stop & validate**: a buildable polyglot workspace with correct dependency direction
is itself a demonstrable increment (the spine exists; nothing renders yet, by design).

### Incremental delivery
US1 (workspace) → US2 (schema + fixtures) → US3 (deterministic codegen) → US4 (CI guardrails). Each adds
a verifiable layer. US4 should not be deferred long — the FFI gate (T028) can land right after US1 to
protect the invariant early (its P1 rationale), even though the freshness half waits for US3.

### Notes
- `[P]` = different files/dirs, no incomplete dependency.
- Generated artifacts are committed and freshness-gated — never hand-edit them (T031 proves the gate).
- The schema contract currently lives at `specs/001-foundations/contracts/`; T015 promotes the
  implementation copy to `schemas/jsonschema/`.
- Commit after each task or logical group; stop at any checkpoint to validate.
