---
description: "Task list for spec 003 — Rust consumer crate (prompting-press)"
---

# Tasks: Rust consumer crate (`prompting-press`)

**Input**: Design documents from `specs/003-rust-consumer/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D7), data-model.md, contracts/consumer-api.md, quickstart.md

**Tests**: INCLUDED. The Success Criteria are verification-driven (SC-002 reject-before-render, SC-003
YAML/JSON parity, SC-004/005 lint detection, SC-006 no-leak, SC-008 composition order) and
quickstart.md enumerates ~18 scenarios. Test tasks are first-class, per user story.

**Organization**: by user story — US1 validate+render (P1, MVP) → US2 dual-input loader (P2) → US3
agreement+provenance lint (P2) → US4 composition (P3) — after Setup + a Foundational phase
(deps + error type + registry) that blocks the stories.

## Conventions / guardrails (from memory + plan)

- Consumer stays FFI-free (C-02) — `ci:check-ffi`. New deps `garde`, `serde_yaml_ng` are pure-Rust
  (research D1/D2); verify.
- NO render/agreement/variant/hashing LOGIC here — WRAP the kernel (C-01). `check()` is set-ops over
  the kernel's `required_roots`/`provenance_view`.
- Native types never leak: garde `Report` + kernel `KernelError` → `ConsumerError` (C-06).
- Declared-vars authority = `def.variables` (clarify Q1); caller passes `render(name, &vars)` (Q3);
  validated struct → `minijinja::Value::from_serialize` (Q4).
- garde `Report` 0.23 has `iter()`/`into_inner()`, **no `flatten()`** (research D3) — normalize via `iter()`.
- Pin deps exactly (no floating — `ci:check-floating-versions`). `rm` blocked → `git mv`/`rmdir`.
  Pushes via `dgit`; cargo/moon under `mise exec --`; single-quote `git commit -m` with backticks.

---

## Phase 1: Setup

**Purpose**: Add the consumer's deps and confirm the FFI gate tolerates them before code.

- [X] T001 Add deps to `crates/prompting-press/Cargo.toml`: `garde = { version = "0.23", features = ["derive", "serde"] }` and `serde_yaml_ng = "0.10"` (pin exact current patch). `serde`/`serde_json`/`prompting-press-core` already present. Add `minijinja = { workspace = true }` if `Value` isn't reachable via the kernel re-export. Fix the STALE comment in Cargo.toml that claims the generated shape lives in `src/generated/` (it moved to the kernel in spec 002).
- [X] T002 Run `mise exec -- cargo build -p prompting-press`, `mise exec -- moon run ci:check-ffi`, and `cargo tree -p prompting-press -i pyo3` / `-i napi` (expect absent). Confirm garde + serde_yaml_ng pull no pyo3/napi (SC-007). Pin patch confirmed via `cargo update -p` if needed; `ci:check-floating-versions` stays green.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The error type, registry, and module skeleton every story depends on. **No user story
can begin until this is done** — render/check/compose all return `ConsumerError` and resolve against
the registry.

- [X] T003 Wire the consumer module tree in `crates/prompting-press/src/lib.rs`: add `pub mod {registry, render, check, compose, error};` + re-exports (`Registry`, `ConsumerError`, `FieldError`, `CheckReport`, `Finding`, `Composition`, `Message`). Keep the existing `pub use prompting_press_core::{PromptDefinition, RenderResult}` re-exports. (No `tokens` module — the hook was dropped, F4.)
- [X] T004 Create `crates/prompting-press/src/error.rs`: `FieldError { field: String, code: String, message: String }` and `ConsumerError` (variants: `Validation(Vec<FieldError>)`, `Kernel(Vec<FieldError>)`, `UnknownPrompt(String)`, plus a `Load`/parse variant for malformed YAML/JSON). Derive `Debug` + impl `Display` + `std::error::Error`. This is the ONLY public error type (C-06). NO `garde::Report` / `KernelError` in its public surface.
- [X] T005 In `error.rs`, implement the normalizers (FR-014/015): `From<garde::Report>` → `Validation(Vec<FieldError>)` via `Report::iter()` over `(Path, Error)` (`field = path.to_string()`, `message = error.message()`, `code` synthesized); and a `KernelError` → `Kernel(Vec<FieldError>)` map (one row per variant: UnknownVariant→`{field:"variant",code:"unknown_variant"}`, UndefinedVariable→`{field:name,code:"undefined_variable"}`, Parse/Render/ExcludedFeature→`{field:"template",code:…}`). **Pin a documented CLOSED set of `code` strings** (critique E2): `validation`, `unknown_prompt`, `unknown_variant`, `undefined_variable`, `parse`, `render`, `excluded_feature`, `load` — as consts, so consumers can match on `code` stably. **SEC-004 (FR-015): sanitize `Parse`/`Render` detail — do NOT copy raw bound-value content into `message`; add a sentinel-secret test (security review): a value carrying a secret that triggers a kernel `Render` error must NOT appear in the resulting `ConsumerError` string.**
- [X] T006 Create `crates/prompting-press/src/registry.rs`: `Registry { prompts: BTreeMap<String, PromptDefinition> }` (BTreeMap → deterministic check ordering) + `new()`, `insert(def)`, `get(name) -> Option<&PromptDefinition>`. Loaders (`load_yaml`/`load_json`) come in US2; for now `insert` + `get` suffice so US1 can render.

**Checkpoint**: crate builds; `ConsumerError`, normalizers, and `Registry` exist. Stories can begin.

---

## Phase 3: User Story 1 — Validate typed inputs + render (Priority: P1) 🎯 MVP

**Goal**: `render(reg, name, &vars, variant, &guard)` validates the typed Vars once, serializes, and
delegates to the kernel — returning `RenderResult` or a normalized error, with no native types leaking.

**Independent Test**: quickstart V1.1–V1.5.

### Tests for US1 ⚠️ (write first, expect fail)

- [X] T007 [P] [US1] Render + validation tests in `crates/prompting-press/tests/render.rs`: define a test Vars struct (`#[derive(Serialize, Validate)]` with a `#[garde(custom)]` and a `#[garde(range/length)]` field); V1.1 (valid → `RenderResult` with text+provenance), V1.5 (render twice → byte-identical text + equal hashes), and a guard-plumb-through assertion (F5): pass a `GuardConfig{enabled:true,..}` and assert `RenderResult.guard` is surfaced (present) vs `GuardConfig::default()` → absent — plumbing only, NOT re-testing kernel guard logic (owned by spec 002). Build the prompt via `Registry::insert` of a constructed `PromptDefinition` (root body referencing the vars).
- [X] T008 [P] [US1] Validation-failure tests in `crates/prompting-press/tests/render_validation.rs`: V1.2 (one field violates → `Err(ConsumerError::Validation([FieldError{field,..}]))`, assert NO render happened), V1.3 (multiple fields fail → all reported), V1.4 (assert the returned types are `RenderResult`/`ConsumerError` only — no `garde::Report`/`KernelError` reachable). **Three-sets gap test (critique E1)**: a Vars struct whose field name does NOT match the prompt's declared `variables`/template root → garde validation passes (the value is fine) but render returns a normalized `undefined_variable`-code `ConsumerError` (strict-undefined surfaced loudly, NOT silent). Pins the documented invariant.

