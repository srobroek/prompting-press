# Feature Specification: TypeScript binding (`prompting-press-node` → `packages/typescript`)

**Feature Branch**: `005-ts-binding`

**Created**: 2026-06-27

**Status**: Draft

**Input**: User description: "TypeScript binding for Prompting Press — `prompting-press-node` (the napi-rs binding crate) shipped as the npm package under `packages/typescript`. The SECOND FFI binding; mirrors the spec-004 Python binding in TypeScript idiom over the SAME shared Rust core (spec 002) + Rust consumer (spec 003). Zero engine logic in the binding (C-02 / Principle I). Scope (in): napi-rs marshaling bridge; a Zod-based typed Vars facade with `.refine()` validators; `check(registry)` agreement+provenance lint; a dual-input loader reusing the Rust consumer's loader via FFI; `from_messages` composition (never `.chain()`); errors normalized to `[{field, code, message}]` raised as JS Errors with native types never crossing FFI; SEC-004 secret-scrub preserved; TS prompt-definition shape codegen'd from the JSON Schema (C-07); napi-rs platform-binary npm packaging. Scope (out): engine logic; any token counter (F4); `.chain()`; I/O / LLM calls / request-body assembly. Governed by C-02, C-06, C-07. The conformance corpus (spec 006) depends on this second binding, so marshaling fidelity is a first-class concern. `napi` appears ONLY in `prompting-press-node`."

## Overview

`prompting-press-node` is the **second FFI binding** for the library — what a Node/TypeScript
application gets from `npm i prompting-press`. Where spec 004 added the Python-native layer, this spec
adds the equivalent **TypeScript-native layer** over the same shared Rust core, using the native systems
of the JS/TS ecosystem: **Zod** for typed Vars + custom validators (Principle VI), JavaScript **`Error`
subclasses** for the normalized error surface, and **napi-rs** for packaging the shared Rust core into
per-platform native npm packages.

The kernel (`prompting-press-core`) turns *already-validated values + a prompt definition* into
*rendered text + provenance*, reports a template's required variables, and exposes a provenance view —
validation-blind and language-agnostic by design. The spec-003 Rust consumer (`prompting-press`) and the
spec-004 Python binding each added a language-native layer over it. **This spec adds that layer for
TypeScript.**

The defining constraint (constitution Principle II / roadmap decision C-02) is that this binding adds
**no engine logic**: rendering, the agreement analysis, variant resolution, and SHA-256 hashing all live
**once, in Rust** (the kernel, reached through the spec-003 Rust consumer). `napi`/`napi-derive` appear
**only** in `prompting-press-node`; the kernel and the Rust consumer stay FFI-free (the existing
`ci:check-ffi` gate enforces this — it ALREADY asserts both `pyo3` AND `napi` absence (the gate's
`FFI_CRATES=("pyo3" "napi")` shipped in spec 001), so this spec VERIFIES the gate covers `napi`, it does
not add the assertion).
Cross-language render byte-identity is therefore a **structural property of the shared core**
(constitution Principle I) — it is **not** re-tested here.

This is also the spec that **makes the FFI boundary real**: with a second binding over the same core, the
conformance corpus (spec 006) can finally test that two independent FFI layers marshal values
identically. Marshaling fidelity across the napi boundary (dates, `bigint` vs `number`, nested objects,
`null` vs `undefined`, integer vs float) is therefore a first-class concern of this spec — but only for
the binding's own render/check/compose paths; the broad cross-binding corpus is spec 006.

## Clarifications

### Session 2026-06-27

The Python binding (spec 004) resolved four design forks (validation ownership, error-surface shape,
loader locus, ABI floor). Three carry directly into TypeScript with the same rationale; the
TS-idiom-specific decisions below are resolved with their rationale, and the genuinely open packaging
question is marked.

- Q: **Validation ownership & timing** (parallels 004 Q1) — Zod parses/validates eagerly, like Pydantic.
  Does the binding own validation at the render boundary? → A: **Yes — the binding owns validation at the
  render boundary.** `render` accepts the caller's Zod schema + data (or pre-parsed data it re-parses via
  `safeParse`), validates **once before any templating**, and normalizes a `ZodError` into the library's
  error (FR-014). A native `ZodError` never surfaces on the public API. Matches spec-003/004's
  "validate at render."
