# Feature Specification: Python binding (`prompting-press-py` → `packages/python`)

**Feature Branch**: `004-python-binding`

**Created**: 2026-06-27

**Status**: Draft

**Input**: User description: "Python binding (`prompting-press-py` → `packages/python`): the FIRST FFI binding over the spec-002 engine kernel — a `pip install prompting-press` package via PyO3 + maturin. Reproduces the spec-003 Rust consumer surface in Python idiom (Pydantic typed-Vars facade, dual-input loader, `check(registry)` agreement+provenance lint, `render()`/`get_source()`, `from_messages` composition, errors normalized to `[{field, code, message}]` then raised as Python exceptions). Marshaling + Pydantic facade ONLY — zero engine logic (Principle II / C-02). Pydantic shapes codegen'd from the JSON Schema (Principle VII / C-07). No I/O, no model calls, NO token counter (F4 — token hook out of scope). Depends on spec 002; mirrors spec 003. maturin wheel, abi3. Governed by C-02, C-06, C-07."

## Overview

`prompting-press-py` is the **first FFI binding** for the library — what a Python application gets from
`pip install prompting-press`. It is the binding the kernel/consumer split (specs 002/003) was built
to de-risk: a native extension module (PyO3, built and packaged as a maturin wheel) that exposes the
same four capabilities the spec-003 Rust consumer provides, rendered in **Python idiom**.

The kernel (`prompting-press-core`) turns *already-validated values + a prompt definition* into
*rendered text + provenance*, reports a template's required variables, and exposes a provenance view —
but it is validation-blind and language-agnostic by design. The spec-003 Rust consumer
(`prompting-press`) added the language-native layer for Rust: a typed-Vars facade, a dual-input loader,
the agreement + provenance lint, render/compose ergonomics, and error normalization. **This spec adds
the equivalent language-native layer for Python**, with the native systems of the Python ecosystem:
**Pydantic v2** for typed Vars + validators (Principle VI), Python **exceptions** for the normalized
error surface, and **maturin/PyO3** for packaging the shared Rust core into a wheel.

The defining constraint (constitution Principle II / roadmap decision C-02) is that this binding adds
**no engine logic**: rendering, the agreement analysis, variant resolution, and SHA-256 hashing all
live **once, in Rust** (the kernel, reached through the spec-003 Rust consumer). `pyo3` appears **only**
in `prompting-press-py`; the kernel and the Rust consumer stay FFI-free (the existing `ci:check-ffi`
gate enforces this). Cross-language render byte-identity is therefore a **structural property of the
shared core** (constitution Principle I) — it is **not** re-tested here.

This is the spec that makes the library's headline guarantee — *a template referencing an undeclared
variable is a caught error, not a silent empty render* — callable from Python, and that makes the
library usable by its first real consumer (Bellwether / `claudebroker`), whose language is Python.

## Clarifications

### Session 2026-06-27

The four design forks below are Python-idiom / packaging decisions with no exact spec-003 precedent
(003 was single-language Rust). All four are now resolved.

- Q: **Validation ownership & timing** — Pydantic validates at model construction; spec-003 validates
  at `render`. Does the binding own validation at the render boundary, or accept an already-constructed
  instance? → A: **The binding owns validation at the render boundary.** `render` accepts the caller's
  Pydantic Vars model + data (or an instance it re-validates), calls `model_validate` before any
  templating, and normalizes a Pydantic `ValidationError` into the library's exception (FR-014). This
  preserves SC-002/SC-006 and the cross-binding error contract; a native `ValidationError` never
  surfaces on the public API.
- Q: **Exception surface shape** — one exception type or a hierarchy? → A: **A small exception
  hierarchy under one base `PromptingPressError`** (e.g. `ValidationError`, `RenderError`/kernel,
  `UnknownPromptError`, `LoadError`), each carrying the `[{field, code, message}]` rows and the stable
  `code`. Maps 1:1 onto the Rust `ConsumerError` variants; gives Python callers idiomatic
  `except`-by-class granularity over the single structured contract.
