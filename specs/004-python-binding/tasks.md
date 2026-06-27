---
description: "Task list for spec 004 — Python binding (prompting-press-py)"
---

# Tasks: Python binding (`prompting-press-py` → `packages/python`)

**Input**: Design documents from `specs/004-python-binding/`

**Prerequisites**: plan.md, spec.md, research.md (D1–D7), data-model.md, contracts/python-api.md, quickstart.md

**Tests**: INCLUDED. The Success Criteria are verification-driven (SC-002 reject-before-render, SC-003
YAML/JSON/object parity, SC-004/005 lint detection, SC-006 no-leak + SEC-004 scrub, SC-008 composition
order, SC-009 build+import, SC-010 codegen-fresh + no-token-surface) and quickstart.md enumerates the
scenarios. Test tasks are first-class, per user story: **Rust-side** marshaling/scrub unit tests
(`cargo test -p prompting-press-py`) + **Python-side** pytest against the built wheel.

**Organization**: by user story — US1 validate+render (P1, MVP) → US2 dual-input loader (P2) → US3
agreement+provenance lint (P2) → US4 composition (P3) — after Setup + a Foundational phase
(deps + marshaling bridge + exception hierarchy + registry pyclass) that blocks the stories.

## Conventions / guardrails (from memory + plan)

- **`pyo3`/`pythonize` ONLY in `prompting-press-py`** (C-02) — `ci:check-ffi` checks the kernel + Rust
  consumer stay FFI-free. The binding adds NO render/agreement/variant/hash LOGIC — it MARSHALS to the
  spec-003 consumer / spec-002 kernel (C-01 / Principle I; render parity structural, not re-tested).
- Native types never leak: Pydantic `ValidationError` + Rust `ConsumerError`/`KernelError` →
  `PromptingPressError` hierarchy (C-06). **SEC-004**: preserve the consumer's scrub — never surface raw
  `parse`/`render`/`excluded_feature` detail.
- **Clarified (2026-06-27)**: Q1 validation owned at render (`model_validate` before templating); Q2
  exception hierarchy under one base `PromptingPressError`; Q3 dual-input loader REUSED from the Rust
  consumer via FFI (marshal text); Q4 abi3 floor = CPython 3.10 (bump crate `abi3-py39` → `abi3-py310`),
  latest stable (3.14) as dev/test.
- **Codegen'd shape** (C-07): never hand-edit `packages/python/python/prompting_press/generated/`;
  `schemas:codegen-check` gates freshness. **No token surface** anywhere (F4).
- Versions verified this cycle: PyO3 0.29 / pythonize 0.29 / maturin 1.14.1 / Pydantic 2.13.4 / dmcg
  0.65.1 (pin). `rm` blocked → `git mv`/`rmdir`. Pushes via `dgit`; cargo/moon/maturin under `mise exec --`;
  single-quote `git commit -m` with backticks. Cite "roadmap decision C-NN", never "constitution C-NN".

---

## Phase 1: Setup

**Purpose**: Reconcile the abi3 floor, add the marshaling dep, and confirm the FFI gate + a baseline
build/import before code.