- Q: **Error-surface shape** (parallels 004 Q2) — one error type or a hierarchy? → A: **A small `Error`
  subclass hierarchy under one base `PromptingPressError`** (e.g. `PromptValidationError`,
  `PromptRenderError`, `UnknownPromptError`, `LoadError`), each carrying the `[{field, code, message}]`
  rows and the stable `code`, mapping 1:1 onto the Rust `ConsumerError` variants. Gives TS callers
  idiomatic `instanceof`/`catch`-by-class granularity over the single structured contract.
- Q: **Loader locus** (parallels 004 Q3) — reuse the Rust loader via FFI, or parse JS-side? → A: **Reuse
  the Rust consumer's dual-input loader via FFI.** YAML/JSON is marshaled in as **text** and parsed by
  the consumer's serde path, so YAML↔JSON parity and malformed-input accept/reject are a **structural**
  property of the shared core (no JS-side YAML dependency, no second loader). The generated TS
  `PromptDefinition` type backs the **constructed-object** input path (object → JSON → the consumer's
  `load_json`).
- Q: **Vars facade — is Zod required, or is a plain typed object accepted?** → A: **Zod is the native
  validation system (Principle VI), but a validator is not mandatory.** `render` accepts a Zod schema +
  data (validated via `safeParse`); for callers who only want types with no runtime validation, the
  binding also accepts plain already-typed data and marshals it directly. Custom validators use Zod
  `.refine()`/`.superRefine()`. (This mirrors how 004 accepts a Pydantic model *or* a constructed
  instance.)
- Q: **npm packaging shape for the native binary** — how is the platform-specific `.node` binary
  distributed? → A: **The napi-rs standard: a main `prompting-press` package with per-platform native
  binaries published as `optionalDependencies`** (e.g. `@prompting-press/node-linux-x64-gnu`,
  `-darwin-arm64`, `-win32-x64-msvc`), selected at install by `os`/`cpu`. The `@napi-rs/cli`
  `build`/`prepublish` flow already scaffolded in `packages/typescript/package.json` produces this. Exact
  platform-triple matrix is finalized at plan time. (Actual publish is spec 007; this spec produces a
  locally buildable, installable, importable package.)
- Q: **`null` vs `undefined` vs absent in the marshaling bridge** — how does a JS Vars value map to the
  kernel value? → A: **`undefined` and an absent object field both marshal as "field not present"**
  (the kernel sees no value for that root → the strict-undefined path fires if the template references
  it); an **explicit `null` marshals as JSON `null`**. This matches the Python binding's `None`-vs-absent
  handling and the kernel's serde model, keeping the two bindings consistent for the spec-006 conformance
  corpus (FR-003a).
- Q: **Zod major version** — which line does the Vars facade target (the `ZodError`→rows mapper depends
  on the issue API)? → A: **Zod v4 (latest stable)**. The exact stable version and the `.issues`/error
  shape the mapper reads are confirmed at plan time (verify-at-spec-time). The mapper targets v4's issue
  format only (not a v3/v4 dual range) — one issue shape, less mapper surface (FR-001/FR-014).
- Q: **Module format the npm package ships** — ESM, CJS, or dual? → A: **ESM-only**, consistent with the
  scaffold's existing `napi build --esm` flag. Consumers are Node 16+ / bundler environments; no
  CommonJS entry point ships (FR-021).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Validate typed inputs and render through one idiomatic TypeScript call (Priority: P1)

A TypeScript application defines its prompt inputs as a Zod schema with custom refinements, loads a
prompt, and renders it. Invalid inputs are rejected with a structured error *before* any templating;
valid inputs produce rendered text plus provenance. The application never touches the Rust kernel
directly and never sees a native `ZodError` or a Rust kernel error type.

**Why this priority**: This is the binding's reason to exist and the minimum viable TS consumer — typed
input + validation + render in one idiomatic call, with the shared Rust core wrapped invisibly. Without
it there is no TypeScript binding. It exercises the napi marshaling path end to end.

**Independent Test**: Define a Zod Vars schema with a `.refine()` validator, load a prompt, and call
render with (a) valid values → rendered text + provenance, and (b) values that fail validation → a
structured JS error listing the offending field(s), with no render attempted. Fully testable with no
other story present.

**Acceptance Scenarios**:

1. **Given** a Zod Vars value that satisfies all refinements and a loaded prompt, **When** the
   application renders, **Then** validation runs once, succeeds, the kernel renders, and the result
   carries the rendered text plus provenance (name, variant, `templateHash`, `renderHash`, optional
   guard).