- Q: **Loader locus** — reuse the Rust loader via FFI, or parse Python-side? → A: **Reuse the Rust
  consumer's dual-input loader via FFI.** YAML/JSON is marshaled in as **text** and parsed by the
  consumer's serde path; the registry is a binding wrapper over the spec-003 `Registry`. YAML↔JSON
  parity and malformed-input accept/reject become a **structural** property of the shared core
  (Principle I — the same argument as render parity), with no Python YAML dependency and no second
  loader to keep in agreement. The generated Pydantic `PromptDefinition` backs the **constructed-object**
  input path (Pydantic object → JSON → the consumer's `load_json`).
- Q: **Python version floor for the abi3 wheel** — the committed scaffold disagreed (crate `abi3-py39`
  vs `requires-python >=3.10` vs codegen targeting 3.10). Which floor is authoritative? → A: **Keep a
  broad abi3 install floor of CPython 3.10**, with the latest stable CPython as the build/dev/test
  target. The crate's `abi3-py39` is **bumped to `abi3-py310`** to match `requires-python >=3.10` and
  the codegen's `--target-python-version 3.10` (the generated `X | None` syntax does not import on 3.9,
  so `abi3-py39` was a latent "installs on 3.9 then ImportErrors" trap). One wheel runs 3.10 → latest.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Validate typed inputs and render through one idiomatic Python call (Priority: P1)

A Python application defines its prompt inputs as a Pydantic model with field validators, loads a
prompt, and renders it. Invalid inputs are rejected with a structured error *before* any templating
happens; valid inputs produce rendered text plus provenance. The application never touches the Rust
kernel directly and never sees a native Pydantic `ValidationError` or a Rust kernel error type.

**Why this priority**: This is the binding's reason to exist and the minimum viable Python consumer —
typed input + validation + render in one idiomatic call, with the shared Rust core wrapped invisibly.
Without it there is no Python binding. It exercises the FFI marshaling path end to end.

**Independent Test**: Define a Pydantic Vars model with a custom validator, load a prompt, and call
render with (a) valid values → rendered text + provenance, and (b) values that fail validation → a
structured Python exception listing the offending field(s), with no render attempted. Fully testable
with no other story present.

**Acceptance Scenarios**:

1. **Given** a Pydantic Vars value that satisfies all field validators and a loaded prompt, **When** the
   application renders, **Then** validation runs once, succeeds, the kernel renders, and the result
   carries the rendered text plus provenance (name, variant, `template_hash`, `render_hash`, optional
   guard).
2. **Given** a Pydantic Vars value that violates a field validator (e.g. an out-of-range number),
   **When** the application renders, **Then** a structured Python exception is raised carrying one row
   per offending field in the common `{field, code, message}` shape, and **no** render is performed.
3. **Given** a rendered result, **When** the application reads its provenance, **Then** the
   `template_hash` and `render_hash` are byte-identical to those the Rust consumer produces for the
   same logical prompt + inputs (a structural property of the shared core, not re-verified here).

---

### User Story 2 - Push prompt data as YAML, JSON, or a constructed object (Priority: P2)

A Python application populates a registry of prompts by pushing in prompt definitions — as a YAML
document, as a JSON document, or as a programmatically constructed definition object — and the binding
normalizes all three into the one prompt-definition representation.

**Why this priority**: Prompts are repo-canonical artifacts the application reads and pushes in (the
library does no I/O). Without the loader the application cannot get prompts into the binding. Builds on
US1's render path.

**Independent Test**: Load the same logical prompt three ways (YAML text, JSON text, constructed
object), then render each with identical inputs and confirm identical output; feed malformed input and
confirm a structured error with nothing partially loaded.

**Acceptance Scenarios**:

1. **Given** a prompt as a YAML document, the equivalent as a JSON document, and the equivalent as a
   constructed definition object, **When** each is loaded and rendered with identical inputs, **Then**
   all three produce identical render output and identical provenance.
2. **Given** malformed input (invalid YAML/JSON, or data violating the prompt-definition shape),
   **When** the application loads it, **Then** a structured Python exception is raised and **nothing**
   is partially loaded or silently coerced.
3. **Given** a registry populated with prompts, **When** the application renders or checks by name,
   **Then** a name absent from the registry surfaces as a structured exception, never a crash.

---

### User Story 3 - Run the agreement + provenance lint as a CI check from Python (Priority: P2)

