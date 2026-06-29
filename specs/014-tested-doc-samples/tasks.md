---
description: "Task list for spec 014 — tested doc samples + consumer sample apps"
---

# Tasks: Tested Documentation Samples & Consumer Sample Apps

**Input**: Design documents from `specs/014-tested-doc-samples/`

**Prerequisites**: spec.md, plan.md, research.md, data-model.md, contracts/{injection-marker,coverage-audit,sample-app}.md, quickstart.md

**Organization**: Phased by the plan's four work units — WU-A (injection harness + coverage audit), WU-B (assertion promotion), WU-C (consumer sample apps), WU-D (CI wiring + launch-flip). Foundations first (the marker grammar + the two trees), then A/C in parallel, B stacked on A, D wires it all.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no incomplete-dep)
- **[Story]**: US1 (broken sample caught), US2 (full coverage), US3 (output assertions), US4 (consumer apps E2E)

## Implementation-open items (settle during execution)

- **IO-1 — anchor convention** (contracts/injection-marker): the in-source named-region marker the MDX `#anchor` resolves to (e.g. `// #region render` / a comment sentinel). Pick one that is valid in all three languages' tooling and is stripped from the injected block. Resolve in T002.
- **IO-2 — assertion-promotion site** (research §3): decide whether `// =>`/`# =>` are promoted by a transform at inject time, or authored directly as assertions in the source file with the `// =>` kept as a trailing human-readable echo. Lean: author as real assertions, keep the comment as documentation. Resolve in T010.
- **IO-3 — samples/ moon topology** (research §5): one moon project for `samples/` vs one-per-app. Lean one-per-language-app (clean independent gates, FR-017). Resolve in T013.
- **IO-4 — migration batching**: ~108 blocks is large. Migrate per-page or per-guide in reviewable batches; the coverage audit (T004) is the completeness oracle, not a hand count.

---

## Phase 1: Foundations (Blocking Prerequisites)

**Purpose**: the two trees + the marker grammar + the coverage classifier — everything else builds on these.

- [ ] T001 Create the two physical trees with READMEs stating their distinct roles (FR-003a/FR-013): `docs/site/samples/{rust,python,typescript}/` (per-snippet tested sources injected into pages — README: "snapshot-injected into MDX; the source of truth for doc code blocks") and top-level `samples/{rust,python,typescript}/` (whole consumer apps — README: "complete consumer apps; depend on the library via the package boundary"). **GATE**: both trees exist with `.gitkeep` + README; no overlap.
- [ ] T002 [US2] (resolve IO-1) `docs/site/scripts/lib/sample-markers.mjs` — the injection-marker grammar per contracts/injection-marker.md: the MDX `{/* INJECT: <path>#<anchor> */}`…`{/* END INJECT */}` pair and the in-source anchor convention; a parser that locates markers in an MDX file and extracts the anchored region from a source file. Reuse the spec-011 MDX walk where possible. **GATE**: unit checks — round-trips a known marker+source to the expected fenced block; handles a `<Tabs><TabItem>`-nested marker (FR-010); idempotent.
- [ ] T003 [US2] `docs/site/scripts/lib/runnable-blocks.mjs` — the coverage classifier per contracts/coverage-audit.md: walk an MDX tree, enumerate fenced code blocks, classify each runnable (`rust`/`python`/`py`/`typescript`/`ts`/`javascript`/`js`) vs definition-only (`yaml`/`json`/`toml`) vs fragment-only (FR-012). **GATE**: on a fixture MDX with one of each, classifies correctly.

**Checkpoint**: trees + marker grammar + classifier exist; harness + audit can build on them.

---

## Phase 2 — WU-A: injection harness + coverage audit (US1/US2)

**Goal**: real source files injected into MDX at build; a broken sample fails; coverage audit reports gaps.

**Independent test**: break a migrated sample (wrong method) → `docs:test-samples` fails citing the source file:line; add an MDX runnable block with no marker → coverage audit reports it.