2. **Given** a Zod Vars value that violates a refinement (e.g. an out-of-range number), **When** the
   application renders, **Then** a structured JS error is thrown carrying one row per offending field in
   the common `{field, code, message}` shape, and **no** render is performed.
3. **Given** a rendered result, **When** the application reads its provenance, **Then** `templateHash`
   and `renderHash` are byte-identical to those the Rust consumer and the Python binding produce for the
   same logical prompt + inputs (a structural property of the shared core, not re-verified here).

---

### User Story 2 - Push prompt data as YAML, JSON, or a constructed object (Priority: P2)

A TypeScript application populates a registry of prompts by pushing in prompt definitions — as a YAML
document, a JSON document, or a programmatically constructed definition object — and the binding
normalizes all three into the one prompt-definition representation.

**Why this priority**: Prompts are repo-canonical artifacts the application reads and pushes in (the
library does no I/O). Without the loader the application cannot get prompts into the binding. Builds on
US1's render path.

**Independent Test**: Load the same logical prompt three ways (YAML text, JSON text, constructed
object), render each with identical inputs and confirm identical output; feed malformed input and
confirm a structured error with nothing partially loaded.

**Acceptance Scenarios**:

1. **Given** a prompt as a YAML document, the equivalent as a JSON document, and the equivalent as a
   constructed definition object, **When** each is loaded and rendered with identical inputs, **Then**
   all three produce identical render output and identical provenance.
2. **Given** malformed input (invalid YAML/JSON, or data violating the prompt-definition shape), **When**
   the application loads it, **Then** a structured JS error is thrown and **nothing** is partially loaded
   or silently coerced.
3. **Given** a registry populated with prompts, **When** the application renders or checks by name,
   **Then** a name absent from the registry surfaces as a structured error, never a crash.

---

### User Story 3 - Run the agreement + provenance lint as a CI check from TypeScript (Priority: P2)

A TypeScript application (or its CI) loads its prompts into a registry and runs a single check that
reports every template referencing an undeclared variable, and every prompt that declares an
untrusted/external input without a guard configured — before anything is rendered.

**Why this priority**: This is the library's headline differentiator (constitution Principle IV), made
runnable from a Node CI pipeline. It is pure analysis and gates merges.

**Independent Test**: Build a registry containing (a) a prompt whose template references an undeclared
variable, (b) a prompt declaring an untrusted field with no guard, and (c) a clean prompt; run check;
confirm it reports (a) and (b) with prompt/variant/field detail, passes (c), and mutates/renders
nothing.

**Acceptance Scenarios**:

1. **Given** a prompt whose template references a variable absent from its declared `variables`, **When**
   check runs, **Then** it reports a finding naming the prompt, the variant, and the undeclared variable.
2. **Given** a prompt declaring an `untrusted`/`external` variable with no guard configured, **When**
   check runs, **Then** it reports a finding naming the prompt and the uncovered field.
3. **Given** a registry of only well-formed prompts, **When** check runs, **Then** it returns an empty
   report (pass), having rendered nothing and mutated nothing.

---

### User Story 4 - Compose a multi-message prompt (Priority: P3)

A TypeScript application builds a multi-message prompt as an explicit ordered array of (prompt, vars,
variant) entries that resolves to an ordered array of `{role, text}` messages.

**Why this priority**: Few-shot / system+user sequences are a common consumer need, but render (US1) is
the prerequisite and the larger value. Composition is additive sugar over render.

**Independent Test**: Build a composition of three (prompt, vars) entries and resolve it; confirm exactly
three ordered `{role, text}` messages, each rendered with its own validated vars; confirm an invalid
entry fails the whole resolution without emitting a partial result.

**Acceptance Scenarios**:

1. **Given** an ordered array of N (prompt, vars, variant) entries, **When** the composition is resolved,
   **Then** it produces exactly N `{role, text}` messages in input order, each rendered with its own
   validated vars and tagged with that prompt's role.
2. **Given** a composition where one entry's vars fail validation (or its prompt is unknown), **When**
   resolution runs, **Then** a structured error is thrown and **no** partial message array is returned as
   success.
3. **Given** an empty composition, **When** it is resolved, **Then** it produces an empty message array.

---

### Edge Cases

