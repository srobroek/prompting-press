# Implementation Plan: Rust consumer crate (`prompting-press`)

**Branch**: `003-rust-consumer` | **Date**: 2026-06-26 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-rust-consumer/spec.md`

## Summary

Build out `prompting-press` — the public, idiomatic Rust API over the spec-002 kernel — filling the
existing stub crate with the language-native layer the kernel deliberately omits: a **garde** typed-Vars
facade (validate once at render), a **dual-input loader** (YAML/JSON/object → the kernel's
`PromptDefinition`), a **registry** (name → definition), the **`check()` agreement + provenance lint**
(the headline guarantee, made usable in CI), ergonomic **render/get_source + composition**, **error
normalization** to `[{field,code,message}]`. (The token-count hook was dropped — F4; deferred.) It wraps the kernel and
duplicates none of its render/agreement/variant/hash logic (Principle I).

**Technical approach** (from Phase 0): `garde 0.23` (`derive`+`serde` features, custom validators via
`#[garde(custom)]`, normalize `Report::iter()` → `FieldError`); `serde_yaml_ng 0.10` for the YAML arm
(pure-Rust, YAML-1.2/Norway-safe via yaml-rust2); bridge the validated struct to the kernel with
`minijinja::Value::from_serialize`; `check()` = `BTreeSet` difference of the kernel's `required_roots`
vs `def.variables` (the authoritative declared set) + a provenance check via `provenance_view`. All
deps pure-Rust → the FFI gate stays green.

## Technical Context

**Language/Version**: Rust, pinned `1.95.0` (workspace lockstep). garde MSRV 1.78, well under.

**Primary Dependencies**:
- `garde = { version = "0.23", features = ["derive", "serde"] }` — typed-Vars validation + custom
  validators (research D1). Pure-Rust (the only native dep, `js-sys`, is optional/non-default).
- `serde_yaml_ng = "0.10"` — YAML arm of the dual-input loader (research D2). Pure-Rust, drop-in
  serde_yaml API, YAML-1.2 (Norway-safe) via yaml-rust2. *Pin the exact current patch at impl time.*