A Python application (or its CI) loads its prompts into a registry and runs a single check that reports
every template referencing an undeclared variable, and every prompt that declares an
untrusted/external input without a guard configured — before anything is rendered.

**Why this priority**: This is the library's headline differentiator (constitution Principle IV), made
runnable from a Python CI pipeline. It is pure analysis and gates merges.

**Independent Test**: Build a registry containing (a) a prompt whose template references an undeclared
variable, (b) a prompt declaring an untrusted field with no guard, and (c) a clean prompt; run check;
confirm it reports (a) and (b) with prompt/variant/field detail, passes (c), and mutates/renders
nothing.

**Acceptance Scenarios**:

1. **Given** a prompt whose template references a variable absent from its declared `variables`,
   **When** check runs, **Then** it reports a finding naming the prompt, the variant, and the
   undeclared variable.
2. **Given** a prompt declaring an `untrusted`/`external` variable with no guard configured, **When**
   check runs, **Then** it reports a finding naming the prompt and the uncovered field.
3. **Given** a registry of only well-formed prompts, **When** check runs, **Then** it returns an empty
   report (pass), having rendered nothing and mutated nothing.

---

### User Story 4 - Compose a multi-message prompt (Priority: P3)

A Python application builds a multi-message prompt as an explicit ordered sequence of (prompt, vars,
variant) entries that resolves to an ordered list of `{role, text}` messages.

**Why this priority**: Few-shot / system+user sequences are a common consumer need, but render (US1) is
the prerequisite and the larger value. Composition is additive sugar over render.

**Independent Test**: Append three (prompt, vars) entries to a composition and resolve it; confirm
exactly three ordered `{role, text}` messages, each rendered with its own validated vars; confirm an
invalid entry fails the whole resolution without emitting a partial result.

**Acceptance Scenarios**:

1. **Given** an ordered sequence of N (prompt, vars, variant) entries, **When** the composition is
   resolved, **Then** it produces exactly N `{role, text}` messages in input order, each rendered with
   its own validated vars and tagged with that prompt's role.
2. **Given** a composition where one entry's vars fail validation (or its prompt is unknown), **When**
   resolution runs, **Then** a structured exception is raised and **no** partial message list is
   returned as success.
3. **Given** an empty composition, **When** it is resolved, **Then** it produces an empty message list.

---

### Edge Cases

- **Reserved `default` variant**: a prompt declaring a variant literally named `default` — the check
  reports it as a reserved-name finding (its declared arm is unreachable, shadowed by the root body),
  matching spec-003 behavior.
- **Un-analyzable template** (parse failure / excluded feature such as `{% include %}`): check records
  an analysis-error finding rather than crashing; check stays total.
- **Struct↔`variables` field-name mismatch**: a Pydantic Vars field misnamed relative to the prompt's
  declared `variables` is **not silent** — the marshaled value lacks the referenced root, so the
  kernel's strict-undefined fires and surfaces as an `undefined_variable`-class exception (never an
  empty render).
- **Secret in a bound value**: a value triggering a kernel parse/render error must never appear in the
  raised exception's message or any log derived from it (SEC-004 scrub preserved).
- **`output_model`**: carried as metadata only; never resolved, loaded, or parsed.
- **Marshaling edge values**: `None`/null, integers vs floats, nested objects, and dates/decimals
  marshal across the FFI boundary without loss or silent coercion (the broad cross-binding corpus is
  spec 006; this spec only requires correctness for the binding's own render/check paths).

## Requirements *(mandatory)*

### Functional Requirements

#### Typed Vars + validation (C-06, Principle VI)

- **FR-001**: The binding MUST let Python applications define typed input models in the native Python
  validation system (**Pydantic v2**) with custom field validators, rather than inventing a bespoke
  validation framework.
- **FR-002**: The binding MUST own validation at the render boundary (clarified Q1): `render` accepts
  the caller's Pydantic Vars model together with its data (or an already-constructed instance it
  re-validates) and runs validation **once, before any templating** (the whole input set validated
  together). If validation fails, no render is performed and the Pydantic `ValidationError` is
  normalized to the library's exception (FR-014) — a native `ValidationError` MUST NOT surface on the
  public API.