- **Reserved `default` variant**: a prompt declaring a variant literally named `default` — the check
  reports it as a reserved-name finding (its declared arm is unreachable, shadowed by the root body),
  matching spec-003/004 behavior.
- **Un-analyzable template** (parse failure / excluded feature such as `{% include %}`): check records an
  analysis-error finding rather than crashing; check stays total.
- **Schema↔`variables` field-name mismatch**: a Zod Vars field misnamed relative to the prompt's declared
  `variables` is **not silent** — the marshaled value lacks the referenced root, so the kernel's
  strict-undefined fires and surfaces as an `undefined_variable`-class error (never an empty render).
- **Secret in a bound value**: a value triggering a kernel parse/render error must never appear in the
  thrown error's message, `.stack`, or any log derived from it (SEC-004 scrub preserved). The `ZodError`
  mapper copies only the issue message, never the rejected input value.
- **`outputModel`**: carried as metadata only; never resolved, loaded, or parsed.
- **Marshaling edge values**: `null` vs `undefined`, integers vs floats, `bigint`, nested objects, and
  dates marshal across the napi boundary without loss or silent coercion (the broad cross-binding corpus
  is spec 006; this spec only requires correctness for the binding's own render/check paths). `undefined`
  in an object field and an absent field are treated consistently with how the kernel's serde model and
  the Python binding treat `None`/absent.

## Requirements *(mandatory)*

### Functional Requirements

#### Typed Vars + validation (C-06, Principle VI)

- **FR-001**: The binding MUST let TypeScript applications define typed input models in the native JS/TS
  validation system (**Zod v4** — clarified Q7) with custom validators (`.refine()`/`.superRefine()`),
  rather than inventing a bespoke validation framework. Static-only typing (a plain typed object, no
  runtime validator) MUST also be accepted (clarified Q4). The `ZodError`→rows mapper (FR-014) targets
  Zod v4's issue API only (not a v3/v4 dual range).
- **FR-002**: The binding MUST own validation at the render boundary (clarified Q1): `render` accepts the
  caller's Zod schema together with its data (or already-typed data it re-validates) and runs validation
  **once, before any templating** (the whole input set validated together). If validation fails, no
  render is performed and the `ZodError` is normalized to the library's error (FR-014) — a native
  `ZodError` MUST NOT surface on the public API.
- **FR-003**: Validation MUST live in this TypeScript binding layer; the binding MUST pass only
  already-validated values across the FFI boundary to the Rust core (the kernel stays validation-blind).
- **FR-003a**: After validation passes, the binding MUST marshal the validated Vars into the kernel's
  value type **losslessly** (no silent coercion of `null`/`undefined`, integer/float, `bigint`, nested
  structures, dates); the caller MUST NOT have to hand-build a value map. The `null`/`undefined`/absent
  treatment is fixed (clarified Q6): **`undefined` and an absent object field both marshal as "field not
  present"** (no value for that root — the kernel's strict-undefined path fires if the template
  references it), and an **explicit `null` marshals as JSON `null`**. This MUST match the Python
  binding's `None`-vs-absent handling and the kernel's serde model, so the two bindings stay consistent
  for the spec-006 conformance corpus.
- **FR-004**: Native validator outputs (`ZodError`) MUST NOT be exposed on the binding's public API; they
  are normalized first (FR-014).

#### Dual-input loader (C-07, Principle VII)

- **FR-005**: The binding MUST accept prompt data pushed as a YAML document, a JSON document, or a
  programmatically constructed definition object, and normalize all three into one internal
  prompt-definition representation. YAML/JSON text MUST be parsed by **reusing the Rust consumer's
  dual-input loader across the FFI boundary** (clarified Q3 — text marshaled in, parsed by the consumer's
  serde path), so accept/reject behavior and YAML↔JSON parity are structural properties of the shared
  core (no JS-side YAML parser, no second loader). The constructed-object path takes a generated-TS
  `PromptDefinition` shape and routes it through the same loader (object → JSON → the consumer's JSON
  load).
- **FR-006**: A prompt definition loaded from YAML and the equivalent loaded from JSON MUST produce
  identical internal representations and identical downstream behavior.
- **FR-007**: Malformed input (invalid YAML/JSON, or data that violates the prompt-definition shape) MUST
  produce a structured error; the binding MUST NOT partially load or silently coerce.
