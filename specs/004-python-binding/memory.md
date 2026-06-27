# Feature Memory — Spec 004 (Python binding `prompting-press-py`)

Feature-local working notes + open questions for the first FFI binding. Durable decisions live in the
governance layer (constitution Principles I–VII + roadmap C-01…C-10); this file is transient.

## What 004 owns (the Python-native layer over the shared core)

`prompting-press-py` is the PyO3 extension module that exposes the spec-003 consumer surface to
Python. It adds exactly the language-native layer: a **Pydantic v2** typed-Vars facade, Python
**exceptions** for the normalized error contract, and **maturin/PyO3** packaging. It MARSHALS to the
Rust core (kernel via the `prompting-press` consumer) — it reimplements **no** render/agreement/
variant/hash logic (Principle II / C-02). `pyo3` lives ONLY here; `ci:check-ffi` enforces the kernel +
Rust consumer stay FFI-free.

## What it binds (as-built, verified against the tree)

- **Rust consumer `prompting-press`** (spec 003) — the surface 004 reproduces: `Registry`
  (`load_yaml`/`load_json`/`insert`), `render`/`get_source` (validate-then-render), `check` →
  `CheckReport`/`Finding`/`FindingKind`, `Composition`/`Message` (`append`/`resolve`, no `.chain()`),
  `ConsumerError`/`FieldError` + the closed `code` vocabulary (`validation`, `unknown_prompt`,
  `unknown_variant`, `undefined_variable`, `parse`, `render`, `excluded_feature`, `load`).
- **Kernel `prompting-press-core`** (spec 002) — `RenderResult { text, name, variant, template_hash,
  render_hash, guard: Option<String> }`, closed `KernelError` (5 variants), `GuardConfig` (Default),
  generated `PromptDefinition` (re-exported by the consumer).
- **Crate scaffold** `crates/prompting-press-py/`: PyO3 `0.29`, `cdylib`, `extension-module` +
  `abi3-py39` (→ bump to `abi3-py310`, see clarified Q4), path deps on BOTH `prompting-press` +
  `prompting-press-core`, macOS `-undefined dynamic_lookup` link arg already in `build.rs`. Only a
  `core_version()` stub exists.
- **Package scaffold** `packages/python/`: maturin `>=1.14,<2.0`, `module-name = prompting_press`,
  `python-source = python`, `requires-python = >=3.10`, Pydantic v2 target. Generated Pydantic
  `PromptDefinition` already present; `datamodel-code-generator==0.65.1` uv-locked; `codegen.sh` +
  the `schemas:codegen-check` freshness gate already wired.

## Clarified (Session 2026-06-27)

- **Q1 Validation ownership**: the binding **owns validation at render** — accepts the Pydantic model +
  data (or an instance it re-validates), `model_validate` before templating, normalizes
  `ValidationError` (no native error escapes). Mirrors spec-003 validate-then-render.
- **Q2 Exception shape**: a **hierarchy under one base `PromptingPressError`** (validation / kernel-render
  / unknown-prompt / load), each carrying `[{field, code, message}]` rows + stable `code`; 1:1 with the
  Rust `ConsumerError` variants. (Subtype names finalized at plan.)
- **Q3 Loader locus**: **reuse the Rust consumer's dual-input loader via FFI** — marshal YAML/JSON
  **text** in, parse with the consumer's serde path. Parity + accept/reject = structural. Constructed
  object (Pydantic) → JSON → consumer `load_json`. No Python YAML dependency.
- **Q4 Python floor**: broad **abi3 floor = CPython 3.10** (crate `abi3-py39` → `abi3-py310`); latest
  stable CPython as the build/dev/test target. Reconciles the scaffold's three-way disagreement.

## Open questions (resolve/verify in plan)

1. **Version verification (verify-at-spec-time)**: confirm current **PyO3**, **maturin**, **Pydantic
   v2**, **datamodel-code-generator** versions/APIs against crates.io / PyPI DIRECTLY (subagent-reported
   versions have been fabricated before). Crate currently pins PyO3 `0.29`, maturin `>=1.14,<2.0`,
   dmcg `0.65.1` — confirm these are current/compatible or bump deliberately.
2. **Marshaling bridge (Python value → kernel `minijinja::Value`)**: how a validated Pydantic instance
   crosses FFI. Candidate: Pydantic `model_dump()` → a Python dict/JSON → PyO3 `depythonize`/serde →
   the kernel value type. Confirm the exact path and that it is lossless for None / int-vs-float /
   nested / date / Decimal (the cases spec 006 will later lock down — here only for own paths).
3. **Registry as a `#[pyclass]`**: the Python `Registry` wraps the consumer's `Registry`. Confirm
   `load_yaml`/`load_json`/`insert(pydantic_obj)` + the name→def map cross cleanly; the consumer
   `Registry` is `BTreeMap`-backed (deterministic check order — preserve).
4. **`check()` → Python**: map `CheckReport`/`Finding`/`FindingKind` (incl. `ReservedVariantName`,
   `AnalysisError`) to a Python report object/list; keep deterministic order. Pure analysis.
5. **`from_messages` composition**: array of `(prompt, vars, variant)` → `[{role, text}]`. Eager-validate
   at append (option (a), like 003) or validate at resolve? No `.chain()`. No partial-as-success.
6. **Exception ↔ `ConsumerError` mapping**: how the binding catches the consumer's `ConsumerError`
   (Rust) and the Pydantic `ValidationError` (Python) and raises the right `PromptingPressError` subtype
   with the rows; preserve the SEC-004 scrub (parse/render/excluded_feature detail → fixed message).
7. **Caller Vars model passing**: `render(reg, name, Model, data)` vs `render(reg, name, instance)` —
   pick the ergonomic signature(s) that keep validation owned by the binding (Q1).

## Non-negotiables to keep green

- **`pyo3` ONLY in `prompting-press-py`** — `ci:check-ffi` checks the kernel + Rust consumer stay
  FFI-free. New binding deps (pythonize/serde bridges) must not leak FFI into `-core`/`prompting-press`.
- **NO engine logic in the binding** — marshaling + Pydantic facade only (C-02). Render/agreement/
  variant/hash are FFI calls into the Rust core (Principle I; parity structural, not re-tested).
- **Generated Pydantic shape is codegen'd from the JSON Schema** (Principle VII / C-07) — never
  hand-edit `packages/python/python/prompting_press/generated/`; the `schemas:codegen-check` gate
  enforces freshness.
- **No I/O, no model calls, NO token counter** (C-03 / F4). The token-hook line in the roadmap 004
  entry is STALE — drop/reconcile it (don't carry).
- Native error types (Pydantic `ValidationError`, Rust errors) **never cross FFI** onto the public API;
  every error → a `PromptingPressError` subtype with `[{field, code, message}]`. Keep the SEC-004 scrub.
- `rm` blocked (use `git mv`/`git rm`); single-quote `git commit -m` with backticks; `dgit push`;
  `Closes #N` one per line; agent-type names per the gotchas memory; cite "roadmap decision C-NN", never
  "constitution C-NN".