### Implementation for US1

- [X] T009 [US1] Create `crates/prompting-press/src/render.rs`: `pub fn render<V: serde::Serialize + garde::Validate>(reg: &Registry, name: &str, vars: &V, variant: Option<&str>, guard: &prompting_press_core::GuardConfig) -> Result<prompting_press_core::RenderResult, ConsumerError>` — look up `name` in the registry (absent → `UnknownPrompt`); `vars.validate()` (or `validate_with` if the struct declares a context) — on `Err(Report)` return `ConsumerError::Validation` BEFORE any render (FR-002); on success `minijinja::Value::from_serialize(vars)` and call `prompting_press_core::render(def, variant, value, guard)`, mapping `KernelError` → `ConsumerError::Kernel`.
- [X] T010 [US1] In `render.rs`, add `pub fn get_source<'a>(reg: &'a Registry, name: &str, variant: Option<&str>) -> Result<&'a str, ConsumerError>` delegating to `prompting_press_core::get_source` (FR-010). No validation needed (no vars).
- [X] T011 [US1] Run `mise exec -- cargo test -p prompting-press --test render --test render_validation`; confirm T007/T008 pass. Quick clippy/fmt on new files.

**Checkpoint**: US1 functional — validate-then-render + get_source, no leaks (MVP).

---

## Phase 4: User Story 2 — Dual-input loader (Priority: P2)

**Goal**: Load the same prompt from YAML, JSON, or a constructed object into the one `PromptDefinition`
representation, with identical downstream behavior.

**Independent Test**: quickstart V2.1–V2.5.

### Tests for US2 ⚠️ (write first, expect fail)

- [X] T012 [P] [US2] Loader tests in `crates/prompting-press/tests/loader.rs`: V2.1 (load_yaml → PromptDefinition), V2.2 (load_json of the equivalent doc → representation identical to the YAML-loaded one — assert STRUCTURAL field-equality of the two parsed `PromptDefinition`s, e.g. via `PartialEq` or comparing re-serialized JSON, NOT a smoke check — **SC-003**, critique E3), V2.3 (insert constructed object on equal footing), V2.4 (malformed YAML/JSON or shape-violating data → `Err(ConsumerError)`, nothing partially loaded), V2.5 (YAML value `no`/`off` → parsed as a STRING not a bool — Norway-safe, research D2). Reuse the spec-001 schema fixtures (`schemas/jsonschema/fixtures/valid/*.json`) as JSON inputs + hand-write equivalent YAML.

