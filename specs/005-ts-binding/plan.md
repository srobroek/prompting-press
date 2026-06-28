# Implementation Plan: TypeScript binding (`prompting-press-node` → `packages/typescript`)

**Branch**: `005-ts-binding` | **Date**: 2026-06-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-ts-binding/spec.md`

## Summary

Build out `prompting-press-node` — the **second FFI binding** — filling the existing napi-rs stub crate
with a napi marshaling layer over the spec-003 Rust consumer (`prompting-press`), and round out the
`packages/typescript` package so `import 'prompting-press'` exposes the four capabilities in **TypeScript
idiom**: a **Zod v4** typed-Vars facade (validation owned at render — clarified Q1), a **dual-input
loader reused from the Rust consumer via FFI** (YAML/JSON text marshaled in; constructed object → JSON →
the consumer's loader — clarified Q3), a **registry** + **`check()`** agreement/provenance lint,
ergonomic **render/getSource + `fromMessages` composition**, and **error normalization** to
`[{field, code, message}]` thrown as a **`PromptingPressError` `Error`-subclass hierarchy** (clarified
Q2). The binding adds **no** render/agreement/variant/hash logic — those are FFI calls into the shared
Rust core (Principle I / C-02); render byte-parity (incl. provenance hashes matching the Python binding +
Rust consumer) is therefore structural, not re-tested in TS. This is the binding that makes the FFI
boundary real for the spec-006 conformance corpus.

**Technical approach** (from Phase 0): expose `#[napi]` classes (`Registry`, `RenderResult`,
`CheckReport`/`Finding`, `Composition`/`Message`) and `#[napi]` functions over the shared Rust core using
**napi 3.x / napi-derive 3.x** (already pinned at `"3"`; latest = **napi 3.9.4 / napi-derive 3.5.7**, which version independently — pin each exact, see below). The
Zod schema is validated in TS (`safeParse`) at the render boundary; on success the plain validated JS
object crosses napi and is marshaled into the kernel value type. **Render + compose marshal to the
*kernel* directly** (`prompting_press_core::render`), mirroring the 004 decision: the *consumer's*
`render<V>` / `Composition::append<V>` are generic over a garde `Validate` Rust type the binding does not
have (it carries an already-Zod-validated, type-erased value). **The loader, registry, `check`, and
`getSource` are still reused from the consumer** (they need no garde type). The TS-side `ZodError` maps to
`PromptValidationError` (copying issue `message` + `path` only — SEC-004-PY-equivalent); the kernel's
`KernelError` is routed through the consumer's existing tested `From<KernelError> for ConsumerError`
scrubber (preserving the SEC-004 fixed-message scrub) and then translated into the `PromptingPressError`
subtypes with the shared closed `code` vocabulary. Calling the kernel directly is still **zero engine
logic in the binding** (Principle I); the only binding-side orchestration is the compose resolve loop
(~10 lines of glue). Packaged as a **napi-rs native addon** built by **@napi-rs/cli 3.7.2**
(`napi build --platform --esm`), shipped **ESM-only** (clarified Q8) with per-platform native binaries as
`optionalDependencies` (clarified Q5). The TS `PromptDefinition` shape stays codegen'd from the JSON
Schema via `json-schema-to-typescript 15.0.4` (Principle VII; already wired). No new Rust deps reach the
kernel or Rust consumer → `ci:check-ffi` stays green (the gate **already asserts `napi`** — `FFI_CRATES=("pyo3" "napi")`, shipped spec 001; FR-022 is verify-not-extend — analyze F1).

## Technical Context

**Language/Version**: Rust (workspace lockstep, pinned `1.95.0`) for the binding crate; TypeScript
**5.9.2** (repo-pinned) targeting **Node 20+** (ESM; matches the scaffold `engines.node: ">=20"`) for the package. Native addon via Node-API (napi).