- [X] T001 In `crates/prompting-press-py/Cargo.toml`: bump the pyo3 feature `abi3-py39` → `abi3-py310` (clarified Q4 — matches `requires-python >=3.10` + the codegen 3.10 target); add `pythonize = "0.29"` (version-matched to PyO3 0.29 — research D2). Confirm path deps on `prompting-press` + `prompting-press-core` remain (already present). Pin exact patch (no floating — `ci:check-floating-versions`).
- [X] T002 In `packages/python/pyproject.toml`: add `pydantic` (v2, bounded range e.g. `>=2,<3`) to `[project] dependencies` (the wheel's runtime needs it for the Vars facade + the generated shape). Leave `requires-python = ">=3.10"`, the maturin backend, `module-name`, and `python-source` as-is. Run `uv lock --project packages/python` to refresh the lock.
- [X] T003 Baseline build + FFI/codegen gates: `mise exec -- cargo build -p prompting-press-py`; `mise exec -- maturin develop -m crates/prompting-press-py/Cargo.toml` (builds into the dev venv); `python -c "import prompting_press"` (the stub `core_version` import succeeds — SC-009 baseline); `mise exec -- moon run ci:check-ffi --force` (pyo3/pythonize absent from `-core` + `prompting-press` — `cargo tree -p prompting-press -i pyo3` empty); `mise exec -- moon run schemas:codegen-check --force` (generated Pydantic shape fresh). All green before writing binding code.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The marshaling bridge, the exception hierarchy, and the `Registry` pyclass every story
depends on. **No user story can begin until this is done** — render/check/compose all marshal values,
raise the exception hierarchy, and resolve against the registry.

- [X] T004 Wire the binding module tree in `crates/prompting-press-py/src/lib.rs`: `mod {registry, render, check, compose, error, marshal};` and a `#[pymodule] fn prompting_press_py(m)` that registers the classes (`Registry`, `RenderResult`, `CheckReport`, `Finding`, `Composition`, `Message`), the functions (`render`, `get_source`, `check`), and the exception types (T006). Keep the `core_version` stub or fold it in. (No token surface — F4.)
- [X] T005 [P] Create `crates/prompting-press-py/src/marshal.rs` (the FFI value bridge, FR-003a — research D2): `fn to_kernel_value(obj: &Bound<PyAny>) -> Result<minijinja::Value, ConsumerError-ish>` using `pythonize::depythonize` (Python object → serde value → `minijinja::Value`, mirroring the consumer's `from_serialize` shape). Decide & pin the `model_dump` mode at the call site (lean `mode="json"` so date/Decimal stringify deterministically — D2). Concentrate ALL Python↔kernel value translation here (one auditable file — C-02).
- [X] T006 [P] Create `crates/prompting-press-py/src/error.rs`: the exception hierarchy (clarified Q2; PyO3 0.29 `create_exception!` + an `#[pyclass(extends=PyException)]` base carrying `.errors`). Base `PromptingPressError` with `errors: list[{field,code,message}]`; subtypes `PromptValidationError`, `PromptRenderError`, `UnknownPromptError`, `LoadError`. Implement the translation from the consumer's `ConsumerError` (EXHAUSTIVE match over its closed variants → the right subtype + rows; a new variant must be a compile error, not a fallthrough) and a Pydantic `ValidationError` → `PromptValidationError` mapper (`.errors()` rows → `{field: loc-joined, code:"validation", message}` — use Pydantic's `msg` ONLY; do NOT echo `input`/`ctx`, which can carry the rejected value — SEC-004-PY). **SEC-004**: render/compose call the kernel DIRECTLY (E1), so route the raw `KernelError` through the consumer's tested `From<KernelError> for ConsumerError` scrubber FIRST (the consumer replaces `parse`/`render`/`excluded_feature` detail with a fixed message — `crates/prompting-press/src/error.rs:191`), then surface those already-scrubbed rows verbatim — never copy raw `KernelError` detail into the exception. The `code` strings reuse the consumer's closed vocabulary. The `#[pyclass(extends=PyException)]` base must NOT hand-write a `__str__`/`__repr__` that re-introduces row content beyond the scrubbed message.
- [X] T007 Create `crates/prompting-press-py/src/registry.rs`: `#[pyclass] struct Registry(prompting_press::Registry)` + `#[new]`, `insert(definition)`. (Loaders `load_yaml`/`load_json` come in US2; `insert` + internal `get` suffice so US1 can render.) A name absent at render/check → `UnknownPromptError` (never a Rust panic across FFI).

**Checkpoint**: extension builds + imports; the exception hierarchy, the marshaling bridge, and the `Registry` pyclass exist. Stories can begin.

---

## Phase 3: User Story 1 — Validate typed inputs + render (Priority: P1) 🎯 MVP

**Goal**: `render(reg, name, Model, data, variant=None, guard=None)` validates the Pydantic Vars once
(owned by the binding — Q1), marshals, and delegates to the consumer/kernel — returning a `RenderResult`
or raising the right `PromptingPressError`, with no native types leaking.

**Independent Test**: quickstart US1 (build+import SC-009; valid render SC-001; reject-before-render SC-002).

### Tests for US1 ⚠️ (write first, expect fail)

- [X] T008 [P] [US1] Rust-side marshaling + scrub unit tests in `crates/prompting-press-py/src/render.rs` / `error.rs` (`#[cfg(test)]`, `Python::attach`): a raw `KernelError::Render{detail with a seeded secret}` routed through the binding's translation → a `PromptRenderError` whose **Python** `str(exc)`, `repr(exc)`, AND `exc.errors` rows do NOT contain the secret (SEC-004 / SEC-004-PY — assert against the Python surface, not just the Rust string); each `ConsumerError` variant maps to the right subtype + `code`. Marshaling: a Python dict with None / int / float / nested → the expected `minijinja::Value` (lossless — FR-003a). A Python-side test also confirms a Pydantic `ValidationError` carrying a secret in the rejected `input` does NOT surface it (mapper uses `msg` only — SEC-004-PY).
- [X] T009 [P] [US1] Python-side render tests in `packages/python/tests/test_render.py` (pytest, against the built wheel): a Pydantic Vars model with a `@field_validator`; US1.valid (valid data → `RenderResult` with text + name + variant + 64-hex `template_hash`/`render_hash`); guard-plumb (pass a guard config → `RenderResult.guard` present, vs default → `None` — plumbing only, NOT re-testing kernel guard logic); US1.invalid (a field violates → `PromptValidationError` with a row naming the field + `code=="validation"`, and assert NO render happened); assert the public types are `RenderResult`/`PromptingPressError` only — no `pydantic.ValidationError` / Rust type reachable (SC-006). **Three-sets gap test**: a Vars field misnamed vs the prompt's `variables` → validation passes but render raises `PromptRenderError` with `code=="undefined_variable"` (loud, not a silent empty render).

### Implementation for US1

- [X] T010 [US1] Create `crates/prompting-press-py/src/render.rs`: `#[pyfn] render(reg, name, vars_model_or_instance, data=None, variant=None, guard=None) -> RenderResult`. Resolve `name` via the consumer `Registry::get` (absent → `UnknownPromptError`); **validate (Q1)**: call `model_validate(data)` on the model (or re-validate the instance) in Python, catching `ValidationError` → `PromptValidationError` BEFORE any render (FR-002); on success `marshal::to_kernel_value(dumped)` → call **`prompting_press_core::render(def, variant, value, &guard)`** DIRECTLY (the consumer's generic-`V` render needs a garde `Validate` Rust type the binding does not have — critique E1; calling the kernel is still zero engine logic, Principle I). Map the returned **`KernelError` through the consumer's `ConsumerError::from` scrubber** (preserves SEC-004 — critique E2), then `ConsumerError` → the exception hierarchy. Return a `RenderResult` pyclass surfacing the kernel result 1:1 (text/name/variant/template_hash/render_hash/guard).
- [X] T011 [US1] In `render.rs`, add `#[pyfn] get_source(reg, name, variant=None) -> str` delegating to `prompting_press::get_source` (FR-010; no vars → no validation). Define the `#[pyclass] RenderResult` with read-only `#[pyo3(get)]` accessors.
- [X] T012 [US1] Build + run: `mise exec -- cargo test -p prompting-press-py` (T008); `mise exec -- maturin develop -m crates/prompting-press-py/Cargo.toml`; `mise exec -- pytest packages/python/tests/test_render.py` (T009). clippy/fmt the new Rust files.

**Checkpoint**: US1 functional — validate-then-render + get_source from Python, no leaks (MVP); the FFI marshaling path works end to end.

---

## Phase 4: User Story 2 — Dual-input loader (Priority: P2)

**Goal**: Load the same prompt from YAML, JSON, or a constructed Pydantic object into the one
representation, with identical downstream behavior — by REUSING the Rust consumer's loader via FFI (Q3).

**Independent Test**: quickstart US2 (YAML/JSON/object parity SC-003; malformed → `LoadError`).

### Tests for US2 ⚠️ (write first, expect fail)

- [X] T013 [P] [US2] Python-side loader tests in `packages/python/tests/test_loader.py`: US2.parity (load the same logical prompt via `load_yaml(text)`, `load_json(text)`, and `insert(PromptDefinition.model_validate(obj))` → render each with identical inputs → identical text + provenance — **SC-003**); US2.malformed (invalid YAML/JSON or shape-violating data → `LoadError`, nothing partially loaded — confirm the registry has no entry afterward); US2.norway (YAML `no`/`off` → parsed as STRING not bool, inherited from the Rust loader's Norway-safe parser — research D2/spec-003). Reuse `schemas/jsonschema/fixtures/valid/*.json` as JSON inputs + equivalent YAML.

### Implementation for US2

- [X] T014 [US2] In `registry.rs`, add `load_yaml(&mut self, text: str)` and `load_json(&mut self, text: str)` that marshal the TEXT to `prompting_press::Registry::load_yaml` / `load_json` (Q3 — the consumer owns parsing); map the consumer's `Load` error → `LoadError`; on error insert NOTHING (FR-007). Extend `insert(definition)` to take a generated-Pydantic `PromptDefinition`, `model_dump_json()` it, and route through the consumer's `load_json` (one loader, one representation — FR-005/006/008).
- [X] T015 [US2] Build + run: `mise exec -- maturin develop ...`; `mise exec -- pytest packages/python/tests/test_loader.py`; confirm T013 (esp. SC-003 parity + Norway-safe). Parity holds because the SAME Rust loader handles all three paths.

**Checkpoint**: US1 + US2 — render + dual-input loading, parity structural.

---

## Phase 5: User Story 3 — Agreement + provenance lint (Priority: P2)

**Goal**: `check(registry)` from Python — the headline guarantee as a CI lint, surfaced over the
consumer's `check`: template referenced-roots ⊆ declared `variables`, and untrusted/external-without-guard.
Pure, pass/fail, deterministic order.

**Independent Test**: quickstart US3 (undeclared-var SC-004; untrusted-without-guard SC-005; clean passes; pure).

### Tests for US3 ⚠️ (write first, expect fail)

- [X] T016 [P] [US3] Python-side check tests in `packages/python/tests/test_check.py`: US3.clean (well-formed registry → `report.passed()` True, empty findings); US3.undeclared (template references a var not in `variables` → a `Finding` with `kind=="undeclared_variable"` naming prompt/variant/var — **SC-004**); US3.untrusted (a prompt declaring an untrusted/external field with no `meta.guard` → `kind=="untrusted_without_guard"` naming prompt/field — **SC-005**); US3.reserved (a variant literally named `default` → `kind=="reserved_variant_name"`); US3.analysis (a template using an excluded feature like `{% include %}` → `kind=="analysis_error"`, no crash); US3.pure (snapshot the registry before/after `check` → unchanged, nothing rendered — FR-019).

### Implementation for US3

- [X] T017 [US3] Create `crates/prompting-press-py/src/check.rs`: `#[pyfn] check(reg) -> CheckReport` delegating to `prompting_press::check`; `#[pyclass] CheckReport` with `findings: list[Finding]` + `passed() -> bool`; `#[pyclass] Finding` with read-only `prompt`, `variant: Option<str>`, `kind: str` (stringify the consumer's `FindingKind` discriminants → `undeclared_variable`/`untrusted_without_guard`/`reserved_variant_name`/`analysis_error`), `detail`. Preserve the consumer's deterministic (BTreeMap/BTreeSet) order. No re-derivation — the consumer/kernel own the analysis (C-01).
- [X] T018 [US3] Build + run: `mise exec -- maturin develop ...`; `mise exec -- pytest packages/python/tests/test_check.py`; confirm T016 (all finding kinds + purity).

**Checkpoint**: the headline lint works from Python; US1+US2+US3 independently functional.

---

## Phase 6: User Story 4 — Composition (Priority: P3)

**Goal**: Assemble a multi-message prompt as an ordered array of (prompt, vars, variant) →
`list[Message{role, text}]`, via `from_messages` / `append`, never `.chain()`.

**Independent Test**: quickstart US4 (N entries → N ordered messages SC-008; one invalid → no partial; empty → []).

### Tests for US4 ⚠️ (write first, expect fail)

- [X] T019 [P] [US4] Python-side composition tests in `packages/python/tests/test_compose.py`: US4.order (`Composition.from_messages([...])` of N entries → `resolve(reg)` → exactly N `Message(role, text)` in input order, each rendered with its own validated vars — **SC-008**); US4.partial (one entry's vars fail validation → exception at append/resolve, NO partial message list returned as success); US4.empty (`Composition()` → `resolve` → `[]`); assert NO `.chain()` method exists on the class (FR-013).

### Implementation for US4

- [X] T020 [US4] Create `crates/prompting-press-py/src/compose.rs`: `#[pyclass] Message { role, text }` (read-only); `#[pyclass] Composition` (a binding-owned ordered list of marshaled `(name, value, variant)` entries — NOT the consumer's `Composition<V>`, which is generic over a garde type the binding lacks — critique E1) with `#[new]`, a `from_messages(entries)` classmethod/constructor, `append(name, vars, variant=None)` (eager-validate — option (a), like spec-003: `model_validate` the entry's vars NOW + marshal, raising `PromptValidationError` on failure; nothing stored on failure), and `resolve(reg) -> list[Message]`. The resolve loop is the only binding-side orchestration (~10 lines of glue, NOT shared-core logic): for each entry in order, `Registry::get` the prompt (absent → `UnknownPromptError`), call `prompting_press_core::render` DIRECTLY with the stored value, `role` from the def's role; one entry's failure propagates as the exception (`KernelError` via the consumer scrubber), partial result discarded — no partial-as-success. NO `.chain()` (FR-013).
- [X] T021 [US4] Build + run: `mise exec -- maturin develop ...`; `mise exec -- pytest packages/python/tests/test_compose.py`; confirm T019.

**Checkpoint**: all four stories functional from Python.

---

## Phase 7: Polish & Cross-Cutting

- [X] T022 [P] Python package facade in `packages/python/python/prompting_press/__init__.py`: re-export the compiled symbols (`Registry`, `RenderResult`, `Message`, `Composition`, `CheckReport`, `Finding`, `render`, `get_source`, `check`) + the exception hierarchy (`PromptingPressError`, `PromptValidationError`, `PromptRenderError`, `UnknownPromptError`, `LoadError`), and re-export `PromptDefinition` from `.generated`. Set `__all__` + source `__version__` from package metadata. Do NOT hand-edit `generated/` (C-07).
- [X] T023 [P] Docs: a module docstring / `packages/python/README.md` quickstart documenting the public Python API (registry, render, check, compose), the C-06 normalization boundary (native types don't leak; exception hierarchy + shared `code` vocabulary), the C-01/C-02 "marshals to the shared core, no engine logic" stance, the three-sets invariant (Vars field names must match the prompt's `variables`; a mismatch → loud `undefined_variable`, not silent), and that the package does no I/O / carries `output_model` as metadata only / ships NO token counter (C-03 / F4). **Guard-usage doctrine (decided 2026-06-27)**: document `guard` as the **system-prompt addendum** — single render → route `RenderResult.guard` into your system prompt and send `text` as the user message; multi-message → place the guard as its own `system` message. The library never assembles the request body (Principle III); `guard` and `text` stay separate (no `composed` field — see roadmap Deferred). Cite "roadmap decision C-NN", NOT "constitution C-NN".
- [X] T024 Build the distributable wheel + fresh-env import (SC-009): `mise exec -- maturin build -m crates/prompting-press-py/Cargo.toml` → an `*-abi3-*.whl`; in a clean venv `pip install` it and `import prompting_press` + run a one-line render. Confirm the wheel tag is abi3 (one wheel, CPython 3.10+).
- [X] T025 Full local gate suite — `mise exec -- moon run :build`; `mise exec -- cargo test -p prompting-press-py`; `mise exec -- pytest packages/python/tests`; `mise exec -- moon run ci:check-ffi --force` (pyo3/pythonize ONLY in `-py`); `ci:check-floating-versions`; `ci:check-advisories` (Rust) + `ci:check-advisories-py` (Python deps, T028); `schemas:codegen-check --force`; `cargo clippy -p prompting-press-py --all-targets -- -D warnings`; `cargo fmt --check`. Confirm NO token-counting surface (SC-010 / F4): `rg -n "count_tokens|token_count|TokenCount|count-tokens" packages/python/python crates/prompting-press-py/src` finds nothing (narrow patterns — avoid a false positive on incidental "token" substrings in docstrings/comments, per analyze C1). All green.
- [X] T026 Walk quickstart.md end-to-end and confirm every SC-001…SC-010 has a passing backing test; note any gap. (No SC-009-style token criterion — token surface dropped, F4.)
- [X] T027 [P] Reconcile the stale roadmap "token hook" line (analyze U1; spec Assumptions §Token-surface): in `.specify/memory/roadmap.md`, amend the 004 (and 005) `Scope (in)` to drop "token hook" (consistent with spec-003 F4 — token surface dropped + deferred to the Deferred "Token budgeting / truncation" entry). Owner-task so the amendment is not silently dropped; do at the roadmap-debrief/sync step if not earlier. (Edits the governance ledger, not code — no FFI/codegen impact.)
- [X] T028 Python dependency advisory gate (FR-025 / SC-011; security review SEC-101): create `scripts/ci/check-advisories-py.sh` that runs `pip-audit` (via `uv run`/`uvx`, reading `packages/python/uv.lock` — `uv` 0.11.8 is the pinned tool) and fails on a known CVE; register it as a `check-advisories-py` task in `ci/moon.yml` (mirror the existing `check-advisories` block: `runFromWorkspaceRoot: true`, `cache: false`); add it to the CI workflow and to the T025 gate list. Pin the audit tool (no floating version — `ci:check-floating-versions`). Mirrors the Rust `ci:check-advisories` (cargo-deny) for the Python side.

---

## Dependencies & Execution Order

### Phase dependencies
- **Setup (T001–T003)**: start immediately. T001→T002 (Cargo then pyproject); T003 after both (baseline build/gates).
- **Foundational (T004–T007)**: after Setup. **BLOCKS all stories** (everything raises the exception hierarchy / marshals / uses `Registry`). T004 (module wiring) first; T005 (marshal) + T006 (error) are [P] (different files); T007 (registry) after T004.
- **US1 (T008–T012)**: after Foundational. The MVP — proves the FFI marshaling path.
- **US2 (T013–T015)**: after Foundational; adds loaders to `registry.rs` (independent of US1's render.rs).
- **US3 (T016–T018)**: after Foundational; `check.rs` is new (independent of US1/US2 files) but reuses `Registry`.
- **US4 (T019–T021)**: after US1 (reuses US1's kernel-direct render path in the resolve loop).
- **Polish (T022–T028)**: after the targeted stories. T027 (roadmap amendment) is doc-only; T028 (Python advisory gate) is CI wiring, independent of the binding code.

### Within each story
- Tests (T008/T009, T013, T016, T019) written first, expected to FAIL before implementation.
- marshal + error + registry before render; render before compose.

### Parallel opportunities
- Foundational T005 (marshal.rs) + T006 (error.rs) [P]. US1 test files T008 (Rust) + T009 (Python) [P].
  Polish T022 + T023 [P].
- Cross-story: clean single-implementer order is Setup→Foundational→US1→US2→US3→US4→Polish. US4 needs US1.

---

## Implementation Strategy

### MVP first (US1)
Setup → Foundational → US1 → STOP & VALIDATE: a Python app can define Pydantic Vars, register a prompt,
and `render` with validation + provenance, no native types leaking (SC-001/002/006/009). The first FFI
binding proves the marshaling path the kernel/consumer split was built to de-risk. Demoable.

### Incremental delivery
+ US2 (load YAML/JSON/object, SC-003) → + US3 (the headline `check()` lint, SC-004/005) → + US4
(composition, SC-008) → + Polish (facade, docs, wheel build, gates).

---

## Notes
- 28 tasks: Setup 3, Foundational 4, US1 5, US2 3, US3 3, US4 3, Polish 7.
- The headline value is US3 (`check()`), but US1 is the MVP that proves the FFI marshaling path.
- Keep `pyo3`/`pythonize` in `prompting-press-py` ONLY at every step (T003 baseline, T025 final). No
  engine logic — marshal to the shared core (C-01/C-02). No token surface (F4). Generated shape codegen'd (C-07).
- Commit after each task/logical group; checkpoint after each phase (agent-assign flow runs checkpoints).

---

## Tech Debt Tasks (Generated by /speckit.cleanup)

**Generated**: 2026-06-27
**Source**: Post-implementation cleanup of spec 004 (Python binding). Phase-3 reviews were
otherwise clean — no debug artifacts, dead code, TODOs, or secrets in the diff.
**Priority**: Address before next iteration; not a release blocker (the exposed path is the
trusted-caller `insert(dict)` path, and render/compose are already depth-bounded by pydantic-core).

- [ ] TD001 Bound or document recursion depth on the `insert(dict)` / marshal path
  (`crates/prompting-press-py/src/marshal.rs:to_kernel_value`, reached from `registry.rs:insert`'s
  `depythonize`). A pathologically deep Python dict could overflow the Rust stack during the
  recursive descent → process abort (DoS). The render/compose paths run pydantic validation first
  (pydantic-core enforces a recursion guard), so this is the one unguarded entry. Either add a
  defensive depth cap in `definition_to_json`/`to_kernel_value`, or document the depth assumption on
  `insert`. Source: security review L-1.