### Implementation for US2

- [X] T013 [US2] In `registry.rs`, add `load_json(&mut self, doc: &str) -> Result<&PromptDefinition, ConsumerError>` (serde_json::from_str → insert) and `load_yaml(&mut self, doc: &str) -> Result<&PromptDefinition, ConsumerError>` (serde_yaml_ng::from_str → insert). Map deserialize errors → `ConsumerError` (a `Load`/parse variant); on error insert NOTHING (FR-007). Both normalize into the same `PromptDefinition` (FR-005/006/008).
- [X] T014 [US2] Run `mise exec -- cargo test -p prompting-press --test loader`; confirm T012 passes (esp. SC-003 YAML/JSON parity + V2.5 Norway-safe).

**Checkpoint**: US1 + US2 — render + dual-input loading.

---

## Phase 5: User Story 3 — Agreement + provenance lint (Priority: P2)

**Goal**: `check(registry)` — the headline guarantee as a CI lint: template referenced-roots ⊆ declared
`variables`, and a prompt declaring untrusted/external fields must have a guard configured (reframed F1). Pure, pass/fail.

**Independent Test**: quickstart V3.1–V3.5.

### Tests for US3 ⚠️ (write first, expect fail)

- [X] T015 [P] [US3] Check tests in `crates/prompting-press/tests/check.rs`: V3.1 (clean registry → empty findings/pass), V3.2 (template references a var NOT in `def.variables` → `Finding::UndeclaredVariable{prompt,variant,name}` — **SC-004**), V3.3 (a prompt declaring an untrusted/external field with NO guard configured → `Finding::UntrustedWithoutGuard{prompt,field}` — **SC-005**, reframed F1), V3.5 (multi-variant prompt → each variant analyzed). Build registries via `insert`.
- [X] T016 [P] [US3] Purity test in `crates/prompting-press/tests/check_purity.rs`: V3.4 — snapshot the registry/defs before `check()`, assert unchanged after, and assert nothing was rendered (FR-019, pure analysis).

### Implementation for US3

- [X] T017 [US3] Create `crates/prompting-press/src/check.rs`: `CheckReport { findings: Vec<Finding> }`, `Finding { prompt: String, variant: Option<String>, kind: FindingKind, detail: String }`, `FindingKind::{UndeclaredVariable{name}, UntrustedWithoutGuard{field}}`. `pub fn check(reg: &Registry) -> CheckReport`. `check()` over an EMPTY registry → empty `CheckReport` (pass — F7).
- [X] T018 [US3] Implement the agreement lint (FR-016/017) in `check.rs`: for each prompt (iterate the BTreeMap → deterministic), for each variant (default + each named), call `prompting_press_core::required_roots(def, variant)` → `Agreement.required_roots` (BTreeSet) **minus** `def.variables.keys()` (the declared set, clarify Q1) → each leftover root = an `UndeclaredVariable` finding (FR-020 names prompt+variant+var). Does NOT re-derive roots (kernel owns that).
- [X] T019 [US3] Implement the provenance lint (FR-018, reframed F1) in `check.rs`: `prompting_press_core::provenance_view(def)` → untrusted∪external; if the prompt declares any such field but carries NO guard configuration/metadata covering it, emit `UntrustedWithoutGuard{field}` per uncovered field (names prompt+field). (The kernel has no in-template "guard position"; the lint is "declared untrusted input + no guard set up.") Pure — no mutation, no render (FR-019).
- [X] T020 [US3] Run `mise exec -- cargo test -p prompting-press --test check --test check_purity`; confirm T015/T016 pass.

**Checkpoint**: the headline lint works; US1+US2+US3 independently functional.

---

## Phase 6: User Story 4 — Composition (Priority: P3)

**Goal**: Assemble a multi-message prompt as an ordered `Vec` of (prompt, vars) → `Vec<Message{role,
text}>`. `Vec` + `append_*`, never `.chain()`.

**Independent Test**: quickstart V4.1–V4.3.

### Tests for US4 ⚠️ (write first, expect fail)

