---
description: "Task list for spec 011 — auto-generated language API references"
---

# Tasks: Auto-generated language API references

**Input**: Design documents from `specs/011-autogen-api-refs/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/api-doc-ir.md, quickstart.md

**Organization**: Phased by the plan's work units. The API-doc IR contract is the linchpin (locked first); the three extractors are the main parallel set once it is locked.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on incomplete tasks)
- **[Story]**: US1 (accuracy without mirroring), US2 (complete+consistent coverage), US3 (readability)
- Exact file paths included.

## Implementation-open items (resolve during execution)

- **IO-1 — exact nightly date**: research R1 left the rustdoc-JSON toolchain as `nightly-<PINNED-DATE>`. Choose a specific recent `nightly-YYYY-MM-DD`, verify `rustdoc -Z unstable-options --output-format json` works on it, record its rustdoc-JSON `format_version`, and pin it. (T004)
- **IO-2 — Rust public-surface walk**: rustdoc JSON emits every `pub` item; the extractor MUST filter to the crate's intended public API = what `crates/prompting-press/src/lib.rs` re-exports, not every `pub` in a private module (memory-synthesis soft-watch). (T010)

---

## Phase 1: Setup & Foundations (Blocking Prerequisites)

**Purpose**: Pin toolchains, extract the shared helper, and LOCK the IR contract before any extractor/renderer is built.

- [x] T001 Pin TypeDoc `0.28.19` as a dev dependency of `docs/site` (package.json devDependencies) — do NOT add to any shipped package runtime deps (FR-011).
- [x] T002 [P] Pin griffe `2.1.0` as a dev/build tool runnable via `uv` for the Python extractor (a `docs/site/scripts/` uv invocation or a pinned `[dependency-groups]` entry); never a runtime dep of `packages/python` (FR-011).
- [x] T003 [P] Add a second, separate **nightly** Rust toolchain pin in `rust-toolchain-nightly.toml` (+ a `mise.toml` entry) used ONLY for rustdoc-JSON extraction; the stable `1.95.0` pin in `rust-toolchain.toml` stays primary and unchanged (research R1). Document the deliberate-upgrade contract in a comment.
- [x] T004 Resolve IO-1: pick the exact `nightly-YYYY-MM-DD`, verify `cargo +nightly-… rustdoc -p prompting-press -- -Z unstable-options --output-format json` produces `target/doc/prompting_press.json`, record its `format_version`, and write both into `rust-toolchain-nightly.toml` + research.md. **GATE**: the command emits JSON locally.
- [x] T005 Extract the jargon-strip + MDX-escape helper out of `docs/site/scripts/gen-shape-table.mjs` into `docs/site/scripts/lib/strip-jargon.mjs`; import it back into `gen-shape-table.mjs` (the one allowed touch of that file). **GATE**: regenerate `prompt-definition.mdx` and assert it is **byte-identical** to the committed version (`git diff --exit-code`).
- [x] T006 Lock the API-doc IR: confirm `contracts/api-doc-ir.md` is the authority and freeze the **canonical group set + order** (Prompt → RenderResult → GuardConfig → CheckReport/Finding → Composition/Message → Errors → Shape types) as a shared constant `docs/site/scripts/lib/api-groups.mjs` that all three extractors import. **GATE**: the group constant + IR field set are documented and importable.

**Checkpoint**: toolchains pinned, shared helper extracted (shape page byte-identical), IR contract + group order locked. Extractors can now be built in parallel.

---

## Phase 2 — User Story 1 (P1): Accuracy without manual mirroring — the three extractors + renderer + orchestrator + gate

**Goal**: A public-API change regenerates the right page automatically; a stale page fails CI. This is the MVP.

**Independent test**: edit a public doc comment in source → regenerate → page changes with no `.mdx` hand-edit; edit source without regenerating → freshness gate fails.

### The three extractors (parallel — each emits the IR independently)

- [x] T007 [P] [US1] `docs/site/scripts/extract-ts-api.mjs` — run TypeDoc `--json` over `packages/typescript/src/index.ts`; map its JSON to the API-doc IR; public surface = the module's exported symbols (FR-005); a symbol with no TSDoc comment → IR `doc: null` (FR-008); assign symbols to the canonical groups. **GATE**: emits valid IR covering the real TS public surface.
- [x] T008 [P] [US1] `docs/site/scripts/extract-python-api.py` — use griffe to walk `prompting_press`; filter to `prompting_press.__all__` (FR-005); docstring → IR `doc` (null if absent, FR-008); map to the canonical groups; run via the pinned `uv`/griffe. **GATE**: emits valid IR covering the real Python public surface.
- [x] T009 [P] [US1] `docs/site/scripts/extract-rust-api.mjs` — invoke the pinned nightly `rustdoc … --output-format json` over `crates/prompting-press`, read the JSON, **assert the recorded `format_version`** (fail loudly on mismatch, R5); map items to the IR; doc comment → `doc` (null if absent). **GATE**: emits valid IR for the Rust surface.
- [x] T010 [US1] Resolve IO-2 inside `extract-rust-api.mjs`: filter the rustdoc-JSON items to the crate's **public re-export set from `lib.rs`** (Prompt, PromptOverlay, RenderResult, GuardConfig, CheckReport, Finding, FindingKind, Composition, Message, ConsumerError, FieldError, `error::code`, and the re-exported PromptDefinition/PromptVariable/PromptVariant) — NOT every `pub` item. Re-exported shape types get IR `shapeRef` set (FR-010), not member expansion. **GATE**: IR contains exactly the public surface; no internal symbols; shape types carry `shapeRef`.

### The shared renderer

- [x] T011 [US1] `docs/site/scripts/lib/render-api-ref.mjs` — IR → MDX: AUTO-GENERATED frontmatter marker; one `##` per group; each symbol a `###` with a language-tagged signature fence + jargon-stripped/MDX-escaped doc (via `lib/strip-jargon.mjs`); `shapeRef` symbols render as a link to `prompt-definition.mdx` (FR-010); `doc: null` → **throw** an actionable error naming language+symbol (R6/FR-008); deterministic order from the IR. **GATE**: renders all three IRs to valid MDX; a `doc:null` IR makes it throw.

