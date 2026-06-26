# Implementation Plan: Engine kernel (`prompting-press-core`)

**Branch**: `002-engine-kernel` | **Date**: 2026-06-26 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/002-engine-kernel/spec.md`

## Summary

Build the `prompting-press-core` engine kernel: the single, FFI-free, validation-blind site where
Prompting Press renders prompts and emits provenance. It takes the spec-001 `PromptDefinition` shape
plus already-validated values and produces rendered text + provenance. Four capabilities: (1) a
MiniJinja render path restricted to interpolation/conditionals/loops with **strict undefined** handling;
(2) the sound agreement analysis (per-variant required-root variables via MiniJinja's stable
`undeclared_variables(nested=false)` minus an env-derived globals allowlist) — the headline
differentiator; (3) caller-owned variant resolution + content-addressed provenance
(`template_hash`/`render_hash`, no `vars_hash`); (4) var-provenance plumbing with an opt-in, additive
guard expansion returned as a **separate result field**.

**Technical approach** (from Phase 0): depend on `minijinja 2.21` with `default-features = false` so
`macros`/`multi_template` are off — making `{% include/import/extends/macro/block %}` **parse errors**
(structural FR-002 enforcement, no unstable API). Configure `UndefinedBehavior::Strict`. Compute the
allowlist dynamically from the kernel's own `Environment` globals (drift-proof). Hash with `sha2`
(pure-Rust, keeps the FFI gate green). **Relocate** the generated `PromptDefinition` shape from the
consumer crate into the kernel (FR-027 requires the kernel to consume it; the kernel must not depend on
the consumer — C-01/C-02), with the consumer re-exporting it and the codegen task + freshness gate
repointed.

## Technical Context

**Language/Version**: Rust, pinned `1.95.0` (rust-toolchain.toml + mise.toml lockstep; MSRV of
minijinja 2.21 is 1.63, well under our pin).

**Primary Dependencies**:
- `minijinja = { version = "2.21", default-features = false, features = ["builtins", "deserialization", "serde", "std_collections", "adjacent_loop_items"] }` — render + stable `undeclared_variables`; `macros`/`multi_template` deliberately OFF (the FR-002 exclusion mechanism), `adjacent_loop_items` KEPT so loops have no gaps, `debug` left off (research D1 + critique E1).
- `sha2` (RustCrypto) — `template_hash`/`render_hash` (research D8).
- `serde` + `serde_json` — already present (the generated shape's derives + open-object maps).
- Both new deps are pure-Rust; neither pulls `pyo3`/`napi` (research D7).

**Storage**: N/A — the kernel does no I/O (Principle III / C-03). Inputs are pushed in.

**Testing**: `cargo test` (kernel unit tests + a small fixture-backed regression set under
`crates/prompting-press-core/tests/`), run via moon (`moon run :test`) under the existing CI matrix.

**Target Platform**: Library crate; portable pure-Rust (Linux/macOS/Windows — the spec-001 3-OS build
matrix already covers the kernel).

**Project Type**: Library (one crate within the existing Rust workspace). No frontend/backend split.

**Performance Goals**: None specified or needed — synchronous, in-memory string templating. Render and
analysis are O(template size). No throughput/latency target (SC set is correctness/determinism, not
perf).

**Constraints**: FFI-free (C-02, CI-gated); validation-blind (FR-004); no I/O / LLM / request-body /
token-count / output-parse (C-03); deterministic output for structural cross-language parity (C-01);
agreement analysis + guard expansion must be pure / non-mutating (FR-018, FR-023).

**Scale/Scope**: One crate; ~5 modules (engine, agreement, provenance, hashing, error) + the relocated
generated shape; a handful of kernel-defined types (data-model.md); ~25 validation scenarios
(quickstart.md). Single dependency-direction change (the shape relocation) touches ~8 files
(research D6 blast radius).

**Unknowns**: none open. Roadmap Q3 (MiniJinja version + stable-API soundness) re-confirmed against
2.21.0 source (research D1/D2). One **spec-internal contradiction** surfaced (FR-010 vs FR-011 + the 001
schema) — resolved-by-recommendation, flagged for user confirmation (see Complexity Tracking + research).

## Constitution Check

*GATE: must pass before Phase 0. Re-checked after Phase 1 design.*

| Principle / Decision | Requirement | This plan | Status |
|---|---|---|---|
| **I — Shared core, structural parity** (C-01) | Behavior implemented once, in Rust; deterministic | Kernel is the sole render/analysis/hash site; `BTreeSet` ordering + `sha2` + no time/random ⇒ byte-identical output | ✅ PASS |
| **II — FFI isolation** (C-02) | Kernel free of `pyo3`/`napi` | New deps `minijinja`,`sha2` are pure-Rust; `check-ffi` gate stays green (SC-007); the relocated generated shape is serde-only | ✅ PASS |
| **III — Minimal boundary** (C-03) | No I/O, LLM, request-body, token-count, output-parse | None added; kernel is pure over its inputs (data-model lifecycle) | ✅ PASS |
| **IV — Typed input / sound agreement check** (C-04) | `undeclared_variables(nested=false)` + allowlist; CI lint; pure; includes/macros excluded | Implemented via stable 2.21 API (verified); excluded features are **parse errors** (features off); analysis never mutates (FR-018) | ✅ PASS |
| **IV / C-09 — Var provenance** | Metadata + lint + opt-in additive guard, never silent mutation | `ProvenanceView` exposure + opt-in guard as a **separate field**; no sanitization (FR-025) | ✅ PASS |
| **V — Repo canonical; git owns versioning** (C-05) | No managed version axis; caller-owned variants; `template_hash`/`render_hash` over strings, no `vars_hash` | Variant selection is caller-passed; provenance is data on the result; both hashes over strings; no `vars_hash` (FR-014) | ✅ PASS |
| **VI — Per-language idiom** (C-06) | Errors normalized at consumer boundary; native types don't cross FFI | Kernel returns native `KernelError`; normalization is the consumer's (003) job — kernel correctly does NOT do it | ✅ PASS (boundary respected) |
| **VII — JSON Schema single source** (C-07) | Per-language shapes code-generated, freshness-gated | The kernel **consumes** the generated shape (FR-027); relocation keeps it generated + freshness-gated, never hand-edited | ✅ PASS |
| **Scope Discipline** (C-08) | No new pluggable interface; one concrete path | No seam introduced; `GuardConfig` is a plain option, not a plugin; allowlist is a kernel constant/env-derived, not configurable | ✅ PASS |
| **Boundary defense** | No I/O/LLM/version-axis/etc. without amendment | None proposed | ✅ PASS |

**Result**: PASS (pre-Phase-0 and post-Phase-1). No violations; Complexity Tracking records the one
flagged spec contradiction (not a constitution deviation).

## Project Structure

### Documentation (this feature)

```text
specs/002-engine-kernel/
├── spec.md
├── plan.md              # this file
├── research.md          # Phase 0 — D1..D8 + the FR-010/011 contradiction
├── data-model.md        # Phase 1 — kernel types
├── quickstart.md        # Phase 1 — validation scenarios
├── contracts/
│   └── kernel-api.md     # Phase 1 — public Rust API contract
├── memory.md            # feature memory (clarified + open-for-plan)
├── memory-synthesis.md  # planning synthesis
└── clarifications.md    # clarify-session record
```

### Source Code (repository root)

```text
crates/
├── prompting-press-core/          # THE KERNEL (this spec's work)
│   ├── Cargo.toml                 # + minijinja (default-features=false), + sha2
│   ├── moon.yml                   # + `codegen` task (moved from consumer), build deps on it
│   ├── scripts/
│   │   └── codegen.sh             # MOVED here from consumer; OUT repointed to this crate
│   ├── src/
│   │   ├── lib.rs                 # + pub mod {generated, engine, agreement, provenance, hashing, error}
│   │   ├── generated.rs           # NEW module file (was in consumer)
│   │   ├── generated/
│   │   │   └── prompt_definition.rs   # MOVED here (the FR-027 input contract)
│   │   ├── engine.rs              # Environment build (Strict, features off) + render + RenderResult + GuardConfig
│   │   ├── agreement.rs           # required_roots() over undeclared_variables(false) + env allowlist
│   │   ├── provenance.rs          # ProvenanceView + guard-text builder
│   │   ├── hashing.rs             # sha256 hex helpers
│   │   └── error.rs               # KernelError
│   └── tests/
│       ├── fixtures/              # small engine-regression (template,values)->output set
│       └── *.rs                   # US1/US2/US3/excluded-feature integration tests
│
└── prompting-press/               # CONSUMER (unchanged behavior; re-export edge updated)
    ├── src/lib.rs                 # `pub use prompting_press_core::generated::prompt_definition::*` (was local)
    └── src/generated.rs           # DELETED (module relocated to kernel)