**Primary Dependencies** (all version-verified this cycle against crates.io / npm directly — see
research.md):
- `napi = "3.9.4"`, `napi-derive = "3.5.7"` — crates.io latest 3.x (the two crates version independently; verified 2026-06-24).
  Pin **exact** (crate currently declares floating `"3"`) so the `ci:check-floating-versions` gate stays
  green — the napi-pin reconciliation flagged in the spec (resolved here: pin exact).
- `prompting-press` (Rust consumer, path dep — already present) and `prompting-press-core` (kernel, path
  dep — already present). The binding marshals render/compose to the **kernel** (`prompting_press_core::
  render`); reuses the **consumer** for loader/registry/`check`/`getSource` and the
  `From<KernelError>`/`ConsumerError` error-normalization + SEC-004 scrub.
- `serde_json` (present transitively) for the constructed-object → JSON → `load_json` path and the
  JS-value → kernel-value marshaling intermediate. napi's own `serde-json`/serde support bridges
  JS ↔ `serde_json::Value`.
- JS build: **@napi-rs/cli `3.7.2`** (npm latest; matches scaffold) for `napi build`/`prepublish`.
  Runtime dep: **Zod `4.4.3`** (npm latest v4; this spec adds it — scaffold `dependencies` is empty).
  Codegen dev-dep: **json-schema-to-typescript `15.0.4`** (npm latest; matches scaffold) +
  **typescript `5.9.2`** (matches scaffold). A test runner (Vitest or node:test — decided in research.md
  D6) is added as a dev-dep.

**Storage**: N/A — no I/O (Principle III). The caller hands in already-read YAML/JSON text or a
constructed object.

**Testing**: `cargo test -p prompting-press-node` (Rust-side marshaling unit tests, if napi test harness
permits under `Env`) + a **TS test runner** against the built native addon (render/check/compose/error
scenarios — quickstart.md), run via moon. A `ci:test-node` gate builds the addon + runs the TS tests (the
spec-004 I1 lesson — the OS-matrix `cargo build` does not run binding tests).

**Target Platform**: Native Node addon (per-platform `.node` binaries), across the existing 3-OS build
matrix; ESM-only consumption on Node 20+.

**Project Type**: Library binding (napi crate `prompting-press-node` + the `packages/typescript`
distribution within the existing workspace).

**Performance Goals**: None specified/needed — synchronous in-process marshal + FFI call. SCs are
correctness/parity/packaging, not perf.

**Constraints**: `napi` ONLY here (C-02, CI-gated — the gate already asserts `napi`); no engine logic in the
binding (C-01 — marshal to the core); no I/O / no LLM / no request-body / no output parsing / **no token
counting** (C-03 / F4); native error types (`ZodError`, Rust errors) never cross FFI onto the public API
(C-06); `check()` pure; generated TS shape codegen'd, never hand-edited (C-07); ESM-only;
`undefined`/absent → field-not-present, `null` → JSON null (clarified Q6, matched to the Python binding).

**Scale/Scope**: One binding crate (build out the stub) + the TS package; ~5 marshaling areas (registry,
render/getSource, check, compose, error) mirroring the consumer's 5 modules and the 004 binding's layout;
the generated shape already present; pin napi exact + add Zod + a test runner; native build + ESM
packaging wiring; VERIFY `ci:check-ffi` covers napi (already does); add a `ci:check-advisories-node` + `ci:test-node` gate.
No kernel or consumer changes; no relocation.