- **FR-008**: The binding MUST consume the spec-001 prompt-definition shape as its single definition
  representation. The TypeScript-side prompt-definition shape MUST be **code-generated from the JSON
  Schema** (the existing `json-schema-to-typescript` pipeline at `packages/typescript/scripts/codegen.mjs`
  → `src/generated/prompt-definition.ts`), never hand-maintained in parallel (Principle VII / C-07).
- **FR-008a**: The binding MUST provide a **registry** — a library-owned collection mapping a prompt name
  to its loaded prompt definition — that the application loads prompts into. `render(name, …)` resolves
  against this registry, and the check (FR-016) runs over it. A name absent from the registry MUST
  surface as a structured error, not a crash.

#### Render, get-source & composition (C-01, C-06)

- **FR-009**: The binding MUST expose an idiomatic `render`-style operation that takes a prompt **name
  resolved against the registry** and the caller's typed Vars value together, validates the vars, then
  delegates rendering to the Rust core, returning the rendered text plus provenance (name, variant,
  `templateHash`, `renderHash`, optional guard). The binding MUST NOT require pre-registering a Vars type
  per prompt. Guard *expansion* is owned and tested by the kernel; the binding only plumbs guard
  configuration through and surfaces the resulting guard field.
- **FR-010**: The binding MUST expose a `getSource`-style operation returning a prompt variant's
  unrendered template source, delegating to the Rust core.
- **FR-011**: The binding MUST NOT reimplement rendering, agreement analysis, variant resolution, or
  hashing; these live once in the Rust core and are reached across the FFI boundary (Principle I / C-02 —
  no engine logic in the binding).
- **FR-012**: The binding MUST support composing a multi-message prompt as an **explicit ordered array**
  of (prompt, vars, variant) entries (a `fromMessages`-style constructor over an ordered array, with
  idiomatic builder sugar permitted) that resolves to an ordered array of `{role, text}` messages.
- **FR-013**: The binding MUST NOT offer a fluent `.chain()` composition API (it cannot cross the napi
  boundary and collides with JS idiom).

#### Error normalization → JS errors (C-06, Principle VI)

- **FR-014**: The binding MUST normalize both validation failures (`ZodError`) and Rust core errors (the
  closed `KernelError`; loader errors) into one common structured shape — rows of `{field, code,
  message}` — and throw them as **JavaScript `Error` instances**; native error types (`ZodError`, the
  Rust error types) MUST NOT cross the FFI boundary onto the public API. The `code` values MUST be drawn
  from the same stable, closed vocabulary the Rust consumer and the Python binding use (`validation`,
  `unknown_prompt`, `unknown_variant`, `undefined_variable`, `parse`, `render`, `excluded_feature`,
  `load`) so the error contract is identical across bindings. The errors MUST form a small **hierarchy
  under one base `PromptingPressError`** (clarified Q2 — e.g. a validation-class, a kernel/render-class,
  an unknown-prompt-class, and a load-class subtype), each exposing the `[{field, code, message}]` rows
  and mapping 1:1 onto the Rust `ConsumerError` variants, so a TS caller can branch by `instanceof` or on
  `code`.
- **FR-015**: Error normalization MUST NOT echo raw, potentially sensitive bound-value content into error
  messages, `.stack`, or logs (the SEC-004 scrub: `parse`/`render`/`excluded_feature` detail is replaced
  by a fixed message). The `ZodError` mapper MUST copy only the issue's `message` + `path`, never the
  rejected input value.

#### Agreement + provenance lint (C-04, C-09)

- **FR-016**: The binding MUST expose a single check operation, runnable as a CI/lint pass over a registry
  of prompts, that verifies for each prompt+variant that the template's referenced variables are a subset
  of that prompt's declared variables, and reports any variable referenced but not declared.
- **FR-017**: The check MUST obtain referenced variables and the provenance view from the Rust core's
  analysis (the binding does not re-derive them). The authoritative "declared variables" set is the
  prompt **definition's `variables` block**, not the Zod Vars schema — the check is pure data and MUST
  NOT require introspecting the caller's Zod types.
- **FR-018**: The check MUST include a provenance lint: a prompt that declares one or more `untrusted` or
  `external` variables (via the kernel's provenance view) but configures **no guard** for them (the
  `meta`/`metadata` `guard`-key convention, per spec 003) MUST be reported, naming the prompt and the
  uncovered field(s).