- [ ] T004 [US2] `docs/site/scripts/check-sample-coverage.mjs` — walk the MDX tree (T003), report every runnable block lacking an injection marker (i.e. not backed by a tested source). Distinguish definition-only blocks (not required). Exit non-zero with the offending page+block on any gap (FR-008). **GATE**: reports 0 on a fully-migrated fixture; reports the block on an unmarked one.
- [ ] T005 [US1] `docs/site/scripts/inject-samples.mjs` — replace each MDX marker's body with the anchored source content at prebuild time; mark injected content with a generator comment (FR-009); run in the SAME prebuild stage as `gen-api-refs.mjs` (after it). Idempotent (re-inject → no diff). **GATE**: prebuild injects all markers; second run = zero diff; injected blocks carry the generator comment (SC-007).
- [ ] T006 [US1] Per-language doc-sample test runners (FR-004): wire `cargo test`(+`--doc`) over `docs/site/samples/rust/**`, `pytest` over `docs/site/samples/python/**`, `tsc --noEmit` + `node:test`/vitest over `docs/site/samples/typescript/**`. Each runs independently (SC-003). Errors cite the source file:line (FR-011). **GATE**: each language suite runs alone and a deliberately broken sample fails citing its file:line (SC-002).
- [ ] T007 [US2] (IO-4) Migrate the ~108 existing MDX code blocks into `docs/site/samples/**` tested sources + markers, in reviewable batches (per guide/page). Fragment-only snippets → compile-check-only (FR-012). The coverage audit (T004) is the completeness oracle. **GATE**: `check-sample-coverage` reports 0 uncovered runnable blocks across all three languages (SC-001).

**Checkpoint (WU-A)**: every doc code block is source-backed, injected, and tested; the gate blocks a broken sample.

---

## Phase 3 — WU-B: assertion promotion (US3)

**Goal**: shown outputs are verified; a changed output fails the build.

**Independent test**: change a `// =>` value without changing code → the test fails with an assertion mismatch.

- [ ] T010 [US3] (resolve IO-2) In the migrated doc-sample sources, promote ALL `// =>` / `# =>` expected-output annotations to real assertions (`assert_eq!` / `==` / `expect().toBe`) (FR-007). **GATE**: a test asserts each shown non-hash output; flipping a shown value fails.
- [ ] T011 [US3] Hash-field exemption (FR-007 edge case): `template_hash`/`render_hash`/`templateHash`/`renderHash` shown values are verified by format+length only (64-char lowercase hex `/^[0-9a-f]{64}$/`), never exact value. **GATE**: a hash-field sample passes on any valid 64-hex and fails on a non-hex/short value; not on exact match.

**Checkpoint (WU-B)**: every shown output is contractual; rot in a value fails CI.

---

## Phase 4 — WU-C: consumer sample apps (US4)

**Goal**: one realistic CLI per language under `samples/`, full-surface, building on local deps, with behavioral + coverage tests.

**Independent test**: `samples:test` builds + tests all three apps on local deps; break a consumed library API → the gate fails citing the app file.

- [ ] T013 [US4] (resolve IO-3) `samples/rust/<app>/` — a realistic CLI (Cargo `path` dep on `../../crates/prompting-press`, FR-015) that loads a prompt from YAML, validates typed vars, renders default + a named variant, composes a 2-message prompt, runs `check()`, prints the guard + provenance hashes, and handles a deliberately-triggered error; the "hand to provider" step is printed/stubbed (FR-018). **GATE**: `cargo run` works; `cargo test` (behavioral) passes.
- [ ] T014 [P] [US4] `samples/python/<app>/` — the same realistic CLI in Python (`uv` path/editable dep on `packages/python`, FR-015), same feature walk, stubbed provider step. **GATE**: app runs; `pytest` passes; builds without the Rust/TS toolchains (FR-017).
- [ ] T015 [P] [US4] `samples/typescript/<app>/` — the same realistic CLI in TS (pnpm `workspace:*` dep on `packages/typescript`, FR-015), same feature walk, stubbed provider step. **GATE**: app runs; `node:test`/vitest passes; builds independently (FR-017).
- [ ] T016 [US4] Per-app feature-coverage suite (FR-014a): in each app, an explicit test that walks every feature in the FR-014 surface list and asserts on each, so SC-009 is provable by inventory. **GATE**: the coverage suite asserts on construct/validate/render/variant/compose/check/guard/hashes/error — no listed feature unexercised.
- [ ] T017 [US4] (IO-3) The `samples:test` gate: moon task(s) building + testing all three apps; a library public-API change an app uses fails it (SC-010). **GATE**: `moon run samples:test` builds+tests all three; breaking a consumed API fails citing the app file.