**Unknowns**: none open. napi **3.9.4** + napi-derive **3.5.7** (crates.io; version independently), Zod **4.4.3**, @napi-rs/cli **3.7.2**,
json-schema-to-typescript **15.0.4**, typescript **5.9.2** all re-verified against crates.io / npm
**directly** this cycle (the project's fabricated-subagent-version guard). Remaining plan-time
confirmations are napi 3.x API-shape details + the Zod v4 issue API + the JS↔serde marshaling primitive
(Phase 0 research.md D-items).

## Constitution Check

*GATE: must pass before Phase 0. Re-checked after Phase 1 design.*

| Principle / Decision | Requirement | This plan | Status |
|---|---|---|---|
| **I — Shared core, no duplication** (C-01) | Render/agreement/variant/hash live once in the kernel | Render/compose MARSHAL to the kernel's `render` directly (the consumer's generic-`V` render needs a garde type the binding lacks); loader/registry/`check`/`getSource`/error-scrub reused from the consumer; zero render/agreement/variant/hash logic in the binding; render parity (+ hash parity vs Python/Rust) structural, not re-tested | ✅ PASS |
| **II — FFI isolation** (C-02) | `napi` ONLY in `prompting-press-node`; kernel + Rust consumer FFI-free | `napi`/`napi-derive` live ONLY in the binding crate; path deps don't pull FFI into `-core`/`prompting-press`; `ci:check-ffi` **already asserts napi** (shipped spec 001) and stays green (SC-007) | ✅ PASS |
| **III — Minimal boundary** (C-03) | No I/O, LLM, request-body, token-count, output-parse | NO token counter (F4); no I/O (caller pushes text/objects); `outputModel` metadata only, never parsed | ✅ PASS |
| **VI — Per-language idiom** (C-06) | Native validation system; errors normalized; native types don't leak | Zod v4 is the native system; `ZodError` + Rust `ConsumerError` → `PromptingPressError` `[{field,code,message}]` JS `Error` subclasses; `fromMessages` array, NOT `.chain()` | ✅ PASS |
| **VII — JSON Schema single source** (C-07) | Codegen'd shape; dual-input into one shape | TS `PromptDefinition` codegen'd from the JSON Schema (freshness-gated); dual-input reused from the consumer's one loader; no parallel hand-shape | ✅ PASS |
| **IV — agreement check / provenance** (C-04/C-09) | The sound check + provenance lint, pure | `check()` surfaced to TS over the consumer's lint; pure, no mutation/render (FR-019) | ✅ PASS |
| **Scope Discipline** (R1) | No new pluggable interface | NO new seam — registry/composition/errors are plain types; token hook already dropped (F4) | ✅ PASS |
| **Boundary defense** | No I/O/LLM/version-axis/etc. | none proposed | ✅ PASS |

**Result**: PASS (pre-Phase-0 and post-Phase-1). No violations; no Complexity Tracking entries needed.

## Project Structure

### Documentation (this feature)

```text
specs/005-ts-binding/
├── plan.md              # this file
├── research.md          # Phase 0 — D1..D7 (napi 3.x classes, JS↔serde bridge, Zod v4 issue map, ESM packaging, test runner, ci:check-ffi-napi)
├── data-model.md        # Phase 1 — binding types (#[napi] classes + the TS-facing surface)
├── quickstart.md        # Phase 1 — validation scenarios (TS tests against the built addon)
├── contracts/
│   └── ts-api.md         # Phase 1 — public TypeScript API contract
├── memory.md · memory-synthesis.md · checklists/requirements.md
```

### Source Code (repository root)

```text
crates/prompting-press-node/      # THE BINDING (this spec's work — builds out the existing stub)
├── Cargo.toml                    # pin napi 3.9.4 + napi-derive 3.5.7 exact (independent versions; was floating "3")
├── src/
│   ├── lib.rs                    # #[napi] module: register classes + functions (replaces the stub fn)
│   ├── registry.rs               # #[napi] Registry over consumer Registry: loadYaml/loadJson/insert
│   ├── render.rs                 # render()/getSource() #[napi]: (Zod validated in TS) -> marshal -> kernel::render
│   ├── check.rs                  # check() #[napi] -> CheckReport/Finding classes (deterministic order preserved)
│   ├── compose.rs                # Composition #[napi] (fromMessages/append/resolve) -> [Message]; no .chain()
│   ├── error.rs                  # ConsumerError -> PromptingPressError hierarchy (+ SEC-004 scrub); JS Error subclasses
│   └── marshal.rs                # napi/serde bridge: JS value <-> kernel value type; lossless null/undefined/number/bigint/nested
└── (napi index.d.ts generated by `napi build` into packages/typescript)

packages/typescript/              # THE DISTRIBUTION (build out the scaffold)
├── package.json                  # pin napi exact; + zod 4.4.3 runtime dep; + test runner dev-dep; ESM-only (already type:module)
├── src/
│   ├── index.ts                  # the Zod-facing facade: render/check/compose wrappers + the PromptingPressError hierarchy + re-export the generated type
│   └── generated/prompt-definition.ts  # GENERATED — do not hand-edit (codegen freshness-gated)
└── test/                         # TS tests: US1-US4 + boundary scenarios (quickstart) against the built addon
```