- **FR-019**: The check MUST be pure analysis — pass/fail — and MUST NOT mutate any prompt, definition, or
  input, render anything, or produce side effects.
- **FR-020**: Check findings MUST be actionable: each identifies the prompt, the variant where
  applicable, and the offending variable/field; the reserved-`default`-name and un-analyzable-template
  cases are reported as distinct finding kinds (spec-003/004 parity).

#### Packaging & boundary (C-02, C-03, Principle II/III)

- **FR-021**: The binding MUST be packaged as a napi-rs native Node addon, built via `@napi-rs/cli`
  (`napi build --platform`), importable from the `prompting-press` npm package, with the platform-specific
  native binaries distributed as per-platform `optionalDependencies` (clarified Q5). The package MUST
  ship **ESM-only** (clarified Q8), consistent with the scaffold's existing `napi build --esm` flag — no
  CommonJS entry point; consumers are Node 20+ / bundler environments (matching the scaffold's `engines.node: ">=20"`). (Actual publish is spec 007; this
  spec produces a locally buildable, installable, importable package.)
- **FR-022**: `napi`/`napi-derive` (and any FFI toolkit dependency) MUST appear **only** in
  `prompting-press-node`; the kernel and the Rust consumer MUST stay FFI-free. The `ci:check-ffi` gate
  ALREADY asserts `napi` (alongside `pyo3`) is absent from `prompting-press-core` and `prompting-press`
  (`FFI_CRATES=("pyo3" "napi")`, shipped spec 001); this spec MUST keep that gate green (it does not need
  to add the assertion).
- **FR-023**: The binding MUST NOT perform I/O (no file/network/database/environment access), make model
  calls, assemble provider request bodies, parse model output, or count tokens. The `outputModel`
  reference is carried as metadata only. **No token-count hook or token counter ships** (consistent with
  spec-003 refinement F4; the token surface is deferred — see the roadmap-line reconciliation in
  Assumptions).
- **FR-024**: The generated TypeScript prompt-definition shape MUST stay in sync with the JSON Schema via
  the existing codegen freshness gate (`schemas:codegen-check`); a schema change not regenerated into the
  TS shape is a build failure (Principle VII).
- **FR-025**: The Node package's dependencies MUST be covered by a CI vulnerability/advisory gate. The
  repo's existing advisory gates (`ci:check-advisories` cargo-deny for Rust; `ci:check-advisories-py`
  pip-audit for Python) give no CVE coverage for the npm deps (Zod, `@napi-rs/cli`,
  `json-schema-to-typescript`); this spec adds a Node advisory gate (e.g. `pnpm audit`/`osv-scanner` over
  the pnpm lockfile) so a known-vulnerable npm dependency fails CI, mirroring the Rust/Python gates.

### Key Entities *(include if feature involves data)*

- **Typed Vars schema**: an application-defined Zod schema carrying the prompt's inputs and their
  refinements; validated as a whole, then marshaled into the kernel's value type. Authored by the
  application, not the library; passed to `render` alongside the prompt name. Static-only typed data
  (no runtime validator) is also accepted.
- **Prompt definition (input)**: the spec-001 shape — name, role, body, declared `variables` (each with a
  provenance tag — the authoritative declared set for the agreement check), variants, opaque
  `meta`/`metadata`, `outputModel` reference. The TS shape is code-generated from the JSON Schema;
  consumed, not redefined.
- **Registry**: a library-owned map of prompt name → loaded prompt definition. The application loads
  prompts into it; `render(name, …)` resolves against it and `check(registry)` lints over it.
- **Render result**: rendered text plus provenance (name, variant, `templateHash`, `renderHash`, optional
  guard), surfaced from the Rust core as library-owned JS data.
- **Normalized error**: the common `{field, code, message}` rows, thrown as a JS `Error` subclass; the
  single error contract shared across all bindings.
- **Message (composition output)**: an ordered `{role, text}` entry; a composition resolves to an array
  of these.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A TypeScript application can define Zod Vars with custom refinements, load a prompt, and
  render — reaching no Rust kernel type and no native `ZodError` on the public API — in a single idiomatic
  call path (validate-then-render).
- **SC-002**: Invalid inputs are rejected before render in 100% of cases, with a structured error naming
  every offending field; a render is never performed on invalid input.
- **SC-003**: The same logical prompt loaded from YAML, from JSON, and from a constructed object yields
  identical internal representations and identical render output for identical inputs (100% parity across
  the three input forms).
