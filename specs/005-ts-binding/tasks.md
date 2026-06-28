---
description: "Task list for spec 005 — TypeScript binding (prompting-press-node)"
---

# Tasks: TypeScript binding (`prompting-press-node` → `packages/typescript`)

**Input**: Design documents from `specs/005-ts-binding/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D7), data-model.md, contracts/ts-api.md, quickstart.md

**Tests**: INCLUDED. The Success Criteria are verification-driven (SC-002 reject-before-render, SC-003
YAML/JSON/object parity, SC-004/005 lint detection, SC-006 no-leak + SEC-004 scrub, SC-008 composition
order, SC-009 build+import, SC-010 codegen-fresh + no-token-surface, SC-011 advisory gate) and
quickstart.md enumerates the scenarios. Test tasks are first-class, per user story: **Rust-side**
marshaling/scrub unit tests (`cargo test -p prompting-press-node`) + **TS-side** tests against the built
addon (run via the chosen runner — D6).

**Organization**: by user story — US1 validate+render (P1, MVP) → US2 dual-input loader (P2) → US3
agreement+provenance lint (P2) → US4 composition (P3) — after Setup + a Foundational phase (deps +
marshaling bridge + error hierarchy + registry class + the TS facade scaffold) that blocks the stories.

## Conventions / guardrails (from memory + plan)

- **`napi`/`napi-derive` ONLY in `prompting-press-node`** (C-02) — `ci:check-ffi` is EXTENDED this spec to
  assert `napi` (not just `pyo3`) is absent from the kernel + Rust consumer. The binding adds NO
  render/agreement/variant/hash LOGIC — it MARSHALS to the spec-003 consumer / spec-002 kernel
  (C-01 / Principle I; render parity structural, not re-tested).
- Native types never leak: `ZodError` + Rust `ConsumerError`/`KernelError` → `PromptingPressError`
  hierarchy (C-06). **SEC-004**: preserve the consumer's scrub — never surface raw
  `parse`/`render`/`excluded_feature` detail; the `ZodError` mapper copies issue `message`+`path` only.
- **Clarified (2026-06-27)**: Q1 validation owned at render (`safeParse` before templating); Q2 error
  hierarchy under one base `PromptingPressError`; Q3 dual-input loader REUSED from the Rust consumer via
  FFI (marshal text); Q4 Zod schema OR plain typed data accepted; Q5 per-platform `optionalDependencies`;
  Q6 `undefined`/absent → field-not-present, `null` → JSON null (matched to the Python binding); Q7
  Zod v4; Q8 ESM-only.
- **Codegen'd shape** (C-07): never hand-edit `packages/typescript/src/generated/`;
  `schemas:codegen-check` gates freshness. **No token surface** anywhere (F4).
- Versions verified this cycle (crates.io/npm directly): napi/napi-derive 3.9.4 (pin EXACT — was floating
  `"3"`), Zod 4.4.3, @napi-rs/cli 3.7.2, json-schema-to-typescript 15.0.4, typescript 5.9.2. `rm` blocked
  → `git mv`/`rmdir`. Pushes via `dgit`; cargo/moon/pnpm under `mise exec --`; single-quote `git commit -m`
  with backticks. Cite "roadmap decision C-NN", never "constitution C-NN".

---

## Phase 1: Setup

**Purpose**: Pin the FFI toolkit, add the Zod + test-runner deps, and confirm the FFI gate + a baseline
build/import before code.

- [X] T001 In `crates/prompting-press-node/Cargo.toml`: pin `napi` and `napi-derive` to the EXACT latest 3.x (`3.9.4`, crates.io-verified) — they currently declare floating `"3"`, which the `ci:check-floating-versions` gate flags. Confirm path deps on `prompting-press` + `prompting-press-core` remain (already present). Keep `crate-type = ["cdylib"]`.
- [X] T002 In `packages/typescript/package.json`: add `zod` (v4, exact `4.4.3` or a pinned spec — the floating-version gate may cover `package.json`; pin, don't caret) to `dependencies` (the runtime Vars facade); add the chosen test runner (D6 — lean Vitest, else node:test → no dep) to `devDependencies` (pinned). Leave `type: module`, the `@napi-rs/cli` build scripts, `json-schema-to-typescript`, and `typescript` as-is. Run `pnpm -C packages/typescript install` to refresh the lockfile.
- [X] T003 Baseline build + FFI/codegen gates: `mise exec -- cargo build -p prompting-press-node`; `mise exec -- pnpm -C packages/typescript build` (napi build → the platform `.node` + `index.{js,d.ts}`); `node --input-type=module -e "import('prompting-press').then(m=>console.log(typeof m))"` (the stub addon imports — SC-009 baseline); `mise exec -- moon run ci:check-ffi --force` (still green for pyo3 — napi assertion added in T028); `mise exec -- moon run schemas:codegen-check --force` (generated TS shape fresh). All green before writing binding code.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The marshaling bridge, the error hierarchy, the `Registry` napi class, and the TS facade
scaffold every story depends on. **No user story can begin until this is done.**

- [X] T004 Wire the binding module tree in `crates/prompting-press-node/src/lib.rs`: `mod {registry, render, check, compose, error, marshal};` and `#[napi]` registration of the classes (`Registry`, `RenderResult`, `CheckReport`, `Finding`, `Composition`, `Message`) + the functions (`render`, `getSource`, `check`). Replace the spec-001 stub `coreVersion` fn (or keep it). (No token surface — F4.)
- [X] T005 [P] Create `crates/prompting-press-node/src/marshal.rs` (the FFI value bridge, FR-003a — research D2): `fn to_kernel_value(js: <napi serde value>) -> Result<minijinja::Value, ConsumerError-ish>` going JS value → `serde_json::Value` (napi serde support) → `minijinja::Value::from_serialize` (the same primitive the consumer uses, so render parity is structural). Implement the Q6 rule: `undefined`/absent → field-not-present; `null` → JSON null; pin `bigint` losslessness. Concentrate ALL JS↔kernel value translation here (one auditable file — C-02).
- [X] T006 [P] Create `crates/prompting-press-node/src/error.rs`: the Rust side of error normalization. Translate the consumer's `ConsumerError` (EXHAUSTIVE match over its closed variants → a structured payload the TS side maps to the right subclass + rows; a new variant must be a compile error, not a fallthrough). **SEC-004**: render/compose call the kernel DIRECTLY (E1), so route the raw `KernelError` through the consumer's tested `From<KernelError> for ConsumerError` scrubber FIRST (the consumer replaces `parse`/`render`/`excluded_feature` detail with a fixed message — `crates/prompting-press/src/error.rs`), then surface those already-scrubbed rows — never copy raw `KernelError` detail across napi. Decide the napi error-return mechanism (research D4: structured rows, not a string-encoded message if avoidable). The `code` strings reuse the consumer's closed vocabulary.
- [X] T007 Create `crates/prompting-press-node/src/registry.rs`: `#[napi] struct Registry(prompting_press::Registry)` + a constructor, `insert(definition)`. (Loaders `loadYaml`/`loadJson` come in US2; `insert` + internal get suffice so US1 can render.) A name absent at render/check → `UnknownPromptError` (never a Rust panic across napi).
- [X] T008 Create the TS facade scaffold in `packages/typescript/src/index.ts`: the `PromptingPressError` base (`extends Error`, `readonly errors: FieldError[]`) + the four subclasses (`PromptValidationError`, `PromptRenderError`, `UnknownPromptError`, `LoadError`); a `FieldError` type; the napi-payload → subclass mapper (research D4); re-export the napi classes (`Registry`, `RenderResult`, `CheckReport`, `Finding`, `Composition`, `Message`) + `PromptDefinition` from `./generated`. (Render/check/compose wrappers are added per story.)

