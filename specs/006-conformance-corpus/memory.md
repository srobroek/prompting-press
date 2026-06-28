# Feature Memory — 006 Conformance corpus + cross-language hardening

Feature-local notes prepared before writing the spec. Durable project memory lives in
`.specify/memory/` (constitution, DECISIONS, roadmap) and `docs/memory/`; this file is transient
feature context only.

## What this feature is (roadmap §278)

Implements constitution **Principle VII** as a CI gate across all three packages (Rust, Python, TS).
The corpus guards exactly **two** properties and nothing else:

1. **FFI-boundary marshaling parity** — the same logical input pushed through each binding yields
   identical rendered text AND identical `template_hash`/`render_hash`. Hard cases: datetime/Date/chrono,
   Decimal, nested models, null/undefined/None, int-vs-float.
2. **Schema round-trip parity** — a schema-valid prompt doc is accepted identically and a schema-invalid
   one is rejected identically across the three languages.

## Constraints pulled from durable memory (load-bearing for this spec)

- **C-01 / Principle I — render parity is STRUCTURAL, not tested.** One shared Rust core renders;
  cross-language byte-identity is guaranteed by construction. The corpus MUST NOT re-test render parity.
  Only the small engine-regression render-fixture set from spec 002 remains
  (`crates/prompting-press-core/tests/fixtures/render/`, 2 files) — unchanged, owned by the kernel.
- **C-02 / Principle II — zero engine logic in bindings.** Bindings only marshal + run native validation
  (Pydantic/Zod/garde) + normalize errors. Corpus tests *marshaling*, never re-tests engine behavior.
- **C-07 / Principle VII — JSON Schema is the single source of truth.** Schema round-trip is the corpus's
  second job: same accept/reject across languages. Existing schema fixtures:
  `schemas/jsonschema/fixtures/{valid,invalid}/` (3 valid + 7 invalid) validated today only by the
  Python `validate_fixtures.py` script (`schemas:validate-fixtures`). 006 must make accept/reject run
  through each *binding's loader*, not just a standalone schema validator.
- **The `null`/`undefined`/`None` contract is already FIXED and consistent** (spec 004 + 005, FR-003a):
  `undefined`/absent (JS) and absent (Py) → "field not present" (kernel strict-undefined fires if
  referenced); explicit `null`/`None` → JSON `null`. The corpus pins this is identical across bindings.
- **Hash provenance**: `template_hash = SHA256(variant template source)`, `render_hash = SHA256(rendered
  output)`, per resolved variant, each over a string. No `vars_hash` (Principle V). Surfaced as
  `templateHash`/`renderHash` (TS), `template_hash`/`render_hash` (Py/Rust). Parity already proven
  empirically TS==Python; 006 makes it a permanent CI gate.

## As-built facts the corpus drives through (verified)

- **Rust**: `prompting-press` consumer — `render`, `get_source`, loaders, `check`. Kernel
  `prompting_press_core::render(def, variant, values: minijinja::Value, &GuardConfig)`.
- **Python**: `prompting-press-py` (PyO3 0.29) → `packages/python`. Marshal bridge
  `pythonize::depythonize` → `minijinja::Value::from_serialize`. Tests: `cargo test -p prompting-press-py`
  (30) + `pytest packages/python/tests` (56). CI: `ci:test-python`.
- **TS**: `prompting-press-node` (napi-rs 3.9.4 + Zod 4.4.3) → `packages/typescript`. napi features
  include `napi6` (lossless bigint). Tests: `cargo test -p prompting-press-node` (36) + `node --test`
  (59). CI: `ci:test-node`.
- **CI shape**: `ci.yml` jobs = `gates` (OS-independent), `test-python`, `test-node`, `build` (OS matrix).
  Gate LOGIC lives in moon tasks (`ci/moon.yml`) + `scripts/ci/*.sh`, called via
  `mise exec -- moon run <task>`. The corpus gate must follow this pattern (locally runnable).

## Open questions to resolve in spec / clarify

- **OQ1 — Corpus fixture format & location.** A language-neutral fixture set (logical input + expected
  render + expected hashes) under a top-level `conformance/` dir? Each binding runs it via its own test
  harness. Need to decide: one shared JSON/YAML corpus consumed by all three, vs per-language mirrors.
  (Leaning: single shared corpus dir, three thin runners — that's what makes parity meaningful.)
- **OQ2 — How are "expected hashes" pinned without overfitting?** Golden hashes committed in the corpus
  (one canonical value per case, asserted equal by all three) vs cross-checking the three live outputs
  against each other at test time (no golden). Golden = also catches an engine-level regression; cross-
  check = pure parity. Possibly both: cross-check is the parity guarantee, golden is the regression guard.
- **OQ3 — Decimal source.** Python has `decimal.Decimal`; JS has no native decimal (bigint + number only);
  Rust has no std decimal. What is the *logical* Decimal case across bindings — a JSON number, a string?
  Must define a representable-everywhere logical value so "Decimal" is testable cross-language.
- **OQ4 — datetime/Date/chrono.** Python `datetime`, JS `Date`, Rust `chrono`. The kernel sees a serde
  value (string after marshaling?). Define the logical date case + expected rendered string so all three
  marshal to the same kernel value → same render.
- **OQ5 — Is the corpus a 4th CI job or folded into existing `test-python`/`test-node` + a new Rust leg?**
  Need a Rust conformance runner too (the consumer is a first-class binding for parity).

## Conflicts with durable memory

None. The roadmap §278 scope (marshaling + schema round-trip, NOT render parity) is exactly what the
constitution Principle VII + C-01/C-07 require. The handover and roadmap agree. No contradiction.