- **SC-004**: The agreement check detects an undeclared-variable reference in 100% of seeded cases and
  reports the prompt/variant/variable; it passes clean prompts; it mutates nothing and renders nothing.
- **SC-005**: The provenance lint flags, in 100% of seeded cases, a prompt that declares an
  untrusted/external variable with no guard configured (naming the prompt + field).
- **SC-006**: No native error type (`ZodError`, Rust kernel error) appears on the binding's public API;
  every error surfaces as the common `{field, code, message}` shape via a JS `Error` subclass, with
  `code` from the shared closed vocabulary.
- **SC-007**: `prompting-press-node` is the only crate with a `napi` dependency (the extended
  `ci:check-ffi` gate passes for both `pyo3` and `napi`), and the binding contains no
  rendering/agreement/variant/hashing logic of its own (it delegates across the FFI boundary to the Rust
  core).
- **SC-008**: A multi-message composition of N (prompt, vars) entries resolves to exactly N ordered
  `{role, text}` messages in input order, each rendered with its own validated vars.
- **SC-009**: `napi build` produces an installable native package; in a fresh Node environment a clean
  install + ESM `import` of `prompting-press` succeeds and the render/check/compose paths execute
  against the compiled core. The provenance hashes from a TS render are byte-identical to the Python
  binding's and the Rust consumer's for the same logical prompt + inputs.
- **SC-010**: The generated TypeScript prompt-definition shape is byte-identical to a fresh regeneration
  from the JSON Schema (the codegen freshness gate passes); no token-counting surface exists anywhere in
  the package.
- **SC-011**: A CI advisory gate scans the Node dependencies (the pnpm lockfile) for known CVEs and fails
  on a vulnerable dependency — the npm deps are covered, mirroring the Rust and Python advisory gates.

## Assumptions

- **Zod v4 is the TS validation system** (resolved Q7; Principle VI). The exact current Zod v4,
  napi-rs / `@napi-rs/cli`, and `json-schema-to-typescript` versions/APIs are confirmed at planning time
  (verify-at-spec-time discipline — subagent-reported versions have been wrong before; check npm /
  crates.io directly). The `ZodError`→`{field,code,message}` mapper reads Zod v4's issue/error shape
  only. Zod is **not** yet a `packages/typescript` dependency (current `dependencies: {}`); this spec
  adds it pinned exact (`4.4.3`), not caret-ranged — the floating-version gate scans the whole
  `package.json` (analyze F4), so a caret range would fail it.
- **The kernel (spec 002) and Rust consumer (spec 003) are the dependencies** and provide
  render/get_source/required_roots/provenance_view + the result/error/report types; this binding wraps
  them across napi and normalizes to the shared error contract. The `prompting-press-node` crate already
  declares path deps on both (verified) and `napi`/`napi-derive` 3.x.
- **The TS codegen pipeline exists** (verified): `packages/typescript/scripts/codegen.mjs` +
  `json-schema-to-typescript@15.0.4` generate `src/generated/prompt-definition.ts` from
  `schemas/jsonschema/prompt-definition.schema.json`; the generated type is present. The Zod Vars schemas
  the *caller* writes are separate, application-authored, and not codegen'd.
- **The spec-004 Python binding is the mirror reference** (implemented + merged): this spec reproduces its
  four user stories, FR set, and SC set in TS idiom, deviating only where the ecosystem differs (Zod vs
  Pydantic, JS `Error` vs Python exception, napi platform packages vs abi3 wheel, `.refine()` vs field
  validator).
- **Validation ownership (resolved Q1)**: the binding **owns validation at the render boundary** — accepts
  a Zod schema + data (or re-validates already-typed data via `safeParse`), catches `ZodError`, and
  normalizes it (FR-014). Keeps C-06 intact (no native `ZodError` escapes).
- **Error shape (resolved Q2)**: a small `Error`-subclass **hierarchy** under one base
  `PromptingPressError`, mapping 1:1 onto the Rust `ConsumerError` variants. (Exact subtype names
  finalized at plan/design time.)
- **Loader locus (resolved Q3)**: **reuse the Rust consumer's dual-input loader** (marshal YAML/JSON text
  across FFI; parse with the consumer's serde path). Makes parity/accept-reject structural (Principle I),
  avoids a JS YAML dependency, keeps "one loader, one representation" literally singular.