**Checkpoint**: addon builds + imports; the error hierarchy, the marshaling bridge, the `Registry` class, and the TS facade scaffold exist. Stories can begin.

---

## Phase 3: User Story 1 — Validate typed inputs + render (Priority: P1) 🎯 MVP

**Goal**: `render(reg, name, schema, data, opts?)` validates the Zod Vars once (owned by the TS facade — Q1),
marshals across napi, and delegates to the kernel — returning a `RenderResult` or throwing the right
`PromptingPressError`, with no native types leaking.

**Independent Test**: quickstart US1 (build+import SC-009; valid render SC-001; reject-before-render SC-002).

### Tests for US1 ⚠️ (write first, expect fail)

- [X] T009 [P] [US1] Rust-side marshaling + scrub unit tests in `crates/prompting-press-node/src/render.rs` / `error.rs` (`#[cfg(test)]`): a raw `KernelError::Render{detail with a seeded secret}` routed through the binding's translation → scrubbed rows whose message/detail do NOT contain the secret (SEC-004); each `ConsumerError` variant maps to the right code. Marshaling: a JS-ish value with null/undefined/number/bigint/nested → the expected `minijinja::Value` (lossless — FR-003a / Q6).
- [X] T010 [P] [US1] TS-side render tests in `packages/typescript/test/render.test.ts` (against the built addon): a Zod Vars schema with a `.refine()`; US1.valid (valid data → `RenderResult` with text + name + variant + 64-hex `templateHash`/`renderHash`); guard-plumb (pass guard config → `RenderResult.guard` present, vs default → `null`); US1.invalid (a field violates → `PromptValidationError` with a row naming the field + `code==="validation"`, and assert NO render happened); assert the thrown error is `instanceof PromptValidationError` and NOT a `ZodError` (SC-006). **Three-sets gap test**: a Vars field misnamed vs the prompt's `variables` → validation passes but render throws `PromptRenderError` with `code==="undefined_variable"` (loud, not a silent empty render). SEC-004-PY-equiv: a secret in a Zod-rejected value does NOT surface (mapper uses issue `message` only).