- **FR-003**: Validation MUST live in this Python binding layer; the binding MUST pass only
  already-validated values across the FFI boundary to the Rust core (the kernel stays validation-blind).
- **FR-003a**: After validation passes, the binding MUST marshal the validated Vars into the kernel's
  value type **losslessly** (no silent coercion of `None`, int/float, nested structures); the caller
  MUST NOT have to hand-build a value map.
- **FR-004**: Native validator outputs (Pydantic's `ValidationError`) MUST NOT be exposed on the
  binding's public API; they are normalized first (FR-014).

#### Dual-input loader (C-07, Principle VII)

- **FR-005**: The binding MUST accept prompt data pushed as a YAML document, as a JSON document, or as
  a programmatically constructed definition object, and normalize all three into one internal
  prompt-definition representation. YAML/JSON text MUST be parsed by **reusing the Rust consumer's
  dual-input loader across the FFI boundary** (clarified Q3 — text marshaled in, parsed by the
  consumer's serde path), so accept/reject behavior and YAML↔JSON parity are structural properties of
  the shared core (no Python-side YAML parser, no second loader). The constructed-object path takes a
  generated-Pydantic `PromptDefinition` and routes it through the same loader (object → JSON → the
  consumer's JSON load).
- **FR-006**: A prompt definition loaded from YAML and the equivalent loaded from JSON MUST produce
  identical internal representations and identical downstream behavior.
- **FR-007**: Malformed input (invalid YAML/JSON, or data that violates the prompt-definition shape)
  MUST produce a structured exception; the binding MUST NOT partially load or silently coerce.
- **FR-008**: The binding MUST consume the spec-001 prompt-definition shape as its single definition
  representation. The Python-side prompt-definition shape (Pydantic model) MUST be **code-generated from
  the JSON Schema** (the existing `datamodel-code-generator` pipeline), never hand-maintained in
  parallel (Principle VII / C-07).
- **FR-008a**: The binding MUST provide a **registry** — a library-owned collection mapping a prompt
  name to its loaded prompt definition — that the application loads prompts into. `render(name, …)`
  resolves against this registry, and the check (FR-016) runs over it. A name absent from the registry
  MUST surface as a structured exception, not a crash.

#### Render, get-source & composition (C-01, C-06)

- **FR-009**: The binding MUST expose an idiomatic `render`-style operation that takes a prompt **name
  resolved against the registry** and the caller's typed Vars value together, validates the vars, then
  delegates rendering to the Rust core, returning the rendered text plus provenance (name, variant,
  `template_hash`, `render_hash`, optional guard). The binding MUST NOT require pre-registering a Vars
  type per prompt. Guard *expansion* is owned and tested by the kernel; the binding only plumbs guard
  configuration through and surfaces the resulting guard field.
- **FR-010**: The binding MUST expose a `get_source`-style operation returning a prompt variant's
  unrendered template source, delegating to the Rust core.
- **FR-011**: The binding MUST NOT reimplement rendering, agreement analysis, variant resolution, or
  hashing; these live once in the Rust core and are reached across the FFI boundary (Principle I /
  C-02 — no engine logic in the binding).
- **FR-012**: The binding MUST support composing a multi-message prompt as an **explicit ordered
  sequence** of (prompt, vars, variant) entries (a `from_messages`-style constructor over an ordered
  array) that resolves to an ordered list of `{role, text}` messages.
- **FR-013**: The binding MUST NOT offer a fluent `.chain()` composition API.

#### Error normalization → Python exceptions (C-06, Principle VI)

- **FR-014**: The binding MUST normalize both validation failures (Pydantic `ValidationError`) and Rust
  core errors (the closed `KernelError`; loader errors) into one common structured shape — rows of
  `{field, code, message}` — and raise them as **Python exceptions**; native error types (Pydantic's
  `ValidationError`, the Rust error types) MUST NOT cross the FFI boundary onto the public API. The
  `code` values MUST be drawn from the same stable, closed vocabulary the Rust consumer uses
  (`validation`, `unknown_prompt`, `unknown_variant`, `undefined_variable`, `parse`, `render`,
  `excluded_feature`, `load`) so the error contract is identical across bindings. The exceptions MUST
  form a small **hierarchy under one base `PromptingPressError`** (clarified Q2 — e.g. a
  validation-class, a kernel/render-class, an unknown-prompt-class, and a load-class subtype), each
  carrying the `[{field, code, message}]` rows and mapping 1:1 onto the Rust `ConsumerError` variants,
  so a Python caller can `except` by class or branch on `code`.
- **FR-015**: Error normalization MUST NOT echo raw, potentially sensitive bound-value content into
  exception messages or logs (the SEC-004 scrub: `parse`/`render`/`excluded_feature` detail is replaced
  by a fixed message).

#### Agreement + provenance lint (C-04, C-09)

- **FR-016**: The binding MUST expose a single check operation, runnable as a CI/lint pass over a
  registry of prompts, that verifies for each prompt+variant that the template's referenced variables
  are a subset of that prompt's declared variables, and reports any variable referenced but not
  declared.
- **FR-017**: The check MUST obtain referenced variables and the provenance view from the Rust core's
  analysis (the binding does not re-derive them). The authoritative "declared variables" set is the
  prompt **definition's `variables` block**, not the Pydantic Vars model — the check is pure data and
  MUST NOT require introspecting the caller's Pydantic types.
- **FR-018**: The check MUST include a provenance lint: a prompt that declares one or more `untrusted`
  or `external` variables (via the kernel's provenance view) but configures **no guard** for them (the
  `meta`/`metadata` `guard`-key convention, per spec 003) MUST be reported, naming the prompt and the
  uncovered field(s).
- **FR-019**: The check MUST be pure analysis — pass/fail — and MUST NOT mutate any prompt, definition,
  or input, render anything, or produce side effects.
- **FR-020**: Check findings MUST be actionable: each identifies the prompt, the variant where
  applicable, and the offending variable/field; the reserved-`default`-name and un-analyzable-template
  cases are reported as distinct finding kinds (spec-003 parity).

#### Packaging & boundary (C-02, C-03, Principle II/III)

- **FR-021**: The binding MUST be packaged as a maturin wheel targeting the stable `abi3` ABI with a
  CPython **3.10** floor (clarified Q4 — one wheel runs on CPython 3.10 → latest; the crate's
  `abi3-py39` is bumped to `abi3-py310` so the ABI floor, `requires-python >=3.10`, and the codegen's
  3.10 target syntax all agree), importable as the `prompting_press` module, distributed on PyPI as
  `prompting-press`. The latest stable CPython is the build/dev/test target. (Actual publish is spec
  007; this spec produces a locally buildable, installable, importable wheel.)
- **FR-022**: `pyo3` (and any FFI toolkit dependency) MUST appear **only** in `prompting-press-py`; the
  kernel and the Rust consumer MUST stay FFI-free (the `ci:check-ffi` gate MUST stay green).
- **FR-023**: The binding MUST NOT perform I/O (no file/network/database/environment access), make
  model calls, assemble provider request bodies, parse model output, or count tokens. The
  `output_model` reference is carried as metadata only. **No token-count hook or token counter ships**
  (consistent with spec-003 refinement F4; the token surface is deferred — see the roadmap-line
  reconciliation in Assumptions).
- **FR-024**: The generated Python prompt-definition shape MUST stay in sync with the JSON Schema via
  the existing codegen freshness gate; a schema change not regenerated into the Python shape is a build
  failure (Principle VII).
- **FR-025**: The Python package's dependencies MUST be covered by a CI vulnerability/advisory gate. The
  repo's existing advisory gate (`ci:check-advisories`, cargo-deny) is Rust-only and gives no CVE
  coverage for the Python deps (Pydantic, maturin, datamodel-code-generator); this spec adds a Python
  advisory gate (`pip-audit`/`osv-scanner` over `packages/python/uv.lock`) so a known-vulnerable Python
  dependency fails CI (security review SEC-101).

### Key Entities *(include if feature involves data)*

- **Typed Vars model**: an application-defined Pydantic v2 model carrying the prompt's inputs and their
  validators; validated as a whole, then marshaled into the kernel's value type. Authored by the
  application, not the library; passed to `render` alongside the prompt name.
- **Prompt definition (input)**: the spec-001 shape — name, role, body, declared `variables` (each with
  a provenance tag — the authoritative declared set for the agreement check), variants, opaque
  `meta`/`metadata`, `output_model` reference. The Python shape is code-generated from the JSON Schema;
  consumed, not redefined.
- **Registry**: a library-owned map of prompt name → loaded prompt definition. The application loads
  prompts into it; `render(name, …)` resolves against it and `check(registry)` lints over it.
- **Render result**: rendered text plus provenance (name, variant, `template_hash`, `render_hash`,
  optional guard), surfaced from the Rust core as library-owned Python data.
- **Normalized error / exception**: the common `{field, code, message}` rows, raised as a Python
  exception; the single error contract shared across all bindings.
- **Message (composition output)**: an ordered `{role, text}` entry; a composition resolves to a list
  of these.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A Python application can define Pydantic Vars with custom validators, load a prompt, and
  render — reaching no Rust kernel type and no native Pydantic error type on the public API — in a
  single idiomatic call path (validate-then-render).
- **SC-002**: Invalid inputs are rejected before render in 100% of cases, with a structured exception
  naming every offending field; a render is never performed on invalid input.
- **SC-003**: The same logical prompt loaded from YAML, from JSON, and from a constructed object yields
  identical internal representations and identical render output for identical inputs (100% parity
  across the three input forms).
- **SC-004**: The agreement check detects an undeclared-variable reference in 100% of seeded cases and
  reports the prompt/variant/variable; it passes clean prompts; it mutates nothing and renders nothing.
- **SC-005**: The provenance lint flags, in 100% of seeded cases, a prompt that declares an
  untrusted/external variable with no guard configured (naming the prompt + field).
- **SC-006**: No native error type (Pydantic `ValidationError`, Rust kernel error) appears on the
  binding's public API; every error surfaces as the common `{field, code, message}` shape via a Python
  exception, with `code` from the shared closed vocabulary.
- **SC-007**: `prompting-press-py` is the only crate with a `pyo3` dependency (the `ci:check-ffi` gate
  passes), and the binding contains no rendering/agreement/variant/hashing logic of its own (it
  delegates across the FFI boundary to the Rust core).
- **SC-008**: A multi-message composition of N (prompt, vars) entries resolves to exactly N ordered
  `{role, text}` messages in input order, each rendered with its own validated vars.
- **SC-009**: `maturin build` (or equivalent) produces an installable abi3 wheel; in a fresh Python
  environment `import prompting_press` succeeds and the render/check/compose paths execute against the
  compiled core.
- **SC-010**: The generated Python prompt-definition shape is byte-identical to a fresh regeneration
  from the JSON Schema (the codegen freshness gate passes); no token-counting surface exists anywhere
  in the package.
- **SC-011**: A CI advisory gate scans the Python dependencies (`packages/python/uv.lock`) for known
  CVEs and fails on a vulnerable dependency — the Python deps are no longer outside CVE coverage
  (closing security-review SEC-101).

## Assumptions

- **Pydantic v2 is the Python validation system** (per the resolved design; Principle VI). The exact
  current Pydantic, PyO3, maturin, and datamodel-code-generator versions/APIs are confirmed at planning
  time (verify-at-spec-time discipline — subagent-reported versions have been wrong before; check PyPI
  / crates.io directly).
- **The kernel (spec 002) and Rust consumer (spec 003) are the dependencies** and provide
  render/get_source/required_roots/provenance_view + the result/error/report types; this binding wraps
  them across FFI and normalizes to the shared error contract. The `prompting-press-py` crate already
  declares path deps on both (verified).
- **The codegen pipeline exists** (verified): `packages/python/scripts/codegen.sh` +
  `datamodel-code-generator==0.65.1` (uv-locked) generate the Pydantic `PromptDefinition` from
  `schemas/jsonschema/prompt-definition.schema.json`; the generated model is present. The Pydantic Vars
  models the *caller* writes are separate, application-authored, and not codegen'd.
- **Validation ownership (resolved Q1)**: the binding **owns validation at the render boundary** —
  accepts a Pydantic model + data (or an instance it re-validates), catches Pydantic's `ValidationError`
  and normalizes it (FR-014). This keeps C-06 intact (no native `ValidationError` escapes) and matches
  spec-003's "validate at render" guarantee.
- **Exception shape (resolved Q2)**: a small exception **hierarchy** under one base
  `PromptingPressError` (a validation-class, a kernel/render-class, an unknown-prompt-class, a
  load-class), each carrying the `[{field, code, message}]` rows and the stable `code`, mapping 1:1 onto
  the Rust `ConsumerError` variants. (Exact subtype names finalized at plan/design time.)
- **Loader locus (resolved Q3)**: **reuse the Rust consumer's dual-input loader** (marshal YAML/JSON
  text across the FFI boundary; parse with the consumer's serde path). Makes YAML↔JSON parity and
  accept/reject behavior a *structural* property of the shared core (same argument as render parity —
  Principle I), avoids a Python YAML dependency, and keeps "one loader, one representation" literally
  singular. The generated Pydantic `PromptDefinition` backs the *constructed-object* input path and the
  typed Python view.
- **Python floor (resolved Q4)**: broad **abi3 floor of CPython 3.10**, latest stable CPython as the
  build/dev/test target; the crate's `abi3-py39` is bumped to `abi3-py310`. This reconciles the
  committed scaffold's three-way disagreement (crate `abi3-py39` vs `requires-python >=3.10` vs codegen
  `--target-python-version 3.10`) — the generated `X | None` syntax fails to import on 3.9, so
  `abi3-py39` was a latent install-then-ImportError trap. The exact "latest stable" CPython version is
  confirmed at plan time (python.org), not guessed here.
- **Three-sets invariant** (spec-003 critique E1): the caller's Pydantic Vars field names must agree
  with the prompt's declared `variables` block. `check()` validates template-roots ⊆ `variables` (a CI
  lint), and Pydantic validates the value's *contents* — but the *Vars↔`variables`* field-name
  agreement is the caller's responsibility. A mismatch is not silent: it surfaces as a loud
  `undefined_variable`-class exception from the kernel, documented and pinned by a test, not enforced by
  an extra check.
- **Token surface — roadmap reconciliation**: the roadmap 004 entry's `Scope (in)` still lists a "token
  hook". This is **stale**: spec 003 dropped the token surface entirely (refinement F4) and deferred it
  to the Deferred "Token budgeting / truncation" entry. This spec **drops** the token hook from 004's
  scope, consistent with F4. The roadmap 004 (and 005) `Scope (in)` lines should be amended to remove
  "token hook" (proposed at plan/roadmap-sync time; recorded here so it is not silently carried).
- **No cross-language conformance work here**: the FFI conformance corpus (broad marshaling fidelity +
  schema round-trip across all three bindings) is spec 006; render-byte-parity is structural
  (Principle I). This spec verifies marshaling only for its own render/check/compose paths.
- **No publish here**: PyPI publish + release tooling is spec 007. This spec produces a locally
  buildable/installable/importable wheel.

## Dependencies

- **Spec 002 (Engine kernel) — satisfied/merged**: the kernel API this binding marshals to
  (`render`, `get_source`, `required_roots`, `provenance_view`), the result/`GuardConfig` types, the
  closed `KernelError` enum, and the re-exported `PromptDefinition` shape.
- **Spec 003 (Rust consumer) — satisfied/merged**: the reference surface this binding reproduces in
  Python idiom, and (per Q3 default) the dual-input loader + registry + `check()` + error-`code`
  vocabulary the binding reuses across FFI.
- **Spec 001 (Foundations) — satisfied/merged**: the prompt-definition JSON Schema, the Python codegen
  pipeline, the `prompting-press-py` crate + `packages/python` package scaffolds, the `ci:check-ffi`
  and codegen-freshness gates.

## Governance Alignment

Governed by constitution Principles **I** (shared core — no engine logic in the binding; render parity
structural), **II** (FFI isolation — `pyo3` only in `prompting-press-py`; kernel + Rust consumer stay
FFI-free), **III** (minimal boundary — no I/O, no model calls, no token counting; token surface
deferred per F4), **VI** (per-language idiom — Pydantic, `from_messages` not `.chain()`, errors as
Python exceptions normalized to the common shape), and **VII** (JSON Schema single source — Python shape
codegen'd, dual-input loader into the one shape), and by roadmap decisions **C-02, C-06, C-07** (plus
C-04/C-09 surfaced via the lint). No new pluggable interface; no boundary-expanding capability added.