- **napi version pin (resolved at plan time)**: the `prompting-press-node` crate declares `napi = "3"`
  / `napi-derive = "3"` — a *floating* major-range the `ci:check-floating-versions` gate flags. Plan
  RESOLVED: pin exact `3.9.4` (crates.io-verified), consistent with the repo's no-floating-versions
  discipline (T001). The npm deps MUST likewise be pinned exact — the floating-version gate scans the
  whole `package.json` (analyze F4), not just a subset.
- **Marshaling edge values**: the napi boundary's JS↔Rust value mapping (`number` vs `bigint`, `null` vs
  `undefined`, nested objects, dates) is a first-class concern because spec 006 will test it across
  bindings; this spec pins correctness only for its own render/check/compose paths. The `null`/`undefined`
  treatment is fixed (resolved Q6, FR-003a): **`undefined`/absent → field-not-present; `null` → JSON
  `null`**, matched to the Python binding's `None`/absent handling for cross-binding consistency.
- **Three-sets invariant** (spec-003 critique E1 / spec-004): the caller's Zod Vars field names must agree
  with the prompt's declared `variables` block. `check()` validates template-roots ⊆ `variables` (a CI
  lint), and Zod validates the value's *contents* — but the *Vars↔`variables`* field-name agreement is the
  caller's responsibility. A mismatch is not silent: it surfaces as a loud `undefined_variable`-class
  error from the kernel, documented and pinned by a test.
- **Token surface — roadmap reconciliation**: the GOVERNANCE ledger `.specify/memory/roadmap.md` 005 entry
  no longer lists a token hook (struck during the spec-004 cycle, T027, consistent with spec-003 F4). The
  older DESIGN doc `docs/research/roadmap.md` (lines 76 + 85) still listed "token hook" for both bindings;
  analyze F1/F3 caught this and it is **struck in this spec's cycle** (both lines amended with the F4
  rationale). This spec ships **no** token surface (FR-023, SC-010).
- **No cross-language conformance work here**: the FFI conformance corpus (broad marshaling fidelity +
  schema round-trip across all bindings) is spec 006; render-byte-parity is structural (Principle I). This
  spec verifies marshaling only for its own render/check/compose paths — but it is the binding whose
  existence *enables* spec 006.
- **No publish here**: npm publish + release tooling is spec 007. This spec produces a locally
  buildable/installable/importable package.

## Dependencies

- **Spec 002 (Engine kernel) — satisfied/merged**: the kernel API this binding marshals to (`render`,
  `get_source`, `required_roots`, `provenance_view`), the result/`GuardConfig` types, the closed
  `KernelError` enum, and the re-exported `PromptDefinition` shape.
- **Spec 003 (Rust consumer) — satisfied/merged**: the reference surface this binding reproduces in TS
  idiom, and (per Q3) the dual-input loader + registry + `check()` + error-`code` vocabulary the binding
  reuses across FFI.
- **Spec 004 (Python binding) — satisfied/merged**: the mirror reference — the same four-user-story / FR /
  SC structure, the error-`code` vocabulary, the SEC-004 scrub pattern, and the "validate at render +
  reuse the Rust loader" decisions, all reproduced here in TS idiom.
- **Spec 001 (Foundations) — satisfied/merged**: the prompt-definition JSON Schema, the TS codegen
  pipeline, the `prompting-press-node` crate + `packages/typescript` package scaffolds, the `ci:check-ffi`
  and codegen-freshness gates.

## Governance Alignment

Governed by constitution Principles **I** (shared core — no engine logic in the binding; render parity
structural), **II** (FFI isolation — `napi` only in `prompting-press-node`; kernel + Rust consumer stay
FFI-free; `ci:check-ffi` extended to `napi`), **III** (minimal boundary — no I/O, no model calls, no token
counting; token surface deferred per F4), **VI** (per-language idiom — Zod, `fromMessages` not `.chain()`,
errors as JS `Error` subclasses normalized to the common shape), and **VII** (JSON Schema single source —
TS shape codegen'd, dual-input loader into the one shape), and by roadmap decisions **C-02, C-06, C-07**
(plus C-04/C-09 surfaced via the lint). No new pluggable interface; no boundary-expanding capability
added. This is the **second binding** that, per the roadmap, makes the FFI marshaling boundary real for
the spec-006 conformance corpus.