schemas/moon.yml                   # codegen-check: dep prompting-press:codegen -> prompting-press-core:codegen; input path repointed
```

**Structure Decision**: Single library crate (`prompting-press-core`) within the existing workspace.
The one structural change beyond adding kernel modules is the **relocation of the generated shape** into
the kernel (research D6), required by FR-027 + the C-01/C-02 dependency direction. Python/TS codegen are
independent and untouched.

## Complexity Tracking

> Only the items below need justification. No constitution violations.

| Item | Why it exists | Why the simpler path was rejected |
|---|---|---|
| **Relocating the generated shape** (consumer → kernel) + moving the codegen task | FR-027 requires the kernel to consume the shape; kernel must not depend on the consumer (C-01/C-02) | Leaving it in the consumer would force the kernel to depend on the consumer (inverts the direction) or duplicate the shape (drift, violates C-07). Writing the consumer's task output into the kernel dir (scan "Option A") leaves a cross-project moon output write — more fragile than moving the task |
| **`minijinja` with `default-features = false`** (non-obvious feature subsetting) | Disabling `macros`/`multi_template` is what turns excluded features into *parse-time* errors (FR-002) without the forbidden `unstable_machinery` AST API. `adjacent_loop_items` is explicitly re-added (it is default-on; dropping it would silently break `loop.previtem`/`nextitem`/`changed` — critique E1) | Default features + a no-op loader only catches includes at *render* time and does nothing for inline macros — weaker FR-002 enforcement. Blanket `default-features = false` without re-adding `adjacent_loop_items` would silently narrow "loops" |

### Resolved (was: a flagged spec contradiction)

**FR-010 vs FR-011 + the 001 schema.** FR-010 demanded a "multi-variant prompt must declare an explicit
default else loud error," but the 001 schema makes `body` required and `body` **is** the default arm —
so every prompt always has a default and FR-010's error path was structurally unreachable. The plan
adopted the only schema-consistent reading (root `body` = always the default; the sole variant error is
unknown-variant). **Ratified by the user and amended into the spec on 2026-06-26** via
`/speckit.refine.update` (FR-010 rewritten; US1 scenario 4 + SC-004 propagated). Spec and plan now
agree.