- [ ] T021 [P] [US4] Composition tests in `crates/prompting-press/tests/compose.rs`: V4.1 (N appended (name,vars) entries → resolve → exactly N `Message{role,text}` in append order, each rendered with its own validated vars — **SC-008**), V4.2 (one entry's vars fail validation → `Err(ConsumerError)` naming the entry/field; no partial-as-success), V4.3 (render a fragment, pass its text into a parent prompt as a declared variable — composition-by-value, no include), V4.4 (empty composition → `resolve()` returns `Ok(vec![])` — F7).

### Implementation for US4

- [ ] T022 [US4] Create `crates/prompting-press/src/compose.rs`: `Message { role: String, text: String }`; `Composition` backed by an ordered `Vec` with `new()` + `append<V: Serialize + Validate>(&mut self, name, vars, variant) -> &mut Self` (NO `.chain()` — FR-013); `resolve(&self, reg: &Registry) -> Result<Vec<Message>, ConsumerError>` renders each entry in order (reusing `render::render`), `role` from the prompt's `role` metadata. One entry's failure → propagate the `ConsumerError`, no partial result.
- [ ] T023 [US4] Run `mise exec -- cargo test -p prompting-press --test compose`; confirm T021 passes.

**Checkpoint**: all four stories functional.

---

## Phase 7: Polish & Cross-Cutting

- ~~T024 [P] Token-count hook~~ — **DROPPED (analyze F4)**: the token-count hook is removed from spec 003 and deferred to a later spec. No `tokens.rs`, no `tests/tokens.rs`.
- [ ] T025 [P] Crate-level rustdoc in `crates/prompting-press/src/lib.rs`: document the public API (registry, render, check, compose), the C-06 normalization boundary (native types don't leak), the C-01 "wraps the kernel, no logic duplicated" stance, and that the crate does no I/O / carries `output_model` as metadata only (C-03). **Document the three-sets invariant (critique E1)**: the caller's Vars struct field names must match the prompt's declared `variables`; `check()` lints template↔`variables`, garde validates values, and a struct↔`variables` mismatch surfaces as a loud `undefined_variable` render error (not silent). **Add a `check()` CI-usage example (critique P2)**: load a registry from repo YAML, run `check`, non-zero exit on findings. Doctest-valid example (build a registry, render a tiny prompt). Cite "roadmap decision C-NN" NOT "constitution C-NN".
- [ ] T026 Full local gate suite — `mise exec -- moon run :build`, `mise exec -- cargo test -p prompting-press`, `mise exec -- moon run ci:check-ffi`, `ci:check-floating-versions`, `ci:check-advisories`, `schemas:codegen-check`; `cargo clippy -p prompting-press --all-targets -- -D warnings`; `cargo fmt --check`. All green.
- [ ] T027 Walk quickstart.md end-to-end (V1.1–V4.3 + the boundary check mapped to tests T007–T023) and confirm every SC-001…SC-008 has a passing backing test; note any gap. (SC-009 / token hook dropped — F4.)

---

## Dependencies & Execution Order

### Phase dependencies
- **Setup (T001–T002)**: start immediately.
- **Foundational (T003–T006)**: after Setup. **BLOCKS all stories** (everything returns `ConsumerError` / uses `Registry`). T004→T005 sequential (same file); T006 after T003.
- **US1 (T007–T011)**: after Foundational. The MVP.
- **US2 (T012–T014)**: after Foundational; adds loaders to `registry.rs` (independent of US1's render.rs).
- **US3 (T015–T020)**: after Foundational; `check.rs` is new (independent of US1/US2 files) but reuses `Registry`.
- **US4 (T021–T023)**: after US1 (reuses `render::render` in `resolve`).
- **Polish (T025–T027)**: after the targeted stories. (T024 dropped — F4.)

### Within each story
- Tests (T007/T008, T012, T015/T016, T021) written first, expected to FAIL before implementation.
- error/registry before render; render before compose.

### Parallel opportunities
- US1 test files T007/T008 [P]; US3 T015/T016 [P]; Polish T025 [P].
- US2 (`registry.rs` loaders) and US3 (`check.rs`) touch different files → could run in parallel after Foundational, BUT both come after US1 establishes the render path the simplest way; sequential US1→US2→US3→US4 is the clean single-implementer order.
- Cross-story: US4 needs US1's `render::render`. Foundational T004/T005 share `error.rs` → sequential.

---

## Implementation Strategy

### MVP first (US1)
Setup → Foundational → US1 → STOP & VALIDATE: an app can define typed Vars, register a prompt, and
`render` with validation + provenance, no native types leaking (SC-001/002/006). Demoable.

### Incremental delivery
+ US2 (load YAML/JSON/object, SC-003) → + US3 (the headline `check()` lint, SC-004/005) → + US4
(composition, SC-008) → + Polish (rustdoc, gates).

---

## Notes
- 26 tasks: Setup 2, Foundational 4, US1 5, US2 3, US3 6, US4 3, Polish 3 (T024 dropped — F4).
- The headline value is US3 (`check()`), but US1 is the MVP that proves the kernel/consumer split.
- Keep the consumer FFI-free at every step (T002 baseline, T026 final). No logic duplication — wrap the kernel.
- Commit after each task/logical group; checkpoint after each phase (agent-assign flow runs checkpoints).
