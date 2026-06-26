---
description: "Task list for spec 002 — Engine kernel (prompting-press-core)"
---

# Tasks: Engine kernel (`prompting-press-core`)

**Input**: Design documents from `specs/002-engine-kernel/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D8), data-model.md, contracts/kernel-api.md, quickstart.md

**Tests**: INCLUDED. The spec's Success Criteria are verification-driven (determinism SC-001, soundness
SC-002/003, mutation-checks SC-006, exclusion SC-008) and quickstart.md enumerates ~25 concrete
scenarios. Test tasks are therefore first-class, organized per user story.

**Organization**: Tasks grouped by user story (US1 render+provenance P1 → US2 agreement analysis P2 →
US3 provenance/guard P3), after Setup + a Foundational phase that performs the load-bearing generated-
shape relocation (research D6) and kernel scaffolding that blocks all stories.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on incomplete tasks)
- **[Story]**: US1 / US2 / US3 (story-phase tasks only)
- Exact file paths included. Run Rust commands under `mise exec --`.

## Conventions / guardrails (from memory + plan)

- Kernel MUST stay FFI-free (C-02) — `ci:check-ffi` gate. New deps `minijinja`, `sha2` are pure-Rust (research D7); verify.
- Agreement analysis + guard expansion are PURE — never mutate template/values/output (FR-018, FR-023).
- No `vars_hash`; provenance is data on the return value (C-05).
- Do NOT hand-edit generated files; the kernel CONSUMES the relocated generated shape.
- `rm` is blocked by a buggy bash-3.2 hook — use `git mv` / `git rm` (git plumbing), not bare `rm`.
- Pushes via `dgit`; `cargo`/`moon` under `mise exec --`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Pin dependencies and confirm the FFI gate tolerates them before any code is written.

- [ ] T001 Add kernel dependencies to `crates/prompting-press-core/Cargo.toml`: `minijinja = { version = "2.21", default-features = false, features = ["builtins", "deserialization", "serde", "std_collections", "adjacent_loop_items"] }` and `sha2` (workspace-pin `sha2` in root `Cargo.toml [workspace.dependencies]`, reference via `{ workspace = true }`). `macros`/`multi_template` deliberately OFF (FR-002 mechanism); `adjacent_loop_items` KEPT so loops have no gaps; `debug` off (research D1 + critique E1).
- [ ] T002 Run `mise exec -- cargo build -p prompting-press-core` and `mise exec -- moon run ci:check-ffi` to confirm the new deps compile and pull no `pyo3`/`napi` (SC-007 / research D7). Record `cargo tree -p prompting-press-core -i pyo3` (expect "nothing depends on").

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Relocate the generated `PromptDefinition` shape into the kernel (FR-027; kernel must not
depend on the consumer — C-01/C-02) and scaffold the kernel module tree + error type. **No user story
can begin until this is done** — every operation consumes the shape and returns `KernelError`.

**⚠️ CRITICAL**: The relocation (T003–T009) is one atomic logical change; keep the freshness gate green throughout.

### Generated-shape relocation (research D6 blast radius)

- [ ] T003 `git mv crates/prompting-press/src/generated.rs crates/prompting-press-core/src/generated.rs` and `git mv crates/prompting-press/src/generated crates/prompting-press-core/src/generated` (moves `generated/prompt_definition.rs` + `generated/README.md`). Update the doc-comment in `crates/prompting-press-core/src/generated.rs` to reference the new codegen-script path (T005).
- [ ] T004 `git mv crates/prompting-press/scripts/codegen.sh crates/prompting-press-core/scripts/codegen.sh`. In the moved script, repoint `OUT="crates/prompting-press-core/src/generated/prompt_definition.rs"`, update `REPO_ROOT` resolution (now `../../..` from `crates/prompting-press-core/scripts/`), and update the static do-not-edit HEADER's "Regenerate with:" line to the new script path.
- [ ] T005 Move the moon `codegen` task from `crates/prompting-press/moon.yml` to `crates/prompting-press-core/moon.yml`: command `bash crates/prompting-press-core/scripts/codegen.sh`, inputs repointed to the kernel script path, `outputs: ['/crates/prompting-press-core/src/generated/prompt_definition.rs']`, `options.runFromWorkspaceRoot: true`. Add a `build: { deps: ['codegen'] }` to the kernel moon.yml; remove the now-orphaned `codegen` task + `build.deps` from the consumer moon.yml.
- [ ] T006 Wire the kernel modules in `crates/prompting-press-core/src/lib.rs`: add `pub mod generated;` and `pub use generated::prompt_definition::PromptDefinition;` (+ supporting types). Update the crate docstring to note it now hosts the generated input-contract shape (FR-027).
- [ ] T007 Update the consumer `crates/prompting-press/src/lib.rs`: delete `pub mod generated;` and re-point the re-exports to `pub use prompting_press_core::generated::prompt_definition;` / `pub use prompting_press_core::generated::prompt_definition::PromptDefinition;` (consumer keeps the same public surface, now sourced from the kernel). Confirm no `crates/prompting-press/src/generated.rs` remains.
- [ ] T008 Update the freshness gate `schemas/moon.yml` `codegen-check` task: change dep `prompting-press:codegen` → `prompting-press-core:codegen` and the input path `/crates/prompting-press/src/generated/prompt_definition.rs` → `/crates/prompting-press-core/src/generated/prompt_definition.rs`.
- [ ] T009 Verify the relocation end-to-end: `mise exec -- cargo build --workspace`, `mise exec -- moon run prompting-press-core:codegen` (regenerates in place), and `mise exec -- moon run schemas:codegen-check` (freshness gate GREEN at the new path). Update path mentions in `crates/prompting-press/README.md` + `crates/prompting-press-core/README.md` (+ `src/generated/README.md` regen command).

### Kernel scaffolding (shared by all stories)

- [ ] T010 Create `crates/prompting-press-core/src/error.rs` with `KernelError` enum: `UnknownVariant { requested: String }`, `ExcludedFeature { detail: String }`, `Parse { detail: String }`, `UndefinedVariable { name: String }`, `Render { detail: String }` (data-model §KernelError, FR-028). Derive `Debug` + `std::error::Error` + `Display`; re-export from `lib.rs`. NO normalization to `[{field,code,message}]` here (that is the consumer's job — C-06).
- [ ] T011 Create `crates/prompting-press-core/src/engine.rs` skeleton: a private `build_environment()` that constructs a `minijinja::Environment` with `set_undefined_behavior(UndefinedBehavior::Strict)` (research D3) and the minimal-feature config; document that `macros`/`multi_template` being off makes excluded features parse errors (FR-002, research D1/D4). No render logic yet.
- [ ] T012 [P] Create the test fixtures dir `crates/prompting-press-core/tests/fixtures/` and a small fixture loader/helper module that reads `(template, values) → expected` cases AND reuses spec-001 schema fixtures (`schemas/jsonschema/fixtures/valid/*.json`) as `PromptDefinition` inputs. Design it so a CI grep over fixtures does not match its own `{{ }}`-containing corpus (spec-001 self-referential-string lesson).

**Checkpoint**: Kernel builds, generated shape lives in & is consumed by the kernel, freshness gate green, `KernelError` + `Environment` builder + fixtures exist. User stories can begin.

---

## Phase 3: User Story 1 — Render + content-addressed provenance (Priority: P1) 🎯 MVP

**Goal**: Given a `PromptDefinition` + values, render the resolved variant to text and return provenance
(`text`, `name`, `variant`, `template_hash`, `render_hash`), with strict-undefined and caller-owned
variant resolution.

**Independent Test**: quickstart V1.1–V1.8 — render single/multi-variant prompts, strict-undefined
errors, unknown-variant errors, determinism, per-variant `template_hash`, `get_source`.

### Tests for User Story 1 ⚠️ (write first, expect fail)

- [ ] T013 [P] [US1] Variant-resolution + render tests in `crates/prompting-press-core/tests/render.rs`: V1.1 (single body), V1.3 (named variant), V1.5 (multi-variant, no name → root body as `default`, per amended FR-010), V1.6 (conditional+loop). Assert `text`, `variant`, presence of hashes.
- [ ] T014 [P] [US1] Determinism + hashing tests in `crates/prompting-press-core/tests/hashing.rs`: V1.2 (render twice → byte-identical text + equal hashes, SC-001), V1.8 (`get_source` bytes hash to the same `template_hash`), per-variant distinct `template_hash` (US1 scenario 7), no `vars_hash` field exists (FR-014).
- [ ] T015 [P] [US1] Error-path tests in `crates/prompting-press-core/tests/render_errors.rs`: V1.7 (undefined var → `UndefinedVariable`, not `"Hello "`, SC-009), V1.4 (unknown variant → `UnknownVariant{requested}`, FR-009).

### Implementation for User Story 1

- [ ] T016 [US1] Implement variant resolution in `crates/prompting-press-core/src/engine.rs`: `resolve_variant(def, Option<&str>) -> Result<ResolvedVariant, KernelError>` per data-model rule (None/`"default"` → root `body`; named → arm; unknown → `UnknownVariant`; reserved `default` enforced in-logic, FR-011). No "missing default" path (amended FR-010).
- [ ] T017 [US1] Implement `get_source(def, Option<&str>) -> Result<&str, KernelError>` in `engine.rs` returning the unrendered resolved arm source (FR-006), reusing `resolve_variant`.
- [ ] T018 [US1] Create `crates/prompting-press-core/src/hashing.rs`: `sha256_hex(&str) -> String` (lowercase hex, UTF-8 bytes) via `sha2::Sha256` (research D8, FR-012/013).
- [ ] T019 [US1] Implement `RenderResult` struct + `render(def, variant, values, guard) -> Result<RenderResult, KernelError>` in `engine.rs` (guard arg wired but US3 fills behavior; for US1 pass a disabled `GuardConfig`): build env (T011), add+render template (strict-undefined → `UndefinedVariable`; excluded-feature/parse → `ExcludedFeature`/`Parse`; other → `Render`), compute `template_hash`(source)/`render_hash`(text), populate `name`/`variant`. Deterministic (FR-003). NOTE (analysis F1): `def.name` is the generated newtype `PromptDefinitionName` (transparent over `String`), not a plain `String` — populate `RenderResult.name` via `def.name.to_string()` / deref, not by direct assignment.
- [ ] T020 [US1] Run `mise exec -- cargo test -p prompting-press-core --test render --test hashing --test render_errors`; confirm T013–T015 pass. Quick clippy/fmt pass on new files.

**Checkpoint**: US1 fully functional — render + provenance + variant resolution + strict-undefined, independently testable (MVP).

---

## Phase 4: User Story 2 — Sound agreement analysis (Priority: P2)

**Goal**: Report, per resolved variant, the set of required ROOT variable names a template references —
excluding loop/`set`/block locals and the engine globals allowlist; pure analysis.

**Independent Test**: quickstart V2.1–V2.6 — interpolation/nested, loop locals, set targets, globals,
no-mutation, undeclared-detectable.

### Tests for User Story 2 ⚠️ (write first, expect fail)

- [ ] T021 [P] [US2] Agreement-analysis tests in `crates/prompting-press-core/tests/agreement.rs`: V2.1 (`{greeting, user}` not `name`), V2.2 (loop local `item` excluded), V2.3 (`{% set x %}` target excluded), V2.4 (global `range`/`namespace` excluded), V2.6 (undeclared `foo` present). Assert sorted/deterministic output (SC-002, SC-003).
- [ ] T022 [P] [US2] Purity test in `crates/prompting-press-core/tests/agreement_purity.rs`: V2.5 — clone def+values, run `required_roots`, assert inputs unchanged; assert analysis renders nothing (SC-006, FR-018).

### Implementation for User Story 2

- [ ] T023 [US2] Create `crates/prompting-press-core/src/agreement.rs`: `Agreement { variant, required_roots: BTreeSet<String> }` and `required_roots(def, Option<&str>) -> Result<Agreement, KernelError>` — resolve variant (reuse T016), add template to env, call `Template::undeclared_variables(false)` (research D2).
- [ ] T024 [US2] In `agreement.rs`, build the globals allowlist DYNAMICALLY from the kernel `Environment`'s registered globals (drift-proof, research D2) and subtract it from the undeclared set; collect into a sorted `BTreeSet`. Document filters/tests are never reported (no allowlist entry needed).
- [ ] T025 [US2] In `agreement.rs`, guard the parse-error footgun (FR-016a): `undeclared_variables` returns empty on parse failure, so analysis MUST first ensure the template parses (add succeeds) and return `Err(Parse|ExcludedFeature)` otherwise — never an empty "requires nothing" success (research D2, FR-016a, FR-028). Covers V4.3.
- [ ] T026 [US2] Run `mise exec -- cargo test -p prompting-press-core --test agreement --test agreement_purity`; confirm T021–T022 pass.

**Checkpoint**: US1 + US2 both work independently — render path and the headline agreement analysis.

---

## Phase 5: User Story 3 — Var-provenance plumbing + opt-in guard expansion (Priority: P3)

**Goal**: Expose untrusted/external fields; on opt-in, return a configurable guard instruction as a
SEPARATE result field (never concatenated into the body), additive and non-mutating.

**Independent Test**: quickstart V3.1–V3.5 — provenance view, guard opt-out/opt-in/override, value
pass-through unchanged.

### Tests for User Story 3 ⚠️ (write first, expect fail)

- [ ] T027 [P] [US3] Provenance + guard tests in `crates/prompting-press-core/tests/provenance.rs`: V3.1 (`untrusted={q}`,`external={ctx}`), V3.2 (opt-out → `guard=None`, body == plain render), V3.3 (opt-in default → `guard=Some(...)` naming q,ctx; body byte-identical to plain render — SC-005), V3.4 (override template used), V3.5 (untrusted value unchanged — FR-025).

### Implementation for User Story 3

- [ ] T028 [P] [US3] Create `crates/prompting-press-core/src/provenance.rs`: `ProvenanceView { untrusted: BTreeSet<String>, external: BTreeSet<String> }` and `provenance_view(def) -> ProvenanceView` derived from `def.variables[*].provenance` (FR-021). Pure.
- [ ] T029 [P] [US3] In `provenance.rs`, add `GuardConfig { enabled: bool, template: Option<String> }` + a kernel default guard template constant (FR-024) and a `build_guard_text(view, &GuardConfig) -> Option<String>` that names the union of untrusted+external fields (sorted → deterministic). Additive only; no value access (FR-023/025). DEFINE the override-template contract (analysis F5): a single `{fields}` placeholder is substituted with the comma-joined sorted field names (e.g. `q, ctx`); the substitution is plain string replacement (NOT MiniJinja rendering — the guard template is not a prompt template and must not re-enter the engine); if the override omits `{fields}`, the text is used verbatim (no error). Document this on `GuardConfig`/the default constant.
- [ ] T030 [US3] Wire guard into `engine::render` (T019): when `guard.enabled`, set `RenderResult.guard = build_guard_text(provenance_view(def), guard)`; else `None`. Assert the rendered `text` is unaffected either way (separate field — FR-022, SC-005).
- [ ] T031 [US3] Run `mise exec -- cargo test -p prompting-press-core --test provenance`; confirm T027 passes.

**Checkpoint**: All three stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T032 [P] Excluded-feature regression tests in `crates/prompting-press-core/tests/excluded_features.rs`: V4.1/V4.2 — one fixture each for `{% include %}`, `{% extends %}`, `{% import %}`, `{% macro %}`, `{% block %}` → each errors at add/parse as `ExcludedFeature`/`Parse` (FR-002, SC-008). Confirm via the disabled-feature engine config (T011); if MiniJinja's error kind doesn't distinguish, document the `Parse` fallback (research D4).
- [ ] T033 [P] Crate-level rustdoc in `crates/prompting-press-core/src/lib.rs`: document the four capabilities, the FFI-free/validation-blind/no-I/O invariants (C-02/C-03), and that error normalization is the consumer's job (C-06). Add a normative **"what the kernel does NOT do with what it returns"** section (critique X1 / security SEC-002): the guard field does NOT sanitize and untrusted/external values pass through byte-for-byte (SC-005); provenance tags are metadata with NO runtime enforcement; `output_model` is a reference that is never parsed. Mirror the no-sanitization invariant on the `GuardConfig`/`ProvenanceView` rustdoc so a consumer cannot mistake them for a sanitizer. Keep examples doctest-valid.
- [ ] T034 Full local gate suite — `mise exec -- moon run :build`, `mise exec -- cargo test -p prompting-press-core`, `mise exec -- moon run schemas:codegen-check`, `mise exec -- moon run ci:check-ffi`, `mise exec -- moon run ci:check-floating-versions`; plus `cargo clippy -p prompting-press-core -- -D warnings` and `cargo fmt --check`. All green.
- [ ] T035 Walk quickstart.md end-to-end (all V-scenarios mapped to tests T013–T032) and confirm every SC-001…SC-009 has a passing test backing it; note any gap.
- [ ] T036 [P] Add a pinned dependency-advisory gate (security SEC-001): a `cargo audit` (or `cargo deny advisories`) check over the workspace, wired as a moon/CI task with the tool hash-pinned via mise (consistent with spec-001's `--locked` discipline). Bind the roadmap-Q3 "re-confirm MiniJinja stable-API soundness on each bump" obligation to this gate's owner/doc so a future `minijinja` upgrade triggers re-verification.

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (T001–T002)**: start immediately.
- **Foundational (T003–T012)**: depends on Setup. The relocation (T003–T009) is sequential and atomic (shared files: moon.yml, lib.rs); T010–T012 can follow. **BLOCKS all user stories.**
- **US1 (T013–T020)**: after Foundational. The MVP.
- **US2 (T021–T026)**: after Foundational; reuses `resolve_variant` (T016) → soft dep on US1's T016 (or implement T016 in Foundational if running US2 first).
- **US3 (T027–T031)**: after Foundational; T030 integrates into `render` (T019) → soft dep on US1's T019.
- **Polish (T032–T035)**: after all targeted stories.

### Within each story

- Tests (T013–T015 / T021–T022 / T027) written first and expected to FAIL before implementation.
- engine/resolve before hashing before render; analysis after resolve; guard after render exists.

### Parallel opportunities

- T001 ∥ nothing (single Cargo.toml); T002 after.
- Foundational: T012 [P] alongside T010/T011 once relocation (T003–T009) is done.
- US1 tests T013/T014/T015 all [P] (separate test files). US2 T021/T022 [P]. US3 T027 [P], impl T028/T029 [P].
- Polish T032/T033 [P].
- Cross-story: with the soft deps noted, US1 must land T016+T019 before US2/US3 reuse them — simplest is sequential US1→US2→US3 (one implementer). Parallel only if T016/T019 are pulled forward.

---

## Implementation Strategy

### MVP first (US1 only)

1. Phase 1 Setup → 2. Phase 2 Foundational (relocation is the big one) → 3. Phase 3 US1 →
4. **STOP & VALIDATE**: render + provenance + variant resolution + strict-undefined work and are
deterministic (SC-001/004/009). This is a demoable kernel: text in, rendered text + hashes out.

### Incremental delivery

- + US2 → the headline agreement analysis (SC-002/003/006).
- + US3 → provenance view + opt-in guard (SC-005).
- + Polish → excluded-feature coverage (SC-008), docs, full gate suite.

---

## Notes

- 36 tasks: Setup 2, Foundational 10, US1 8, US2 6, US3 5, Polish 5 (T032–T036).
- The Foundational relocation (T003–T009) is the highest-risk change — it touches the codegen pipeline + freshness gate. Verify the gate green (T009) before building kernel logic on top.
- Keep the kernel FFI-free at every step (T002 baseline, T034 final).
- Commit after each task or logical group; checkpoint after each phase (agent-assign flow runs checkpoints).