**Checkpoint (WU-C)**: a real consumer app per language proves the library end-to-end; the gate catches consumer-facing breaks.

---

## Phase 5 — WU-D: CI wiring + launch-flip (cross-cutting)

- [ ] T020 Wire the doc-sample gate into the docs-publish workflow (FR-002): `docs:test-samples` + `docs:check-sample-coverage` run in `docs.yml` (already mise-bootstrapped by spec 011, so rust/uv/node are present); docs MUST NOT publish if a sample test fails. **GATE**: docs.yml runs both; a broken sample blocks the deploy.
- [ ] T021 Wire the sample-app gate into `ci.yml` (FR-016): `samples:test` as a CI job (3 independent language legs, FR-017). **GATE**: ci.yml runs `samples:test`; a consumed-API break fails the PR.
- [ ] T022 Document the launch-flip (FR-019): a short doc (e.g. `samples/README.md` + a release-checklist note) recording the single post-publish step — flip each app's manifest from local/workspace deps to the published-version constraint; the apps then double as the published-package smoke test. **GATE**: the flip step is documented; pre-publish, no app manifest references a published version (SC-011).

---

## Phase 6: Verification

- [ ] T030 Run the full quickstart.md locally: break a doc sample → doc-sample gate fails citing file:line; coverage audit = 0 gaps; change a `// =>` value → assertion fails; hash field passes on valid hex; `samples:test` builds+tests all three apps on local deps; break a consumed API → fails. **GATE**: every quickstart check passes.
- [ ] T031 [P] No-published-runtime-dep proof (SC-006): inspect the published library manifests — no example/harness/app artifact leaks into `prompting-press`/`-py`/`-node` published packages. **GATE**: published manifests unchanged; samples are dev-only/consumer-side.
- [ ] T032 [P] Version-agnostic proof (FR-006/SC-004): confirm the doc-sample sources + lockfiles live inside the docs tree so a spec-012 frozen snapshot pins the matching library version (git-branch-implicit; no extra manifest). **GATE**: a frozen-tree dry-run tests against the snapshot's lockfile-pinned version.

---

## Dependencies & order

- **Phase 1 blocks all** (trees + marker grammar + classifier).
- WU-A (Phase 2) and WU-C (Phase 4) are largely independent (different trees, gates) → parallel.
- WU-B (Phase 3) stacks on WU-A (assertions live in the injected sources).
- WU-D (Phase 5) wires whatever A/B/C produce.
- Phase 6 is final verification.

## Parallel opportunities

- T014 + T015 (Python + TS apps, different trees) once T013 establishes the app shape.
- WU-A and WU-C can proceed concurrently.
- T031 + T032 (independent verification checks).

## MVP scope

**Phase 1 + WU-A (one language)** = the core anti-rot gate catching a broken sample — already valuable. WU-B adds output-correctness; WU-C adds the E2E consumer proof; WU-D wires the gates into publish/CI.

## Note on pre-publish testability

Everything here is testable NOW on local/workspace deps (FR-015) — nothing waits for publish except the single FR-019 dependency-source flip (T022), which is documented but not executed until launch.