### Implementation for US1

- [X] T011 [US1] Create `crates/prompting-press-node/src/render.rs`: `#[napi] fn render(reg, name, value, variant?, guard?) -> RenderResult` where `value` is the already-Zod-validated plain JS object. Resolve `name` via the consumer `Registry::get` (absent → `UnknownPromptError`); `marshal::to_kernel_value(value)` → call **`prompting_press_core::render(def, variant, value, &guard)`** DIRECTLY (the consumer's generic-`V` render needs a garde Rust type the binding lacks — critique E1; calling the kernel is still zero engine logic, Principle I). Map the returned `KernelError` through the consumer's `ConsumerError::from` scrubber (SEC-004), then to the structured error payload. Return a `RenderResult` `#[napi]` class surfacing the kernel result 1:1 (text/name/variant/templateHash/renderHash/guard) with read-only getters.
- [X] T012 [US1] In `render.rs`, add `#[napi] fn getSource(reg, name, variant?) -> string` delegating to `prompting_press::get_source` (FR-010; no vars → no validation).
- [X] T013 [US1] In the TS facade (`packages/typescript/src/index.ts`), add the `render(reg, name, schema, data, opts?)` wrapper: `schema.safeParse(data)` (Q1) — on `!success` map `error.issues` → `PromptValidationError` rows (issue `message`+`path` only — SEC-004-PY-equiv, research D3) and throw BEFORE calling the addon; on success call the napi `render` and rethrow any structured addon error as the right subclass. Accept already-typed data with no schema (Q4). Add `getSource`/`check`/`Composition` re-exports/wrappers as needed.
- [X] T014 [US1] Build + run: `mise exec -- cargo test -p prompting-press-node` (T009); `mise exec -- pnpm -C packages/typescript build`; run the TS render tests (T010). clippy/fmt the new Rust files; lint/typecheck the TS.

**Checkpoint**: US1 functional — validate-then-render + getSource from TS, no leaks (MVP); the FFI marshaling path works end to end.

---

## Phase 4: User Story 2 — Dual-input loader (Priority: P2)

**Goal**: Load the same prompt from YAML, JSON, or a constructed object into the one representation, with
identical downstream behavior — by REUSING the Rust consumer's loader via FFI (Q3).

**Independent Test**: quickstart US2 (YAML/JSON/object parity SC-003; malformed → `LoadError`).

### Tests for US2 ⚠️ (write first, expect fail)

- [X] T015 [P] [US2] TS-side loader tests in `packages/typescript/test/loader.test.ts`: US2.parity (load the same logical prompt via `loadYaml(text)`, `loadJson(text)`, and `insert(obj)` → render each with identical inputs → identical text + provenance — **SC-003**); US2.malformed (invalid YAML/JSON or shape-violating data → `LoadError`, nothing partially loaded — confirm the registry has no entry afterward); US2.norway (YAML `no`/`off` → parsed as STRING not bool, inherited from the Rust loader — research D2/spec-003). Reuse `schemas/jsonschema/fixtures/valid/*.json` as JSON inputs + equivalent YAML.

### Implementation for US2

- [X] T016 [US2] In `registry.rs`, add `#[napi]` `loadYaml(text)` and `loadJson(text)` that marshal the TEXT to `prompting_press::Registry::load_yaml` / `load_json` (Q3 — the consumer owns parsing); map the consumer's `Load` error → `LoadError`; on error insert NOTHING (FR-007). Extend `insert(definition)` to take the generated `PromptDefinition` object, JSON-stringify it, and route through the consumer's `load_json` (one loader, one representation — FR-005/006/008).
- [X] T017 [US2] Build + run: `mise exec -- pnpm -C packages/typescript build`; run T015 (esp. SC-003 parity + Norway-safe). Parity holds because the SAME Rust loader handles all three paths.

**Checkpoint**: US1 + US2 — render + dual-input loading, parity structural.

---

## Phase 5: User Story 3 — Agreement + provenance lint (Priority: P2)

**Goal**: `check(registry)` from TS — the headline guarantee as a CI lint, surfaced over the consumer's
`check`: template referenced-roots ⊆ declared `variables`, and untrusted/external-without-guard. Pure,
pass/fail, deterministic order.

**Independent Test**: quickstart US3 (undeclared-var SC-004; untrusted-without-guard SC-005; clean passes; pure).

### Tests for US3 ⚠️ (write first, expect fail)

- [X] T018 [P] [US3] TS-side check tests in `packages/typescript/test/check.test.ts`: US3.clean (well-formed registry → `report.passed()` true, empty findings); US3.undeclared (template references a var not in `variables` → a `Finding` with `kind==="undeclared_variable"` naming prompt/variant/var — **SC-004**); US3.untrusted (a prompt declaring an untrusted/external field with no `meta.guard` → `kind==="untrusted_without_guard"` naming prompt/field — **SC-005**); US3.reserved (a variant literally named `default` → `kind==="reserved_variant_name"`); US3.analysis (a template using an excluded feature like `{% include %}` → `kind==="analysis_error"`, no crash); US3.pure (snapshot the registry before/after `check` → unchanged, nothing rendered — FR-019).

### Implementation for US3

- [X] T019 [US3] Create `crates/prompting-press-node/src/check.rs`: `#[napi] fn check(reg) -> CheckReport` delegating to `prompting_press::check`; `#[napi]` `CheckReport` with `findings: Finding[]` + `passed() -> boolean` + `isEmpty()`; `#[napi]` `Finding` with read-only getters `prompt`, `variant?`, `kind` (stringify the consumer's `FindingKind` discriminants → `undeclared_variable`/`untrusted_without_guard`/`reserved_variant_name`/`analysis_error`), `detail`. Preserve the consumer's deterministic (BTreeMap/BTreeSet) order. No re-derivation — the consumer/kernel own the analysis (C-01).
- [X] T020 [US3] Build + run: `mise exec -- pnpm -C packages/typescript build`; run T018 (all finding kinds + purity).

**Checkpoint**: the headline lint works from TS; US1+US2+US3 independently functional.

---

## Phase 6: User Story 4 — Composition (Priority: P3)

**Goal**: Assemble a multi-message prompt as an ordered array of (prompt, vars, variant) →
`Message[]{role, text}`, via `fromMessages` / `append`, never `.chain()`.

**Independent Test**: quickstart US4 (N entries → N ordered messages SC-008; one invalid → no partial; empty → []).

### Tests for US4 ⚠️ (write first, expect fail)

- [X] T021 [P] [US4] TS-side composition tests in `packages/typescript/test/compose.test.ts`: US4.order (`Composition.fromMessages([...])` of N entries → `resolve(reg)` → exactly N `Message{role, text}` in input order, each rendered with its own validated vars — **SC-008**); US4.partial (one entry's vars fail validation → throws at append/resolve, NO partial message array returned as success); US4.empty (`new Composition()` → `resolve` → `[]`); assert NO `.chain()` method exists on the class (FR-013).

### Implementation for US4

- [X] T022 [US4] Create `crates/prompting-press-node/src/compose.rs`: `#[napi]` `Message { role, text }` (read-only getters); `#[napi]` `Composition` (a binding-owned ordered list of marshaled `(name, value, variant)` entries — NOT the consumer's generic `Composition<V>`, which is generic over a garde type the binding lacks — critique E1) with a constructor, a `fromMessages(entries)` static factory, `append(name, vars, variant?)` (eager-validate at the TS-facade boundary — the napi `append` receives an already-validated value + marshals it; nothing stored on failure), and `resolve(reg) -> Message[]`. The resolve loop is the only binding-side orchestration (~10 lines of glue, NOT shared-core logic): for each entry in order, `Registry::get` (absent → `UnknownPromptError`), call `prompting_press_core::render` DIRECTLY with the stored value, `role` from the def's role; one entry's failure propagates as the error (`KernelError` via the consumer scrubber), partial result discarded — no partial-as-success. NO `.chain()` (FR-013). (Vars validation for compose entries lives in the TS facade `fromMessages`/`append` wrapper, mirroring US1's `safeParse`-at-boundary.)
- [X] T023 [US4] Build + run: `mise exec -- pnpm -C packages/typescript build`; run T021.

**Checkpoint**: all four stories functional from TS.

---

## Phase 7: Polish & Cross-Cutting

- [X] T024 [P] TS package facade finalize in `packages/typescript/src/index.ts`: confirm the public exports (`Registry`, `RenderResult`, `Message`, `Composition`, `CheckReport`, `Finding`, `render`, `getSource`, `check`) + the error hierarchy (`PromptingPressError`, `PromptValidationError`, `PromptRenderError`, `UnknownPromptError`, `LoadError`) + `FieldError` type + re-export `PromptDefinition` from `./generated`. Ensure the package `main`/`types`/`exports` map (ESM-only) points at the built entry + the facade. Do NOT hand-edit `generated/` (C-07).
- [X] T025 [P] Docs: `packages/typescript/README.md` quickstart documenting the public TS API (registry, render, check, compose), the C-06 normalization boundary (native types don't leak; error hierarchy + shared `code` vocabulary), the C-01/C-02 "marshals to the shared core, no engine logic" stance, the three-sets invariant (Vars field names must match the prompt's `variables`; a mismatch → loud `undefined_variable`, not silent), and that the package does no I/O / carries `outputModel` as metadata only / ships NO token counter (C-03 / F4). **Guard-usage doctrine (decided 2026-06-27, carried from 004)**: document `guard` as the **system-prompt addendum** — single render → route `RenderResult.guard` into your system prompt and send `text` as the user message; multi-message → place the guard as its own `system` message. The library never assembles the request body (Principle III); `guard` and `text` stay separate. Cite "roadmap decision C-NN", NOT "constitution C-NN".
- [X] T026 Build the distributable package + fresh-env import (SC-009): `mise exec -- pnpm -C packages/typescript build` → the platform `.node` + ESM entry; in a clean dir `pnpm pack` + install the tarball and `import('prompting-press')` + run a one-line render. Confirm ESM-only resolution works on Node 20+ (scaffold `engines.node: ">=20"`).
- [ ] T027 Full local gate suite — `mise exec -- moon run :build`; `mise exec -- cargo test -p prompting-press-node`; the TS test run (`render`/`loader`/`check`/`compose`); `mise exec -- moon run ci:check-ffi --force` (now asserts napi/napi-derive absent from `-core` + `prompting-press` — T028); `ci:check-floating-versions` (napi pinned exact + zod/test-runner pinned); `ci:check-advisories` (Rust) + `ci:check-advisories-node` (npm, T029); `schemas:codegen-check --force`; `cargo clippy -p prompting-press-node --all-targets -- -D warnings`; `cargo fmt --check`; TS lint/typecheck. Confirm NO token-counting surface (SC-010 / F4): `rg -n "count_tokens|tokenCount|countTokens|count-tokens" packages/typescript/src crates/prompting-press-node/src` finds nothing (narrow patterns). All green.
- [X] T028 VERIFY the FFI-isolation gate covers napi (FR-022 / SC-007) — **analyze F1 correction: the gate ALREADY asserts `napi`** (`scripts/ci/check-ffi-isolation.sh` ships `FFI_CRATES=("pyo3" "napi")` from spec 001; do NOT re-add it). Confirm `cargo tree -p prompting-press -i napi` and `-p prompting-press-core -i napi` are empty and `ci:check-ffi --force` stays green now that `prompting-press-node` actually depends on `napi`. The binding crate (`prompting-press-node`) is exempt (it's the FFI boundary). Only touch the script if a gap is found (none expected).
- [X] T029 Node dependency advisory gate (FR-025 / SC-011): create `scripts/ci/check-advisories-node.sh` that scans the pnpm lockfile for known CVEs (`pnpm audit --audit-level=…` or `osv-scanner`, pinned) and fails on a known CVE; register it as a `check-advisories-node` task in `ci/moon.yml` (mirror the `check-advisories-py` block: `runFromWorkspaceRoot: true`, `cache: false`); add it to the CI workflow and the T027 gate list. Pin the audit tool (no floating version). Mirrors the Rust + Python advisory gates.
- [X] T030 Wire `ci:test-node` into CI (the spec-004 I1 lesson): create `scripts/ci/test-node.sh` (build the addon via `napi build` + run `cargo test -p prompting-press-node` + the TS test run) + a `test-node` moon task + a CI job, mirroring `ci:test-python`. **Verify the napi build + addon-load works on the LINUX runner** (the spec-004 maturin/libpython lesson — a binding that compiles can still fail to load at runtime in CI). Without this, the TS-observable guarantees (Zod validation, error subclasses, marshaling) are un-gated and can rot.
- [ ] T031 Walk quickstart.md end-to-end and confirm every SC-001…SC-011 has a passing backing test or gate; note any gap.

---

## Dependencies & Execution Order

### Phase dependencies
- **Setup (T001–T003)**: start immediately. T001→T002 (Cargo then package.json); T003 after both (baseline build/gates).
- **Foundational (T004–T008)**: after Setup. **BLOCKS all stories**. T004 (module wiring) first; T005 (marshal) + T006 (error) are [P] (different files); T007 (registry) after T004; T008 (TS facade scaffold) is [P] with the Rust files (different language/dir).
- **US1 (T009–T014)**: after Foundational. The MVP — proves the FFI marshaling path. Note the split: Rust addon (T011/T012) + TS facade wrapper (T013).
- **US2 (T015–T017)**: after Foundational; adds loaders to `registry.rs` (independent of US1's render.rs).
- **US3 (T018–T020)**: after Foundational; `check.rs` is new (independent of US1/US2 files) but reuses `Registry`.
- **US4 (T021–T023)**: after US1 (reuses US1's kernel-direct render path + the TS-facade validation boundary in resolve).
- **Polish (T024–T031)**: after the targeted stories. T028 (extend FFI gate), T029 (Node advisory gate), T030 (ci:test-node) are CI wiring, partly independent of the binding code but verified against it.

### Within each story
- Tests (T009/T010, T015, T018, T021) written first, expected to FAIL before implementation.
- marshal + error + registry + facade-scaffold before render; render before compose.

### Parallel opportunities
- Foundational T005 (marshal.rs) + T006 (error.rs) + T008 (TS facade scaffold) [P]. US1 test files T009 (Rust) + T010 (TS) [P]. Polish T024 + T025 [P].
- Cross-story: clean single-implementer order is Setup→Foundational→US1→US2→US3→US4→Polish. US4 needs US1.

---

## Implementation Strategy

### MVP first (US1)
Setup → Foundational → US1 → STOP & VALIDATE: a TS app can define Zod Vars, register a prompt, and
`render` with validation + provenance, no native types leaking (SC-001/002/006/009). The second FFI
binding proves the marshaling path on a different runtime — and makes the conformance corpus (spec 006)
possible. Demoable.

### Incremental delivery
+ US2 (load YAML/JSON/object, SC-003) → + US3 (the headline `check()` lint, SC-004/005) → + US4
(composition, SC-008) → + Polish (facade, docs, package build, the three CI gates).

---

## Notes
- 31 tasks: Setup 3, Foundational 5, US1 6, US2 3, US3 3, US4 3, Polish 8.
- The headline value is US3 (`check()`), but US1 is the MVP that proves the FFI marshaling path on Node.
- **Structural difference from 004**: the Zod facade lives in TS (`src/index.ts`) over the napi addon
  (Zod can't live in Rust), so US1/US4 each have a Rust-addon task + a TS-facade task. Still zero engine
  logic — the facade does `safeParse` + error mapping + delegates to the addon → kernel.
- Keep `napi`/`napi-derive` in `prompting-press-node` ONLY at every step (T003 baseline, T027/T028 final).
  No engine logic — marshal to the shared core (C-01/C-02). No token surface (F4). Generated shape codegen'd (C-07).
- Commit after each task/logical group; checkpoint after each phase (agent-assign flow runs checkpoints).