- `prompting-press-core` (the kernel, path dep — already present), `serde` + `serde_json` (present;
  JSON arm + the generated shape). `minijinja` referenced at the render boundary for `Value` (via the
  kernel's re-export, or added explicitly).

**Storage**: N/A — no I/O (Principle III). The caller hands in already-read YAML/JSON text or a
constructed object.

**Testing**: `cargo test` (unit + integration), run via moon under the existing CI matrix.

**Target Platform**: Library crate, portable pure-Rust (existing 3-OS build matrix covers it).

**Project Type**: Library (the public Rust consumer crate within the existing workspace).

**Performance Goals**: None specified/needed — synchronous in-process validate+wrap. SCs are
correctness/parity, not perf.

**Constraints**: FFI-free (C-02, CI-gated); no logic duplication (C-01 — wrap the kernel); no I/O / no
LLM / no request-body / no output parsing (C-03); native error/validator types never leak (C-06);
`check()` + validators pure (no mutation).

**Scale/Scope**: One crate (the existing stub), 5 modules (registry, render, check, compose, error);
a handful of consumer types (data-model.md); ~16 validation scenarios (quickstart.md). 2 new
pure-Rust deps. No relocation/restructure — the kernel/consumer split is already in place.

**Unknowns**: none open. garde 0.23 + serde_yaml_ng 0.10 + `Value::from_serialize` re-verified against
crates.io + tagged source (research D1–D4; a research subagent's fabricated version numbers were
caught and discarded). Plan-time confirmations remaining are version-pin patch levels only.

## Constitution Check

*GATE: must pass before Phase 0. Re-checked after Phase 1 design.*

| Principle / Decision | Requirement | This plan | Status |
|---|---|---|---|
| **I — Shared core, no duplication** (C-01) | Render/agreement/variant/hash live once in the kernel | The consumer WRAPS `render`/`get_source`/`required_roots`/`provenance_view`; `check()` is set-ops over kernel output; no such logic reimplemented | ✅ PASS |
| **II — FFI isolation** (C-02) | Consumer free of `pyo3`/`napi` | garde + serde_yaml_ng are pure-Rust (research D1/D2); `check-ffi` gate stays green (SC-007) | ✅ PASS |
| **III — Minimal boundary** (C-03) | No I/O, LLM, request-body, token-count, output-parse | NO token counting at all (the hook was dropped — F4; deferred); no I/O (caller pushes text/objects); output_model carried as metadata, never parsed | ✅ PASS |
| **VI — Per-language idiom** (C-06) | Native validation system; errors normalized; native types don't leak | garde is the native system; `Report`/`KernelError` → `ConsumerError` `[{field,code,message}]`; composition is `Vec`+`append_*`, NOT `.chain()` | ✅ PASS |
| **VII — JSON Schema single source** (C-07) | Dual-input loader into one shape | YAML/JSON/object → the kernel's `PromptDefinition`; no parallel shape (FR-008) | ✅ PASS |
| **IV — agreement check / provenance** (C-04/C-09) | The sound check + provenance lint, pure | `check()` surfaces both as a CI lint; pure, no mutation/render (FR-019) | ✅ PASS |
| **Scope Discipline** (C-08) | No new pluggable interface | NO new seam — the token-count hook (the only candidate) was dropped (F4); registry/composition are plain types | ✅ PASS |
| **Boundary defense** | No I/O/LLM/version-axis/etc. | none proposed | ✅ PASS |

**Result**: PASS (pre-Phase-0 and post-Phase-1). No violations; no Complexity Tracking entries needed.

## Project Structure

### Documentation (this feature)

```text
specs/003-rust-consumer/
├── plan.md              # this file
├── research.md          # Phase 0 — D1..D7 (garde 0.23, serde_yaml_ng, bridge, lint)
├── data-model.md        # Phase 1 — consumer types
├── quickstart.md        # Phase 1 — validation scenarios
├── contracts/
│   └── consumer-api.md   # Phase 1 — public Rust API contract
├── memory.md · memory-synthesis.md · clarifications.md
```

### Source Code (repository root)

```text
crates/prompting-press/          # THE CONSUMER (this spec's work — builds out the existing stub)
├── Cargo.toml                   # + garde (derive,serde), + serde_yaml_ng; fix stale generated-shape comment
├── src/
│   ├── lib.rs                   # module wiring + re-exports (PromptDefinition, RenderResult already re-exported)
│   ├── registry.rs              # Registry { BTreeMap<String, PromptDefinition> } + load_yaml/load_json/insert
│   ├── render.rs                # render<V: Serialize+Validate>() + get_source() — validate → from_serialize → kernel
│   ├── check.rs                 # check(registry) -> CheckReport: required_roots ∖ variables + provenance lint
│   ├── compose.rs               # Composition (Vec + append_*) -> Vec<Message>; no .chain()
│   └── error.rs                 # ConsumerError + FieldError; garde Report / KernelError normalization (+ SEC-004 scrub)
│                                # (token-count hook dropped — F4; deferred to a later spec)
└── tests/                       # US1–US4 + boundary integration tests (quickstart scenarios)
```

**Structure Decision**: Single library crate, already present as a stub from spec 001 (it already
depends on the kernel and re-exports `PromptDefinition`). 003 fills it in — no new crate, no
relocation. The only Cargo.toml change beyond adding the two deps is fixing a stale comment that still
claims the generated shape lives in the consumer (it moved to the kernel in spec 002).

## Complexity Tracking

> No constitution violations; no entries required.

The one item worth noting (not a violation): the `render` generic bound `V: Serialize + Validate` ties
the public render API to garde's `Validate` trait. This is the intended C-06 idiom (garde IS the Rust
validation system), not an invented abstraction — so it is in-bounds, not a complexity deviation.

### Verified-this-cycle (so a future reader doesn't re-litigate)

- garde latest = **0.23.0** (roadmap pin correct); `Report` has `iter()`/`into_inner()`, **no
  `flatten()`** — normalize via `iter()` over `(Path, Error)`. A research subagent fabricated
  `0.22.0` + a `flatten()` method (`tool_uses: 0`); discarded and re-verified against the `v0.23.0`
  source tag.
- `serde_yaml` is archived (`0.9.34+deprecated`); `serde_yaml_ng 0.10` is the maintained, pure-Rust,
  Norway-safe successor.
- `minijinja::Value::from_serialize` exists in 2.21.0 (`value/mod.rs:856`) — the validated-struct → kernel-value bridge.
