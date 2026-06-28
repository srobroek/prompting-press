---
description: "Task list for 006 — Conformance corpus + cross-language hardening"
---

# Tasks: Conformance corpus + cross-language hardening

**Input**: Design documents from `specs/006-conformance-corpus/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D6), data-model.md, contracts/corpus-format.md, quickstart.md

**Tests**: This feature's *deliverable is the test corpus itself* — the three runners ARE the tests. No
separate TDD test tasks are generated (none requested in the spec); each runner task produces an
executable test that asserts parity. The "golden generator" is the only non-test code.

**Organization**: by user story (US1 marshaling parity, US2 schema round-trip, US3 CI gate). US1/US2 are
both P1; US1 is the MVP. US3 (P2) integrates US1+US2 (it runs their runners) — an accepted cross-story
dependency for a gate.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no incomplete-task dependency)
- **[Story]**: US1 / US2 / US3 (setup/foundational/polish carry no story label)

## Path Conventions

Polyglot workspace (per plan.md): shared corpus at `conformance/`; Rust runner in
`crates/prompting-press/tests/`; Python runner in `packages/python/tests/`; TS runner in
`packages/typescript/test/`; CI glue in `ci/moon.yml` + `scripts/ci/` + `.github/workflows/ci.yml`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: create the shared corpus skeleton — the single source of truth all runners read.

- [ ] T001 Create the `conformance/` directory structure (`conformance/marshaling/`, `conformance/schema/`, `conformance/schema/yaml/`) and write `conformance/README.md` documenting the corpus contract (summarize `specs/006-conformance-corpus/contracts/corpus-format.md`: what the corpus guards, the fixture families, runner obligations, the golden/regen rule, the scope guard that render parity is NOT tested here).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: author the fixtures and build the golden generator. BLOCKS US1 (runners need fixtures +
goldens) and US2 (runners need the schema manifest).

**⚠️ CRITICAL**: No runner (US1/US2) can pass until goldens exist (T006) and the manifest exists (T003).

- [ ] T002 [P] Author the five marshaling fixture inputs in `conformance/marshaling/{date,decimal,nested-model,null-undefined-none,int-vs-float}.json` per `data-model.md` — each with `case`, a spec-001 `definition` whose `body` references the input fields, and an `input` map of `{type,value}` typed descriptors (date→ISO-8601 string, decimal→decimal-as-string, nested→typed object/array, null-undefined-none→one `null`+one `absent`+one present field, int-vs-float→an `int` (`1`) and a **fractional** `float` (`2.5`) — NOT `1.0`, which is JS-unrepresentable and excluded per spec Edge Cases). Leave `expected` as an empty placeholder (filled by T006).
- [ ] T003 [P] Author `conformance/schema/manifest.json` mapping each existing `schemas/jsonschema/fixtures/valid/*.json` (verdict `accept`) and `schemas/jsonschema/fixtures/invalid/*` (verdict `reject`) to `{path, form:"json", verdict, note}` per `data-model.md`; add one valid + one invalid **YAML twin** under `conformance/schema/yaml/` (form `yaml`) so the `load_yaml` path is covered (FR-011, D6).
- [ ] T004 Create the shared Rust test-support module `crates/prompting-press/tests/common/mod.rs`: the test-only `RawVars(serde_json::Value)` newtype (`Serialize` delegating to the inner value + a no-op `garde::Validate` impl, `Context = ()`); fixture deserialization structs (`MarshalingFixture`, `TypedValue`, `Expected`, `SchemaEntry`); the `type`→`serde_json::Value` builder (D2 table); and a helper that loads a fixture's `definition` into a `Registry` via `load_json` (D4). Zero engine logic — Serialize delegates, Validate is empty.
- [ ] T005 Implement the golden generator as an `#[ignore]`d integration test `crates/prompting-press/tests/conformance_goldens.rs` (`mod common;`): for each `conformance/marshaling/*.json`, build `RawVars` from the typed `input`, call `prompting_press::render(&reg, name, &RawVars(..), variant, &GuardConfig::default())`, and write `expected.{text, template_hash, render_hash}` back into the fixture file (D3). (depends on T002, T004)
- [ ] T006 Run the generator (`cargo test -p prompting-press --test conformance_goldens -- --ignored`) to populate the goldens in all five marshaling fixtures; commit the filled fixtures. (depends on T005)

**Checkpoint**: corpus fixtures authored and goldens committed; the schema manifest exists. Runners can now be built.

---

## Phase 3: User Story 1 - Marshaling parity across all three bindings (Priority: P1) 🎯 MVP

**Goal**: prove the same logical input through each binding yields identical rendered text + identical `template_hash`/`render_hash`, over dates / Decimal / nested models / null-undefined-None / int-vs-float.

**Independent Test**: run the three marshaling runners over `conformance/marshaling/*`; each renders every fixture's native-constructed input through its real public render path and asserts equality to the committed golden — so all three agree transitively (SC-001/002).

- [ ] T007 [P] [US1] Rust marshaling runner `crates/prompting-press/tests/conformance_marshaling.rs` (`mod common;`): iterate `conformance/marshaling/*.json`, build `RawVars` from each `input`, render via `prompting_press::render`, and assert `result.text`, `result.template_hash`, `result.render_hash` equal the fixture's golden; on mismatch fail naming case + divergence kind (FR-014). (depends on T004, T006)
- [ ] T008 [P] [US1] Python marshaling runner `packages/python/tests/test_conformance_marshaling.py` (pytest): load each fixture, construct the native Vars from the `type` tags (`datetime.fromisoformat`, `Decimal(str)`, nested dict/list, `None`/absent, int/float) per the D2 table, load the `definition` via `Registry.load_json`, render via `prompting_press.render` (use the binding's static-data render path — consult the signature in `crates/prompting-press-py/src/render.rs`), and assert `.text`/`.template_hash`/`.render_hash` equal the golden.
- [ ] T009 [P] [US1] TS marshaling runner `packages/typescript/test/conformance.marshaling.test.mjs` (`node:test`): load each fixture, construct the native Vars from the `type` tags (`new Date(value)`, decimal-as-string, nested object/array, `null` vs omitted key, integer vs float) per the D2 table, `Registry.loadJson` the `definition`, render via the facade's static-data render path (consult `packages/typescript/src/index.ts`), and assert `.text`/`.templateHash`/`.renderHash` equal the golden.
- [ ] T010 [US1] Confirm cross-binding marshaling parity: all three runners green against the SAME committed goldens (transitive cross-check, SC-001), and verify the seeded-divergence check from `quickstart.md` fails the relevant runner naming binding+case+kind (SC-004). (depends on T007, T008, T009)

**Checkpoint**: marshaling parity proven across Rust + Python + TypeScript for all five hard cases — the MVP.

---

## Phase 4: User Story 2 - Schema round-trip parity across all three bindings (Priority: P1)

**Goal**: prove a schema-valid document is accepted and a schema-invalid one rejected identically across all three bindings' own loaders, including the YAML path.

**Independent Test**: run the three schema runners over `conformance/schema/manifest.json`; each loads every doc through its own `load_json`/`load_yaml` and asserts the expected accept/reject; all three verdicts agree (SC-003).

- [ ] T011 [P] [US2] Rust schema runner `crates/prompting-press/tests/conformance_schema.rs` (`mod common;`): iterate `conformance/schema/manifest.json`, read each doc, call `Registry::load_json`/`load_yaml` per `form`, and assert `Ok` for `accept` and a `ConsumerError` (no panic, no partial load) for `reject` (FR-009/010). (depends on T003, T004)
- [ ] T012 [P] [US2] Python schema runner `packages/python/tests/test_conformance_schema.py`: per manifest entry, `Registry.load_json`/`load_yaml`; assert it returns for `accept` and raises `LoadError` (structured, no crash) for `reject`.
- [ ] T013 [P] [US2] TS schema runner `packages/typescript/test/conformance.schema.test.mjs`: per manifest entry, `Registry.loadJson`/`loadYaml`; assert success for `accept` and a thrown `LoadError`/`PromptingPressError` (structured, no crash) for `reject`.
- [ ] T014 [US2] Confirm schema verdict parity across all three loaders for 100% of manifest entries, including the YAML twins (FR-011, SC-003). (depends on T011, T012, T013)

**Checkpoint**: schema round-trip parity proven across all three bindings, JSON + YAML.

---

## Phase 5: User Story 3 - Run the corpus as a CI gate, locally reproducible (Priority: P2)

**Goal**: wire the corpus as a merge-gating, locally-reproducible check that runs all three runners (incl. the Rust consumer leg) and fails the build on any divergence.

**Independent Test**: `mise exec -- moon run ci:conformance` on a clean checkout runs all runners and passes; a seeded divergence fails the same command; CI invokes it. Integrates US1+US2 (runs their runners).

- [ ] T015 [US3] Add `scripts/ci/conformance.sh`: build the Python extension + Node addon as the existing `test-python`/`test-node` scripts do, then run `cargo test -p prompting-press --test conformance_marshaling --test conformance_schema` (NOT the `conformance_goldens` test — it is `#[ignore]`d and regeneration-only; omit it from the gate so CI never even compiles a regen path into the run) plus the Python and TS conformance runners; exit non-zero on any failure; locally reproducible (FR-012/013, D5). Mirror the structure of `scripts/ci/test-node.sh`.
- [ ] T016 [US3] Add a `conformance` task to `ci/moon.yml` (`command: 'bash scripts/ci/conformance.sh'`, `options: {runFromWorkspaceRoot: true, cache: false}`) with a description citing FR-012/015, matching the existing `test-python`/`test-node` task shape.
- [ ] T017 [US3] Wire the conformance gate into `.github/workflows/ci.yml` as a job (or step) that runs `mise exec -- moon run ci:conformance`, ensuring the **Rust consumer leg** runs — closing the gap that `cargo test -p prompting-press` runs nowhere in CI today (FR-015). Follow the existing `test-node` job's checkout/mise/pnpm setup.
- [ ] T018 [US3] Add a documented goldens-regeneration entry point (a `conformance:regen` moon task, or a quickstart command) invoking the `#[ignore]`d generator test, so a deliberate golden change is a reviewable, reproducible step (D3); confirm it is NOT run by the CI gate.

**Checkpoint**: the corpus is an enforced, locally-reproducible CI gate covering all three bindings.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: documentation, scope-guard verification, end-to-end validation.

- [ ] T019 [P] Finalize `conformance/README.md` and cross-link `quickstart.md` ↔ `data-model.md` ↔ `contracts/corpus-format.md`; document the `type`-tag vocabulary and how to add a fixture.
- [ ] T020 Verify the scope guards (SC-006): `mise exec -- moon run ci:check-ffi` green; `git status crates/prompting-press-core/tests/fixtures/` shows the spec-002 render fixtures byte-unchanged; the golden set is bounded to the five named cases (no render-parity creep, FR-016).
- [ ] T021 Run the full `quickstart.md` validation on a clean state: `mise exec -- moon run ci:conformance` green; all five marshaling cases + all schema verdicts pass in all three bindings; the seeded-divergence check fails as expected (SC-001–SC-007).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies — start immediately.
- **Foundational (Phase 2)**: depends on Setup. BLOCKS US1 and US2 (goldens + manifest must exist). T004→T005→T006 are sequential (support module → generator → run); T002/T003 are parallel authoring.
- **US1 (Phase 3)**: depends on Foundational (T006 goldens, T004 support). The three runners (T007–T009) are parallel; T010 gates on all three.
- **US2 (Phase 4)**: depends on Foundational (T003 manifest, T004 support). The three runners (T011–T013) are parallel; T014 gates on all three. Independent of US1 (different files, different fixtures).
- **US3 (Phase 5)**: depends on US1 + US2 (it runs their runners). T015→T016→T017 roughly sequential; T018 parallel-ish.
- **Polish (Phase 6)**: depends on US1–US3 complete.

### User Story Dependencies

- **US1 (P1, MVP)**: after Foundational; no dependency on US2/US3.
- **US2 (P1)**: after Foundational; independent of US1 (separate runner files + fixture family).
- **US3 (P2)**: after US1 + US2 — it wires their runners into the gate (accepted integration dependency).

### Parallel Opportunities

- Foundational authoring: **T002 ∥ T003** (different files).
- US1 runners: **T007 ∥ T008 ∥ T009** (Rust / Python / TS — different files).
- US2 runners: **T011 ∥ T012 ∥ T013**.
- US1 and US2 can proceed in parallel after Foundational (disjoint files).
- Polish: **T019 ∥ T020** are independent of each other.

---

## Parallel Example: User Story 1

```bash
# After Foundational (goldens committed), launch the three marshaling runners in parallel:
Task: "Rust marshaling runner in crates/prompting-press/tests/conformance_marshaling.rs"
Task: "Python marshaling runner in packages/python/tests/test_conformance_marshaling.py"
Task: "TS marshaling runner in packages/typescript/test/conformance.marshaling.test.mjs"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Phase 1 Setup → Phase 2 Foundational (fixtures + generator + goldens).
2. Phase 3 US1: the three marshaling runners.
3. **STOP and VALIDATE**: all three render the five hard cases to identical golden text + hashes (SC-001/002).

### Incremental Delivery

1. Setup + Foundational → corpus + goldens ready.
2. US1 (marshaling parity) → validate → MVP.
3. US2 (schema round-trip) → validate (independent of US1).
4. US3 (CI gate) → wire US1+US2 into an enforced, locally-runnable gate.
5. Polish → scope-guard verification + full quickstart run.

---

## Notes

- [P] = different files, no incomplete-task dependency.
- The Rust support module (`tests/common/mod.rs`) is shared by the generator (T005) and both Rust runners
  (T007, T011) via `mod common;` — build it once in Foundational.
- The golden generator (T005) is an `#[ignore]`d test so CI never regenerates; goldens change only via the
  documented regen step (T018) and are reviewed in PR (the regression tripwire working as intended).
- Scope guards are load-bearing: NO render-parity fixtures (FR-016), NO engine logic in runners (C-02),
  spec-002 render fixtures untouched (SC-006). T020 verifies these explicitly.
- Commit after each task or logical group. Use `git commit -F <file>` (the `-m`/`-n` precommit-gate
  false-positive); the APM working-tree drift (`.claude/agents/*.md` + `apm.lock.yaml`) stays OUT of every
  commit.