### The orchestrator + prebuild wiring

- [x] T012 [US1] `docs/site/scripts/gen-api-refs.mjs` — orchestrator: accept `--version` (default `latest`) + `--out` (default the live `reference/` dir) (FR-016, R8); for each language run extract → render → write `reference/<lang>.mdx`. **GATE**: `node gen-api-refs.mjs` writes all three pages.
- [x] T013 [US1] Wire `gen-api-refs.mjs` into `docs/site/package.json` `prebuild` alongside `gen-shape-table.mjs`. **GATE**: `pnpm -C docs/site build` regenerates all three language pages and the Astro build is clean.

### The freshness gate (the "can't rot" guarantee)

- [x] T014 [US1] `docs/site/scripts/check-api-refs-fresh.sh` mirroring `schemas/scripts/codegen-check.sh`: regenerate the three pages, `git diff --exit-code` against committed (twice-run determinism, SC-003); fail naming any drifted page; the orchestrator's `doc:null` throw also fails here (FR-008). **GATE**: gate passes clean on fresh output, fails on a stale page, fails on an undocumented public symbol.
- [x] T015 [US1] Wire the freshness gate into CI (the same place `schemas:codegen-check` runs in `.github/workflows/ci.yml`). **GATE**: CI runs the gate; a committed-but-stale page fails the build.

**Checkpoint (US1 / MVP complete)**: all three pages generate from source, are wired into the build, and a freshness gate prevents drift. SC-001/002/003/006/007 satisfied.

---

## Phase 3 — User Story 2 (P2): Complete + consistent coverage

**Goal**: every public symbol present per language; the three pages parallel; no internal leakage.

**Independent test**: diff each generated page's symbol set against `lib.rs` re-exports / `index.ts` exports / `__all__`.

- [x] T016 [US2] Coverage assertion: a script/test that compares each generated page's symbol set to the language's public surface (Rust lib.rs re-exports, TS index.ts exports, Python `__all__`) and fails on any missing public symbol or any leaked internal symbol (FR-005, SC-004). **GATE**: coverage check passes for all three.
- [x] T017 [US2] Cross-language consistency check: assert the three pages use the same canonical group order and that a group present in one language is present (or explicitly empty) in the others (FR-009). **GATE**: structural parity verified.

**Checkpoint (US2)**: full, parallel, leak-free coverage. SC-004 satisfied.

---

## Phase 4 — User Story 3 (P3): Reads well (full-autogen, no curated layer)

**Goal**: conceptual framing comes from the source doc comments themselves; no separate prose layer to drift.

**Independent test**: confirm primary types carry conceptual prose sourced from their doc comments; confirm there is NO sidecar prose store.

- [ ] T018 [US3] (DEFERRED — quality polish, non-blocking) Audit the generated pages for readability: where a primary type reads thin, enrich the **source doc comment** (in `crates/`, `packages/typescript/src/index.ts`, `packages/python/…`) — NOT the generated page or any sidecar (full-autogen, FR-014). Also clean up the pre-existing `prompting-press-core` private-item rustdoc-link warnings surfaced by extraction. Regenerate. **GATE**: primary types have conceptual prose; no sidecar prose file exists.

**Checkpoint (US3)**: pages are readable and drift-proof (prose lives in source).

---

## Phase 5: Migration & Cleanup (Cross-cutting)

**Purpose**: flip the three reference pages from hand-written to generated; verify the boundary.

- [x] T019 Replace the committed hand-written `reference/{rust,python,typescript}.mdx` with the generated output (commit the generated pages; they now carry the AUTO-GENERATED marker). **GATE**: the three pages match a fresh generation (gate green).
- [x] T020 [P] Verify Principle II/III boundary: `mise exec -- moon run ci:check-ffi` passes and no extractor toolchain (TypeDoc/griffe/nightly-rustdoc) appears in any crate `Cargo.toml [dependencies]` or package runtime deps (FR-011, SC-006). **GATE**: ci:check-ffi green; grep confirms dev-only.
- [x] T021 [P] Sweep for now-stale hand-maintenance notes (e.g. any "edit this page by hand" guidance, the sidebar/AGENTS references) and remove them; confirm the version/output-path param shape is stable for spec 012 to call (R8). **GATE**: no stale hand-maintenance guidance remains.

---

## Dependencies & order

- **Phase 1 blocks everything** (IR lock + shared helper + toolchain pins).
- **T007/T008/T009 are parallel [P]** once T006 locks the IR; T010 extends T009.
- **T011 (renderer)** needs the IR (T006) but not the extractors; can start in parallel with extractors using a hand-written sample IR, then validated against real extractor output.
- **T012/T013** need the extractors + renderer.
- **T014/T015 (gate)** need T012.
- **Phase 3 (US2)** needs Phase 2 pages generating.
- **Phase 4 (US3)** needs pages generating; iterative.
- **Phase 5** is the final flip + boundary verification.

## MVP scope

**Phase 1 + Phase 2 (US1)** = the MVP: three generated pages + freshness gate. US2/US3/cleanup are quality increments on top.

## Parallel opportunities

- T002 + T003 (toolchain pins, different files).
- T007 + T008 + T009 (the three extractors — the main parallel set, once T006 locks the IR).
- T020 + T021 (independent cleanup checks).
