# Implementation Plan: Python binding (`prompting-press-py` → `packages/python`)

**Branch**: `004-python-binding` | **Date**: 2026-06-27 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-python-binding/spec.md`

## Summary

Build out `prompting-press-py` — the **first FFI binding** — filling the existing PyO3 stub crate with a
PyO3 marshaling layer over the spec-003 Rust consumer (`prompting-press`), and round out the
`packages/python` package so `import prompting_press` exposes the four capabilities in **Python idiom**:
a **Pydantic v2** typed-Vars facade (validation owned at render — clarified Q1), a **dual-input loader
reused from the Rust consumer via FFI** (YAML/JSON text marshaled in; constructed Pydantic object →
JSON → the consumer's loader — clarified Q3), a **registry** + **`check()`** agreement/provenance lint,
ergonomic **render/get_source + `from_messages` composition**, and **error normalization** to
`[{field, code, message}]` raised as a **`PromptingPressError` exception hierarchy** (clarified Q2). The
binding adds **no** render/agreement/variant/hash logic — those are FFI calls into the shared Rust core
(Principle I / C-02); render byte-parity is therefore structural, not re-tested in Python.

**Technical approach** (from Phase 0): expose `#[pyclass]` wrappers (`Registry`, `RenderResult`,
`CheckReport`/`Finding`, `Composition`/`Message`) and `#[pyfunction]`s over the shared Rust core using
**PyO3 0.29** (already pinned) + **pythonize 0.29** for the Pydantic-value → `minijinja::Value` /
`serde_json::Value` marshaling bridge. The Pydantic Vars instance is validated in Python
(`model_validate`), dumped (`model_dump(mode="json")`) to a Python object, and `depythonize`d into the
kernel value type. **Render + compose marshal to the *kernel* directly** (`prompting_press_core::render`,
reached via the binding's `prompting-press-core` path dep / `prompting_press::core`), because the
*consumer's* `render<V>` / `Composition::append<V>` are generic over a garde `Validate` Rust type the
Python binding does not have (it carries an already-Pydantic-validated, type-erased value). **The
loader, registry, `check`, and `get_source` are still reused from the consumer** (they need no garde
type). The Python `ValidationError` maps to `PromptValidationError`; the kernel's `KernelError` is routed
through the consumer's existing tested `From<KernelError> for ConsumerError` scrubber (preserving the
SEC-004 fixed-message scrub) and then translated into the `PromptingPressError` subtypes with the shared
closed `code` vocabulary. Calling the kernel directly is still **zero engine logic in the binding** (the
kernel is the shared core — Principle I); the only binding-side orchestration is the compose resolve loop
(~10 lines of glue), not rendering/agreement/variant/hash logic. Packaged as a **maturin** abi3 wheel with a **CPython
3.10** floor (crate bumped `abi3-py39` → `abi3-py310` — clarified Q4); latest stable CPython (3.14,
already the repo `mise` pin) is the build/dev/test target. The Pydantic `PromptDefinition` shape stays
codegen'd from the JSON Schema (Principle VII; already wired). No new Rust deps reach the kernel or Rust
consumer → `ci:check-ffi` stays green.

## Technical Context

**Language/Version**: Rust (workspace lockstep, pinned `1.95.0`) for the binding crate; Python **3.10+**
(abi3 floor) for the package, latest stable **3.14** (repo `mise` pin `3.14.4`) as the dev/test target.

**Primary Dependencies** (all version-verified this cycle against crates.io / PyPI — see research.md):
- `pyo3 = "0.29"` (features `extension-module`, `abi3-py310`) — already pinned at `0.29` (crates.io max
  stable = **0.29.0**); bump the feature `abi3-py39` → `abi3-py310` (clarified Q4).
- `pythonize = "0.29"` — serde bridge between Python objects and `serde`/`minijinja` values (crates.io
  max stable = **0.29.0**, version-matched to PyO3 0.29). The marshaling primitive for FR-003a.
- `prompting-press` (Rust consumer, path dep — already present) and `prompting-press-core` (kernel, path
  dep — already present). The binding marshals render/compose to the **kernel** (`prompting_press_core::
  render`); reuses the **consumer** for loader/registry/`check`/`get_source` and the
  `From<KernelError>`/`ConsumerError` error-normalization + SEC-004 scrub.
