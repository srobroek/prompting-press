# Phase 0 Research — Python binding (`prompting-press-py`)

All version/API facts below were verified **directly** against crates.io / PyPI / python.org / the PyO3
docs (context7) this cycle — **not** via a research subagent (per the project's systemic
subagent-fabrication guard; see speckit-workflow-gotchas). Each decision is recorded as
Decision / Rationale / Alternatives.

---

## D1 — PyO3 version + extension-module + abi3 floor

- **Decision**: `pyo3 = "0.29"` with features `["extension-module", "abi3-py310"]`. **Bump** the
  scaffold's `abi3-py39` → `abi3-py310`.
- **Rationale**: crates.io max stable = **0.29.0** (the crate's existing `0.29` pin is current). The
  `extension-module` feature avoids linking libpython so a bare `cargo build`/`check` works without a
  Python dev env (already exploited by the scaffold's `build.rs` macOS link arg). The abi3-py310 floor
  reconciles the committed three-way disagreement (crate `abi3-py39` vs `requires-python >=3.10` vs
  codegen `--target-python-version 3.10`) — clarified Q4. abi3 = one wheel across CPython ≥ floor.
- **Alternatives**: `abi3-py39` (rejected — the generated `X | None` syntax does not import on 3.9, so
  it was a latent install-then-ImportError trap); non-abi3 version-specific wheels (rejected — a wheel
  matrix per CPython minor, far more build/publish surface for spec 007).
- **Watch-item**: CPython **3.10 EOL = 2026-10-31** (~4 months out). The floor still *runs* post-EOL;
  3.10 just stops getting upstream patches. Decision stands; revisit at release (007) if desired.

## D2 — The marshaling bridge: Python value ↔ kernel value type

- **Decision**: use **`pythonize = "0.29"`** (`depythonize` to read a Python object into a serde value;
  `pythonize` for the reverse where needed). Flow for FR-003a: Python caller's Pydantic instance →
  `model_validate` (validate) → `model_dump(mode="json")` → a Python object → `depythonize` into the
  kernel's `minijinja::Value`. **The marshaled value is handed to `prompting_press_core::render`
  DIRECTLY** (critique E1): the *consumer's* `render<V: Serialize + Validate>` is generic over a garde
  Rust type the Python binding does not have, so the binding bypasses it for render/compose and calls
  the kernel (still zero engine logic — Principle I). The consumer is still reused for
  loader/registry/`check`/`get_source`/error-scrub.
- **Rationale**: crates.io max stable = **0.29.0**, version-locked to PyO3 0.29 (pythonize tracks PyO3
  minors). It is the standard serde↔Python bridge, pure-Rust, and concentrates all value translation in
  one auditable `marshal.rs` (C-02 — the FFI boundary is one file). `minijinja::Value::from_serialize`
  is the same primitive the spec-003 consumer uses, so the kernel sees an identical value shape →
  render parity stays structural.
- **Lossless edge handling** (FR-003a, spec-006 preview): `None`→null, `bool`, `int` vs `float`
  preserved by serde's data model; nested dict/list → nested value; `datetime`/`Decimal` arrive as the
  Python objects Pydantic produced — confirm at impl whether to dump with `mode="json"` (stringifies
  date/Decimal deterministically, matching the kernel's text rendering) vs `mode="python"`. **Lean
  `mode="json"`** so the marshaled value is JSON-primitive and parity with the other bindings is
  trivial; pin the choice with a marshaling unit test.
- **Alternatives**: hand-rolled `PyDict`→`Value` walk (rejected — re-implements pythonize, error-prone
  on nesting/None); requiring the caller to pass a dict (rejected — defeats the Pydantic facade and
  FR-003a "caller MUST NOT hand-build a value map").

## D3 — Dual-input loader: reuse the Rust consumer's loader via FFI (clarified Q3)

- **Decision**: the Python `Registry` is a `#[pyclass]` wrapping the consumer's `Registry`.
  `load_yaml(text)` / `load_json(text)` marshal the **text** across FFI to the consumer's
  `Registry::load_yaml` / `load_json`. `insert(obj)` takes a generated-Pydantic `PromptDefinition`,
  `model_dump_json()`s it, and routes through the consumer's `load_json` (one loader, one
  representation).
- **Rationale**: makes YAML↔JSON parity (FR-006/SC-003) and malformed-input accept/reject (FR-007) a
  **structural** property of the shared core — the same Principle I argument that makes render parity
  free — with no Python YAML dependency and no second loader to keep in agreement. Aligns with C-02
  ("marshaling + facade only"): a Python-side parser would be parsing logic in the binding.
- **Alternatives**: parse Python-side against the Pydantic model (rejected — a second loader
  implementation, PyYAML dependency, accept/reject drift that spec 006 would then have to police).

## D4 — Error → Python exception hierarchy (clarified Q2)

- **Decision**: a base **`PromptingPressError(Exception)`** carrying the `[{field, code, message}]` rows,
  with subtypes mapping 1:1 onto the Rust `ConsumerError` variants:
  `PromptValidationError` (Pydantic + garde-class `validation`), `PromptRenderError`
  (kernel `unknown_variant`/`undefined_variable`/`parse`/`render`/`excluded_feature`),
  `UnknownPromptError` (`unknown_prompt`), `LoadError` (`load`). Each carries `.errors`
  (list of `{field, code, message}`). The `code` strings are the **same closed vocabulary** the Rust
  consumer exposes (`error::code`).
- **Rationale**: PyO3 0.29 supports this directly — the **field-carrying base** via
  `#[pyclass(extends=PyException)]` with `#[pyo3(get)]` accessors, and/or `create_exception!` for plain
  subtypes (verified against the PyO3 0.29 guide). Gives Python callers idiomatic `except`-by-class while
  preserving the single structured contract; the 1:1 map keeps translation exhaustive over the closed
  `ConsumerError` enum (a new Rust variant becomes a compile/translation error, not a silent fallthrough).
- **SEC-004 scrub** (refined per critique E2 + security SEC-004-PY): render/compose call the kernel
  DIRECTLY (E1), so the binding receives a RAW `KernelError`, not a pre-scrubbed `ConsumerError`. It MUST
  route that `KernelError` through the consumer's existing tested `From<KernelError> for ConsumerError`
  scrubber FIRST (the consumer replaces `parse`/`render`/`excluded_feature` detail with a fixed message —
  `crates/prompting-press/src/error.rs:191`, with passing secret tests), then surface those
  already-scrubbed rows. Never copy raw `KernelError` detail into the exception. Tests assert the seeded
  secret is absent from the **Python** `str(exc)`, `repr(exc)`, AND `exc.errors` — not just a Rust string.
  The `#[pyclass(extends=PyException)]` base must not hand-write a `__str__`/`__repr__` that leaks rows.
- **Pydantic `ValidationError` translation**: caught at `render`/`compose`, its `.errors()` rows mapped to
  `{field: loc-joined, code: "validation", message: msg}` — using Pydantic's `msg` ONLY (NOT `input`/`ctx`,
  which can carry the rejected value — SEC-004-PY) — matching the consumer's garde→`FieldError` shape so
  validation errors look identical across bindings.
- **Alternatives**: single exception type (rejected at clarify Q2 — no `except`-by-class granularity);
  letting native errors propagate (rejected — violates C-06 / SC-006).

## D5 — Packaging: maturin abi3 wheel

- **Decision**: keep the existing maturin backend (`pyproject.toml` already wires
  `manifest-path = ../../crates/prompting-press-py/Cargo.toml`, `module-name = prompting_press`,
  `python-source = python`, `features = ["pyo3/extension-module"]`). Build via `maturin build`/`develop`;
  add `pydantic` as a runtime dependency of the package. maturin `>=1.14,<2.0` (PyPI latest 1.14.1).
- **Rationale**: maturin is the PyO3 packaging standard (already chosen in spec 001 research D1). abi3 +
  the 3.10 floor → one wheel per OS/arch across CPython 3.10→latest. No publish here (spec 007); SC-009
  only requires a locally buildable/installable/importable wheel.
- **Alternatives**: setuptools-rust (rejected — maturin is the idiomatic PyO3 path and is already wired).

## D6 — Codegen freshness (Pydantic `PromptDefinition`)

- **Decision**: no change to the codegen pipeline — `packages/python/scripts/codegen.sh` +
  `datamodel-code-generator==0.65.1` (uv-locked) already generate the Pydantic shape from the JSON
  Schema, and `schemas:codegen-check` already gates freshness. The generated model is present and
  current. The caller-authored Pydantic Vars models are separate (application-owned, not codegen'd).
- **Rationale**: Principle VII / C-07 — the shape is the single source, codegen'd, never hand-edited.
  dmcg PyPI latest is 0.66.0; the `0.65.1` pin is intentional (codegen determinism) — bump deliberately,
  not as drift.
- **Alternatives**: hand-writing the Pydantic shape (rejected — violates C-07, drift risk).

## D7 — Composition (`from_messages`) + check surface

- **Decision**: `Composition` is a `#[pyclass]` holding a **binding-owned** ordered list of marshaled
  `(name, value, variant)` entries — NOT the consumer's `Composition<V>`, which is generic over a garde
  type the binding lacks (critique E1). Expose a `from_messages([(prompt, vars, variant), ...])`
  constructor (idiomatic Python array) + `append` + `resolve(registry) -> [Message]`. Validate
  **eagerly at append/from_messages** (option (a), as spec 003 — `model_validate` + marshal now), so an
  invalid entry fails immediately and `resolve` never emits a partial-as-success. The `resolve` loop is
  the only binding-side orchestration (~10 lines: iterate, `Registry::get`, call `prompting_press_core::
  render` direct, tag the role) — not shared-core logic.
  `check(registry)` returns a `CheckReport` pyclass with a `.findings` list of `Finding`
  (prompt/variant/kind/detail), preserving the consumer's deterministic BTreeMap/BTreeSet order and the
  `ReservedVariantName`/`AnalysisError` kinds.
- **Rationale**: mirrors the merged spec-003 semantics exactly (FR-012/013, US3/US4); `from_messages` is
  the Pythonic spelling of the explicit ordered array; never `.chain()` (FR-013).
- **Alternatives**: a fluent builder (rejected — FR-013); lazy validation at resolve (acceptable but
  eager-at-append matches 003 and gives earlier, clearer errors).

---

## Resolved unknowns

All Technical-Context unknowns are resolved: PyO3 0.29 / pythonize 0.29 / maturin 1.14.1 / Pydantic
2.13.4 / dmcg 0.66.0 (pin 0.65.1) / CPython 3.14.6 latest confirmed against registries; the marshaling
bridge (pythonize), loader locus (FFI-reuse), exception hierarchy (PyO3 `create_exception!` +
`extends=PyException`), and packaging (maturin abi3-py310) are all decided. No open NEEDS CLARIFICATION.
Remaining impl-time confirmations: the exact `model_dump` mode (json vs python) for date/Decimal,
finalized by a marshaling unit test (D2).
