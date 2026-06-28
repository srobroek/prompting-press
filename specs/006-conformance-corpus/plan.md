# Implementation Plan: Conformance corpus + cross-language hardening

**Branch**: `006-conformance-corpus` | **Date**: 2026-06-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/006-conformance-corpus/spec.md`

## Summary

Build a **shared, language-neutral conformance corpus** (proposed top-level `conformance/`) plus **one
thin runner per binding** (the Rust consumer `prompting-press`, the Python binding `prompting-press-py` /
`packages/python`, the TypeScript binding `prompting-press-node` / `packages/typescript`), wired as a
**CI gate**. The corpus proves the two per-binding seams the shared core cannot self-verify: (1)
**marshaling parity** — the same logical input through each binding yields byte-identical rendered text +
identical `template_hash`/`render_hash`, exercised over dates, Decimal/high-precision numerics, nested
models, `null`/`undefined`/`None`/absent, and integer-vs-float; and (2) **schema round-trip parity** — a
schema-valid prompt document is accepted and a schema-invalid one is rejected identically across all
three bindings, driven through each binding's own loader. **Render parity is structural (C-01) and is NOT
re-tested**; the spec-002 engine-regression render fixtures stay byte-unchanged. The corpus adds **tests +
a gate, never library capability or engine logic** (C-02 / Principle III): no new public API, no I/O in
the library, no new pluggable seam.

**Technical approach** (from Phase 0): the corpus is a directory of JSON fixture files in two families —
`marshaling/` (each = a prompt definition + a typed-input descriptor in **canonical serialized form** +
the expected rendered text + expected `template_hash`/`render_hash`) and `schema/` (each = a prompt
document + expected accept/reject verdict, reusing the existing `schemas/jsonschema/fixtures/{valid,
invalid}/` set). The **assertion model is cross-check + golden tripwire** (clarified Q2): each runner
asserts its binding's output equals the committed golden value in the fixture; because all three assert
against the *same* committed value, cross-binding equality is guaranteed transitively, and the golden also
trips on a lockstep kernel regression. The **canonical-serialized-form** decision (clarified Q3) is what
makes a date / Decimal testable across languages: the fixture pins the serialized form the kernel sees
(ISO-8601 string for a date, decimal-as-string for a Decimal), and each runner constructs its native type
(Python `datetime`/`Decimal`, JS `Date`, Rust `chrono`) that MUST marshal to that form — driving each
binding's **real marshaling bridge** (`pythonize::depythonize`+Pydantic `mode="json"`; napi `serde-json`;
the consumer's `Serialize`). A divergence here is the hardening finding the corpus exists to surface.

Each runner uses the binding's existing public render/loader entry points (verified as-built): Rust
`prompting_press::{render, get_source, Registry::{load_yaml, load_json, insert}, check}`; Python
`prompting_press.{render, get_source, Registry, check}` with `Registry.{load_yaml, load_json, insert}`;
TypeScript `{render, getSource, Registry, check}` with `Registry.{loadYaml, loadJson, insert}`. The
runners are **test harnesses, not the library** — they may read fixture files (the library still does no
I/O). Gate logic lives in moon tasks + `scripts/ci/*.sh` and is invoked from `.github/workflows/ci.yml`,
mirroring `ci:test-python` / `ci:test-node` (FR-012/FR-013), and the **Rust consumer participates as a
first-class binding** (FR-015) — which also closes a found gap: the consumer's own `cargo test -p
prompting-press` does not currently run in CI (only `-py` and `-node` crates do).

## Technical Context

**Language/Version**: Rust (workspace lockstep, pinned `1.95.0`) for the Rust conformance runner; Python
**3.10+** (the `abi3-py310` floor) for the Python runner (`pytest`); TypeScript **5.9.2** / Node **20+**
(ESM, `node:test`) for the TS runner. No new language surface — the runners use each binding's existing
public API.

**Primary Dependencies**: none new in the shipped libraries. The corpus reuses what's already pinned:
- Rust runner: `prompting-press` (consumer, path dep) — its `render`/`get_source`/`Registry`/`check` +
  `RenderResult.{template_hash, render_hash}`. A **data-driven passthrough Vars** newtype is needed
  because the consumer's `render<V>` is generic over `V: Serialize + Validate` (garde): the runner wraps
  each fixture's already-marshaled `serde_json::Value` in a `#[derive(Serialize)]` + no-op-`Validate`
  newtype so a single fixture loop can render arbitrary shapes. (Test-only; lives in the runner, not the
  consumer — zero engine logic added, C-02.) `serde_json` (already present) reads the fixture input.
- Python runner: `prompting_press` (the built extension) + `pytest` (already the binding's test runner) +
  stdlib `json`/`datetime`/`decimal`. No new dep.
- TS runner: `prompting-press` (the built addon + facade) + `node:test` (already the binding's zero-dep
  runner) + the built-in `Date`/`JSON`. No new dep. **No JS decimal library** is added (Principle III /
  the no-floating-versions + minimal-dep discipline) — the Decimal case uses the canonical serialized
  form (string), which is exactly the cross-language-representable choice from clarify Q3.

**Storage**: N/A. The library does no I/O (Principle III). The runners read fixture files as test
harnesses; the library under test still receives pushed-in data.

**Testing**: three runners over the one shared corpus —
- Rust: an integration test (`crates/prompting-press/tests/conformance.rs`) iterating the corpus.
- Python: `packages/python/tests/test_conformance.py` (pytest), run by the existing `ci:test-python`.
- TS: `packages/typescript/test/conformance.test.mjs` (`node:test`), run by the existing `ci:test-node`.
- Plus a moon gate that runs all three (and a Rust leg, since the consumer isn't CI-tested today).

**Target Platform**: the existing 3-OS CI matrix is unaffected; the conformance runners are OS-independent
(hashes are SHA-256 over canonical strings — FR-004 pins OS/arch stability) and run on one Linux leg each,
consistent with how `test-python`/`test-node` are scoped.

**Project Type**: cross-language test corpus + CI gate within the existing polyglot workspace. No new
crate, no new package, no relocation.

**Performance Goals**: none. Synchronous in-process render + hash over a small fixture set. SCs are
correctness/parity/coverage/gate-enforcement, not perf.

**Constraints**: render parity NOT re-tested (C-01 — corpus tests marshaling + schema acceptance only);
zero engine logic in any binding or runner (C-02 — runners call existing public APIs); no I/O / LLM /
token surface / new public API (Principle III); golden set stays tiny, never a render-parity corpus
(FR-016); failure output names binding+fixture+divergence-kind without leaking bound values beyond the
fixture's own content (FR-014 / SEC-004 posture); any helper dep pinned exact (`ci:check-floating-versions`
scans whole manifests) — preference is **zero** new deps.

**Scale/Scope**: one `conformance/` dir (~5 marshaling cases × the named hard types + the existing 10
schema fixtures); three thin runners (~one file each); moon-task + `ci.yml` wiring; one test-only
passthrough-Vars newtype in the Rust runner; **no kernel/consumer/binding source changes** beyond adding
the runners and the gate (the consumer's `cargo test` gets wired into CI as part of FR-015).

**Unknowns**: none blocking. Plan-time confirmations (Phase 0 research.md D-items): the exact canonical
serialized forms for date + Decimal (D1); whether the marshaling fixture format carries a per-field
`type` tag so each runner knows which native type to construct (D2); the cross-check-via-golden mechanics
+ golden authoring/regeneration (D3); the Rust passthrough-Vars newtype shape (D4); the CI gate topology —
dedicated `conformance` task vs folding into existing jobs + a Rust leg (D5); whether the existing schema
fixtures are sufficient or need a YAML twin for the YAML-path round-trip (D6).

## Constitution Check

*GATE: must pass before Phase 0. Re-checked after Phase 1 design.*

| Principle / Decision | Requirement | This plan | Status |
|---|---|---|---|
| **I — Shared core, structural parity** (C-01) | Render/agreement/variant/hash live once in the kernel; render parity is structural, not test-enforced | The corpus tests MARSHALING + SCHEMA ACCEPTANCE, the two per-binding seams the core can't self-verify; it does NOT add render-parity fixtures; the spec-002 render set stays byte-unchanged (FR-016, SC-006); golden set stays tiny | ✅ PASS |
| **II — FFI isolation** (C-02) | `pyo3`/`napi` only in binding crates; no engine logic in bindings | Runners call each binding's EXISTING public API; add no render/agreement/variant/hash logic; the Rust runner's passthrough-Vars newtype is test-only no-op glue; `ci:check-ffi` stays green (SC-006) | ✅ PASS |
| **III — Minimal boundary** (C-03) | No I/O, LLM, request-body, token-count, output-parse in the library; no new public API | Corpus adds fixtures + test runners + a gate; runners read fixtures AS TEST HARNESSES (the library still does no I/O); no token surface; no new public API on any binding (FR-018) | ✅ PASS |
| **VII — JSON Schema single source** (C-07) | One schema; round-trip accept/reject identical across languages | Schema round-trip is the corpus's 2nd guarantee, driven through each binding's own loader, reusing `schemas/jsonschema/fixtures/{valid,invalid}/` (FR-003/009/010/011) | ✅ PASS |
| **IV — agreement check / provenance** (C-04/C-09) | The check is pure analysis; provenance is data | Corpus exercises render/loader paths; does not mutate or re-implement `check`; pins the `template_hash`/`render_hash` provenance values | ✅ PASS |
| **V — repo canonical; git owns versioning** (C-05) | No managed version axis; provenance = per-variant hashes over strings | Corpus asserts the existing `template_hash`/`render_hash` (no `vars_hash`); adds no version axis | ✅ PASS |
| **VI — Per-language idiom** (C-06) | Native idiom per language; errors normalized | Each runner is idiomatic to its ecosystem (cargo test / pytest / node:test); schema-reject asserts the binding's normalized `[{field,code,message}]` surface | ✅ PASS |
| **Scope Discipline** (R1) | No new pluggable interface | NO new seam — the corpus is fixtures + runners + a gate; nothing pluggable introduced | ✅ PASS |
| **Boundary defense** | No I/O/LLM/version-axis/token/output-parse added | none proposed — this is a test+gate spec | ✅ PASS |

**Result**: PASS (pre-Phase-0). Re-checked post-Phase-1 below. No violations; no Complexity Tracking
entries needed.

## Project Structure

### Documentation (this feature)

```text
specs/006-conformance-corpus/
├── plan.md              # this file
├── research.md          # Phase 0 — D1..D6 (canonical date/Decimal forms, fixture type-tag, golden mechanics, Rust passthrough-Vars, CI topology, YAML round-trip twin)
├── data-model.md        # Phase 1 — the fixture schema (marshaling + schema families) + the logical-type mapping
├── quickstart.md        # Phase 1 — how to run the corpus locally + add a fixture
├── contracts/
│   └── corpus-format.md  # Phase 1 — the fixture-file contract + runner obligations + gate contract
├── memory.md · memory-synthesis.md · checklists/requirements.md
```

### Source Code (repository root)

```text
conformance/                          # THE SHARED CORPUS (new — single source of truth, all runners read it)
├── README.md                         # what the corpus is, the fixture contract, how to add a case
├── marshaling/                       # marshaling-parity fixtures (one JSON per logical case)
│   ├── date.json                     # canonical ISO-8601 serialized form; runners build datetime/Date/chrono
│   ├── decimal.json                  # canonical decimal-as-string; no JS decimal lib
│   ├── nested-model.json             # object-in-object + list-of-objects
│   ├── null-undefined-none.json      # explicit null vs undefined/None vs absent (FR-008 contract)
│   └── int-vs-float.json             # int 1 vs fractional float 2.5 (1.0-vs-1 excluded — JS-unrepresentable)
└── schema/                           # schema-round-trip fixtures (verdict per doc)
    └── manifest.json                 # maps the existing schemas/jsonschema/fixtures/{valid,invalid}/* → expected verdict (+ any YAML twin)

crates/prompting-press/
├── tests/conformance.rs              # NEW — Rust runner: iterate conformance/, render via the consumer, assert golden
└── (test-only passthrough-Vars newtype lives in this test file — Serialize + no-op Validate)

packages/python/tests/
└── test_conformance.py               # NEW — Python runner (pytest): build native datetime/Decimal, render, assert golden

packages/typescript/test/
└── conformance.test.mjs              # NEW — TS runner (node:test): build native Date, render, assert golden

ci/moon.yml                           # + a conformance task (or extend test-python/test-node + add a Rust leg — D5)
scripts/ci/
├── conformance.sh                    # NEW (shape per D5) — runs the runners; locally reproducible (FR-013)
.github/workflows/ci.yml              # + invoke the conformance gate (D5)
```

**Structure Decision**: a new top-level `conformance/` directory is the single source of truth (clarified
Q1), mirroring how `schemas/jsonschema/fixtures/` is one shared set. Each binding gets one thin runner in
its existing test location (so the existing `ci:test-python` / `ci:test-node` gates pick the Python/TS
runners up for free) plus a Rust runner in the consumer's `tests/` (newly CI-wired — FR-015). No new
crate, no new package, no relocation, no library source change beyond the runners + gate wiring. The only
non-fixture code is: three runner files, one test-only passthrough-Vars newtype (Rust), and the moon/CI
gate glue.

## Complexity Tracking

> No constitution violations; no entries required.

Three items worth noting (not violations):
- **The Rust passthrough-Vars newtype** is the one piece of non-obvious glue: the consumer's `render<V>`
  is generic over a garde-`Validate` type, but the corpus is data-driven (it has a `serde_json::Value`,
  not a compile-time Rust struct). A test-only newtype `struct RawVars(serde_json::Value)` with
  `#[derive(Serialize)]` (delegating to the inner value) and a no-op `Validate` impl lets one loop render
  every fixture. It adds **no engine logic** (validation already happened in the higher bindings; the
  corpus tests marshaling, and a no-op validate is correct here) and lives in the test file, never in the
  shipped consumer.
- **The found CI gap** (not a violation, a hardening win): `cargo test -p prompting-press` (the consumer)
  does not run in CI today — only `-py`/`-node` do, and `build` only `cargo build`s. FR-015 requires the
  Rust binding to participate, so the conformance gate adds the consumer's Rust leg, which also starts
  covering the consumer's existing tests in CI.
- **Cross-check realized via a shared golden**: rather than a runner spawning the other two languages to
  compare live (cross-process, brittle), each runner asserts against the *same committed golden* in the
  fixture. Three runners all equal to one value ⇒ equal to each other (transitive parity), and the golden
  additionally trips a lockstep kernel regression. This is simpler and more deterministic than live
  cross-process comparison and keeps each runner single-language.
- **What the Rust marshaling leg actually proves (critique E2 — be honest about it)**: because the goldens
  are GENERATED from the Rust reference binding (D3), the Rust *marshaling* runner asserts Rust output ==
  a golden that IS Rust output — so it can only fail on nondeterminism, never on a parity divergence. The
  genuine independent marshaling-parity votes are **Python-vs-golden** and **TS-vs-golden**. The Rust
  leg's real value is fourfold: (a) it is the golden *source*, (b) it runs the **schema round-trip** (a
  real independent check — the loader is not the golden source), (c) it is a **determinism** guard on the
  reference, and (d) it closes the CI gap (the consumer's tests run nowhere today). So "3-way parity" is
  precisely: two independent marshaling votes against a Rust-anchored reference + the schema round-trip in
  all three + a determinism check — not three independent marshaling votes. Stated so a reader does not
  over-credit the Rust marshaling assertion.

### Verified-this-cycle (so a future reader doesn't re-litigate)

- **As-built render/loader entry points confirmed by reading source** (not a subagent — the systemic
  fabrication guard): Rust `prompting_press::{render, get_source, check, Registry::{new, load_yaml,
  load_json, insert}}`, `RenderResult.{template_hash, render_hash}` (snake_case). Python
  `prompting_press.{render, get_source, check, Registry}`, result `.template_hash`/`.render_hash`. TS
  `{render, getSource, check, Registry}`, result `.templateHash`/`.renderHash` (camelCase getters).
- **Marshaling internals confirmed by reading source**: Python `to_kernel_value` =
  `pythonize::depythonize` → `serde_json::Value` → `minijinja::Value::from_serialize`, and Pydantic
  `mode="json"` **already stringifies `datetime`/`Decimal` deterministically** before marshaling. Node
  marshal = napi `serde-json` (`FromNapiValue for serde_json::Value`) → `from_serialize`, with `napi6`
  bigint losslessness and the documented `undefined`/absent-dropped vs `null`→JSON-null rule. **Node has
  no `Date`→string special-case** — a JS `Date` goes through napi's serde, so the canonical-serialized-form
  (ISO string) is what makes date parity hold across Python's `mode="json"` and the TS runner; the corpus
  pins exactly this.
- **CI coverage gap confirmed**: `scripts/ci/test-python.sh` runs `cargo test -p prompting-press-py`;
  `test-node.sh` runs `cargo test -p prompting-press-node`; the `build` job runs `cargo build --workspace`
  only. Nothing runs `cargo test -p prompting-press` (the consumer) → the Rust conformance leg fills this.
- **Plan-time research items** (research.md): D1 canonical serialized forms (date ISO-8601; Decimal as
  string) + confirm Python `mode="json"` output exactly matches; D2 the fixture file schema incl. a
  per-field `type` tag so each runner constructs the right native type; D3 the golden author/regenerate
  mechanism (a small generator vs hand-pinned + a freshness note); D4 the Rust `RawVars` passthrough newtype
  (Serialize delegation + no-op garde Validate); D5 the CI gate topology (dedicated task vs fold-in + Rust
  leg) + local-repro command; D6 whether the schema round-trip needs a YAML twin of a valid/invalid fixture
  to exercise the `load_yaml` path (the existing fixtures are JSON).