- `serde_json` (present transitively) for the constructed-object → JSON → `load_json` path.
- Python build: **maturin** `>=1.14,<2.0` (PyPI latest = **1.14.1**, in range); **Pydantic v2** runtime
  dep for the package (PyPI latest = **2.13.4**); **datamodel-code-generator** `0.65.1` (uv-locked;
  PyPI latest 0.66.0 — pinned for codegen determinism, bump deliberately if at all).

**Storage**: N/A — no I/O (Principle III). The caller hands in already-read YAML/JSON text or a
constructed Pydantic object.

**Testing**: `cargo test -p prompting-press-py` (Rust-side marshaling unit tests) + **pytest** against
the built wheel (Python-side render/check/compose/exception scenarios — quickstart.md), run via moon.

**Target Platform**: Native CPython extension module (abi3 wheel), portable across the existing 3-OS
build matrix; one wheel per OS/arch across CPython 3.10 → latest.

**Project Type**: Library binding (PyO3 crate `prompting-press-py` + the `packages/python` distribution
within the existing workspace).

**Performance Goals**: None specified/needed — synchronous in-process marshal + FFI call. SCs are
correctness/parity/packaging, not perf.

**Constraints**: `pyo3` ONLY here (C-02, CI-gated); no engine logic in the binding (C-01 — marshal to
the core); no I/O / no LLM / no request-body / no output parsing / **no token counting** (C-03 / F4);
native error types (Pydantic `ValidationError`, Rust errors) never cross FFI onto the public API (C-06);
`check()` pure; generated Pydantic shape codegen'd, never hand-edited (C-07).

**Scale/Scope**: One binding crate (build out the stub) + the Python package; ~5 marshaling areas
(registry, render/get_source, check, compose, error/exception) mirroring the consumer's 5 modules; the
generated shape already present; ~1 new Rust dep (`pythonize`, pure-Rust); wheel build wiring. No kernel
or consumer changes; no relocation.

**Unknowns**: none open. PyO3 0.29 / pythonize 0.29 / maturin 1.14.1 / Pydantic 2.13.4 / dmcg 0.66.0 and
latest CPython 3.14.6 all re-verified against crates.io / PyPI / python.org directly this cycle (a prior
project pattern of fabricated subagent versions was avoided by querying registries). Remaining plan-time
confirmations are pythonize API-shape details (Phase 0 research.md D-items).

## Constitution Check

*GATE: must pass before Phase 0. Re-checked after Phase 1 design.*

| Principle / Decision | Requirement | This plan | Status |
|---|---|---|---|
| **I — Shared core, no duplication** (C-01) | Render/agreement/variant/hash live once in the kernel | Render/compose MARSHAL to the kernel's `render` directly (the consumer's generic-`V` render needs a garde type the binding lacks); loader/registry/`check`/`get_source`/error-scrub reused from the consumer; zero render/agreement/variant/hash logic in the binding; render parity structural, not re-tested | ✅ PASS |
| **II — FFI isolation** (C-02) | `pyo3` ONLY in `prompting-press-py`; kernel + Rust consumer FFI-free | `pyo3`/`pythonize` live ONLY in the binding crate; the path deps don't pull FFI into `-core`/`prompting-press`; `ci:check-ffi` stays green (SC-007) | ✅ PASS |
| **III — Minimal boundary** (C-03) | No I/O, LLM, request-body, token-count, output-parse | NO token counter (F4); no I/O (caller pushes text/objects); `output_model` metadata only, never parsed | ✅ PASS |
| **VI — Per-language idiom** (C-06) | Native validation system; errors normalized; native types don't leak | Pydantic v2 is the native system; Pydantic `ValidationError` + Rust `ConsumerError` → `PromptingPressError` `[{field,code,message}]`; `from_messages` array, NOT `.chain()` | ✅ PASS |
| **VII — JSON Schema single source** (C-07) | Codegen'd shape; dual-input into one shape | Pydantic `PromptDefinition` codegen'd from the JSON Schema (freshness-gated); dual-input reused from the consumer's one loader; no parallel hand-shape | ✅ PASS |
| **IV — agreement check / provenance** (C-04/C-09) | The sound check + provenance lint, pure | `check()` surfaced to Python over the consumer's lint; pure, no mutation/render (FR-019) | ✅ PASS |
| **Scope Discipline** (R1) | No new pluggable interface | NO new seam — registry/composition/exceptions are plain types; token hook (the only candidate) dropped (F4) | ✅ PASS |
| **Boundary defense** | No I/O/LLM/version-axis/etc. | none proposed | ✅ PASS |