**Structure Decision**: Build out the existing `prompting-press-node` stub crate (already a `cdylib` with
path deps on the consumer + kernel and napi 3.x declared) plus the `packages/typescript` scaffold
(@napi-rs/cli build + generated TS shape already wired). No new crate, no relocation. Cargo.toml change:
pin napi/napi-derive exact. The TS package gains its Zod-facing facade (`src/index.ts`), the error
hierarchy, the test suite, and the Zod + test-runner deps; the generated shape is untouched (regenerated
only via `codegen.mjs`). A thin TS wrapper layer (`src/index.ts`) sits over the napi-generated binding to
host the Zod `safeParse`-at-render-boundary logic and the JS `Error` subclasses — the napi addon exposes
the marshaling + kernel delegation; the Zod facade is TS-side (Zod cannot live in Rust). This split is the
TS analogue of the 004 Python facade `__init__.py`.

## Complexity Tracking

> No constitution violations; no entries required.

Two items worth noting (not violations):
- The binding crate is the **single** place `napi`/`napi-derive` appear — the intended C-02 idiom (the
  binding layer IS the FFI boundary). `marshal.rs` concentrates all JS↔kernel value translation so the
  FFI boundary is auditable in one file.
- Unlike 004 (where the Pydantic facade lives entirely in the compiled extension), the Zod validation
  must live in **TS** (`src/index.ts`), because Zod is a TS library — the napi addon can't host it. So the
  public surface is a thin TS wrapper over the napi binding. This is idiomatic and still zero-engine-logic
  (the wrapper does `safeParse` + error mapping + delegates to the addon, which delegates to the kernel).

### Verified-this-cycle (so a future reader doesn't re-litigate)

- **napi** crates.io latest 3.x = **3.9.4**, **napi-derive** = **3.5.7** (they version INDEPENDENTLY;
  verified 2026-06-24); crate currently declares floating `"3"` → **pin each exact** (resolves the floating-version concern; the
  `ci:check-floating-versions` gate covers Cargo manifests).
- **Zod** npm latest = **4.4.3** (v4 — the clarified Q7 target; the `ZodError` issue API the mapper reads
  is v4's). **@napi-rs/cli** npm latest = **3.7.2** (matches scaffold). **json-schema-to-typescript** npm
  latest = **15.0.4** (matches scaffold). **typescript** **5.9.2** (matches scaffold).
- All version checks were made by querying crates.io / npm **directly** (not via a research subagent),
  per the project's systemic-fabrication guard. The `mcp-package-version` tool confirmed the three npm
  packages; crates.io API confirmed napi 3.9.4.
- **Plan-time research items** (research.md): D1 napi 3.x `#[napi]` class/method patterns + how to expose
  a class holding a Rust struct; D2 the JS-value ↔ `serde_json::Value`/kernel-value marshaling primitive
  (napi serde support) + the null/undefined/bigint rules (Q6); D3 the Zod v4 `.issues` shape for the
  `{field,code,message}` mapper; D4 the JS `Error`-subclass hierarchy pattern (and how napi surfaces a
  thrown error — `napi::Error` vs a JS-side throw); D5 ESM-only packaging + per-platform optionalDeps via
  `napi prepublish`; D6 the TS test runner choice (Vitest vs node:test) + how to load the built addon in
  tests; D7 the `ci:check-ffi` extension to assert `napi` + a `ci:check-advisories-node` + `ci:test-node`.