**Result**: PASS (pre-Phase-0 and post-Phase-1). No violations; no Complexity Tracking entries needed.

## Project Structure

### Documentation (this feature)

```text
specs/004-python-binding/
├── plan.md              # this file
├── research.md          # Phase 0 — D1..D7 (PyO3 0.29, pythonize bridge, maturin abi3, exception map, loader-via-FFI)
├── data-model.md        # Phase 1 — binding types (pyclasses + the Python-facing surface)
├── quickstart.md        # Phase 1 — validation scenarios (pytest against the built wheel)
├── contracts/
│   └── python-api.md     # Phase 1 — public Python API contract
├── memory.md · memory-synthesis.md · checklists/requirements.md
```

### Source Code (repository root)

```text
crates/prompting-press-py/        # THE BINDING (this spec's work — builds out the existing stub)
├── Cargo.toml                    # bump pyo3 feature abi3-py39 -> abi3-py310; + pythonize 0.29
├── build.rs                      # unchanged (macOS -undefined dynamic_lookup already handled)
└── src/
    ├── lib.rs                    # #[pymodule] wiring: register classes + functions
    ├── registry.rs               # #[pyclass] Registry over consumer Registry: load_yaml/load_json/insert
    ├── render.rs                 # render()/get_source() #[pyfn]: model_validate -> depythonize -> consumer::render
    ├── check.rs                  # check() #[pyfn] -> CheckReport/Finding pyclasses (deterministic order preserved)
    ├── compose.rs                # Composition #[pyclass] (from_messages/append/resolve) -> [Message]; no .chain()
    ├── error.rs                  # ConsumerError + Pydantic ValidationError -> PromptingPressError hierarchy (+ SEC-004 scrub)
    └── marshal.rs                # pythonize bridge: Pydantic value <-> kernel value type; lossless None/int/float/nested

packages/python/                  # THE DISTRIBUTION (build out the scaffold)
├── pyproject.toml                # + pydantic runtime dep; requires-python >=3.10 (already); maturin backend (already)
├── python/prompting_press/
│   ├── __init__.py               # re-export the compiled symbols + the Python-facing facade/types + exceptions
│   └── generated/prompt_definition.py   # GENERATED — do not hand-edit (codegen freshness-gated)
└── tests/                        # pytest: US1-US4 + boundary scenarios (quickstart) against the built wheel
```

**Structure Decision**: Build out the existing `prompting-press-py` stub crate (already a `cdylib` with
path deps on the consumer + kernel, and the macOS link arg handled) plus the `packages/python` scaffold
(maturin backend + generated Pydantic shape already wired). No new crate, no relocation. Cargo.toml
changes: bump the abi3 feature and add `pythonize`. The Python package gains its facade `__init__`,
the exception hierarchy, and the pytest suite; the generated shape is untouched (regenerated only via
`codegen.sh`).

## Complexity Tracking

> No constitution violations; no entries required.

One item worth noting (not a violation): the binding crate is the **single** place `pyo3`/`pythonize`
appear — this is the intended C-02 idiom (the binding layer IS the FFI boundary), not an invented
abstraction. The `marshal.rs` module concentrates all Python↔kernel value translation so the FFI
boundary is auditable in one file.

### Verified-this-cycle (so a future reader doesn't re-litigate)

- **PyO3** crates.io max stable = **0.29.0** (crate pin `0.29` correct). **pythonize** max stable =
  **0.29.0** (version-matched to PyO3 0.29) — the serde marshaling bridge.
- **maturin** PyPI latest = **1.14.1** (pyproject `>=1.14,<2.0` in range). **Pydantic** PyPI latest =
  **2.13.4** (v2 target correct). **datamodel-code-generator** PyPI latest = **0.66.0** (uv-locked at
  `0.65.1` for codegen determinism — bump deliberately, not as drift).
- Latest stable **CPython = 3.14.6** (python.org); repo `mise.toml` already pins `python = 3.14.4` →
  the Q4 build/dev/test target is wired. **Watch-item**: CPython **3.10 reaches EOL 2026-10-31** (~4
  months out) — the abi3-py310 floor still runs post-EOL; 3.10 just stops receiving upstream patches.
  Floor decision stands (broad reach); revisit at release (spec 007) if desired.
- All version checks were made by querying crates.io / PyPI / python.org **directly** (not via a
  research subagent), per the project's systemic-fabrication guard.
