# Feature Specification: Rust consumer crate (`prompting-press`)

**Feature Branch**: `003-rust-consumer`

**Created**: 2026-06-26

**Status**: Clarified

**Clarified**: 2026-06-26 — 4 design decisions resolved (declared-vars authority = the definition's `variables` block; Registry = owned name→definition map; caller passes prompt+vars at render; validated struct is serialized into the kernel value type). Integrated into FRs, Key Entities, and Assumptions.

**Refined**: 2026-06-26 (analyze gate) — F1: reframed the provenance lint to "declares untrusted/external + no guard configured" (`UntrustedWithoutGuard`; the kernel has no in-template guard-position concept). F4: DROPPED the token-count hook (FR-021/022, SC-009) — deferred to a later spec. F3: render resolves by registry name only (no "prompt handle"). F5: guard-expansion is kernel-owned; the consumer only plumbs it through. F7: empty composition → `Ok(vec![])`, empty-registry check → empty report. Net: 8 SCs (was 9), FR-021/022 dropped, 26 tasks (was 27).

**Input**: User description: "Rust consumer crate (`prompting-press`): the first full consumer layer over the spec-002 engine kernel — the public, idiomatic Rust API. garde typed-Vars facade, dual-input loader, check(registry) agreement+provenance lint, ergonomic render()/get_source() + composition, error normalization, token-count hook. No FFI, no logic duplication."

## Overview

`prompting-press` is the public Rust API for the library — what an application gets from
`cargo add prompting-press`. It is the **first full consumer layer** over the spec-002 engine kernel
(`prompting-press-core`), and it deliberately proves the kernel/consumer split *before* any
language-binding (Python/TypeScript) exists: everything language-native lives here; everything shared
lives in the kernel.

The kernel turns *already-validated values + a prompt definition* into *rendered text + provenance*,
and reports a template's required variables — but it is validation-blind and idiom-free by design.
This crate supplies exactly what the kernel omits: a **typed-Vars facade with custom validators**, a
**dual-input loader** (YAML/JSON or a constructed object), the **agreement + provenance lint** as a
usable check, **idiomatic render/compose** ergonomics, and **normalized errors**. (A token-count hook
was considered and dropped — deferred to a later spec; see Clarifications/analyze F4.) It adds no
rendering, agreement, variant-resolution, or hashing *logic* — those live once
in the kernel and are wrapped here (constitution Principle I).

This is the spec that turns the library's headline guarantee — *a template referencing an undeclared
variable is a caught error, not a silent empty render* — into something a Rust application actually
calls.

## Clarifications

### Session 2026-06-26

- Q: For the agreement check, what is the authoritative set of "declared" variables? → A: **The prompt
  definition's own `variables` block** (the spec-001 shape the kernel carries). `check()` compares
  template-referenced roots against `definition.variables` — pure data, runnable in CI without
  introspecting the caller's Rust types. The garde Vars struct is the runtime *validator*, not the
  lint's authority.
- Q: What is the `Registry`? → A: **A library-owned map, prompt name → `PromptDefinition`.** The
  application loads prompts into it (from YAML/JSON/objects); `render(name, …)` resolves against it and
  `check(registry)` lints over the whole collection.
- Q: How does a render call know which garde struct validates which prompt? → A: **The caller passes
  both at render** — `render(prompt, vars)`: the caller supplies the prompt (by name/handle) and the
  typed Vars value together; the crate validates the vars, then renders. No type-registration
  machinery; Vars↔prompt correctness is a caller responsibility (and a `check()`-time concern).
- Q: What does the crate accept as render input values, bridging garde's typed struct to the kernel's
  value map? → A: **Serialize the validated struct.** The caller's Vars struct derives `Serialize`;
  after garde validation passes, the crate serializes it into the kernel's value type. One typed
  struct in → validated → serialized to the kernel (the standard serde+garde pairing).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Validate typed inputs and render through one idiomatic call (Priority: P1)

A Rust application defines its prompt inputs as a typed struct with field validators, loads a prompt,
and renders it. Invalid inputs are rejected with a structured error *before* any templating happens;
valid inputs produce rendered text plus provenance. The application never touches the kernel directly
and never sees a native validator or kernel error type.

**Why this priority**: This is the crate's reason to exist and the minimum viable consumer — typed
input + validation + render in one ergonomic call, with the kernel wrapped invisibly. Without it there
is no consumer layer. It exercises the kernel/consumer split end to end.

**Independent Test**: Define a typed Vars struct with a custom validator, load a prompt, and call
render with (a) valid values → rendered text + provenance, and (b) values that fail validation → a
structured error listing the offending field(s), with no render attempted. Fully testable with no
other story present.

**Acceptance Scenarios**:

1. **Given** a typed Vars value that satisfies all field validators and a loaded prompt, **When** the
   application renders, **Then** validation runs once, succeeds, the kernel renders, and the result
   carries the rendered text plus provenance (name, variant, template/render hashes).
2. **Given** a typed Vars value that violates a field validator (e.g. an out-of-range number),
   **When** the application renders, **Then** a structured validation error is returned naming the
   offending field, and no render is performed.
3. **Given** multiple fields fail validation at once, **When** the application renders, **Then** the
   error reports all failing fields (validation is evaluated as a whole, once).
4. **Given** a successful render, **When** the application inspects the result, **Then** it sees only
   normalized, library-owned types — never a native validator report or a kernel error type.
5. **Given** the same prompt + same valid values rendered twice, **When** both complete, **Then** the
   outputs and provenance hashes are identical (the kernel's determinism, surfaced unchanged).

---

### User Story 2 - Push prompt data as YAML, JSON, or a constructed object (Priority: P2)

A team keeps canonical prompts as in-repo YAML or JSON files; another part of the application builds a
prompt definition programmatically. Both reach the library through one loader that normalizes them
into a single internal representation, so downstream behavior is identical regardless of input form.

**Why this priority**: Repo-canonical prompts are a core project goal, and "give data OR give an
object, same behavior" is the promise of the single-source-of-truth shape. It is P2 because it builds
on the render path of US1.

**Independent Test**: Load the *same* logical prompt three ways — from a YAML document, from an
equivalent JSON document, and from a constructed object — and confirm all three produce an identical
internal definition and identical render output for identical inputs.

**Acceptance Scenarios**:

1. **Given** a prompt definition as a YAML document, **When** loaded, **Then** it normalizes to the
   library's single prompt-definition representation.
2. **Given** the equivalent prompt definition as a JSON document, **When** loaded, **Then** it
   normalizes to a representation identical to the YAML-loaded one.
3. **Given** a prompt definition constructed programmatically as an object, **When** used, **Then** it
   is accepted on equal footing with loaded data (no second-class path).
4. **Given** malformed input (invalid YAML/JSON, or data that violates the prompt-definition shape),
   **When** loaded, **Then** a structured error is returned describing what is wrong; nothing is
   partially loaded.

---

### User Story 3 - Run the agreement + provenance lint as a CI check (Priority: P2)

A team wants the headline guarantee enforced in CI: every prompt's template must reference only
declared variables, and a prompt declaring untrusted/external inputs must have a guard configured for them. They
point a single check at their registry of prompts; it passes or fails with actionable detail and
mutates nothing.

**Why this priority**: This is the BAML-equivalent static guarantee that distinguishes the library —
catching the undeclared-variable bug before runtime. It is the differentiator made usable. P2 because
it consumes the same validated-definition machinery US1/US2 establish.

**Independent Test**: Run the check over a registry containing (a) a well-formed prompt → pass, (b) a
prompt whose template references an undeclared variable → fail naming the variable and prompt, and (c)
a prompt declaring an untrusted/external field with no guard configured → fail naming the
field. Confirm the check changes no files and renders nothing.

**Acceptance Scenarios**:

1. **Given** a registry of prompts whose templates reference only declared variables, **When** the
   check runs, **Then** it passes.
2. **Given** a prompt whose template references a variable not declared for that prompt, **When** the
   check runs, **Then** it fails, identifying the prompt, the variant, and the undeclared variable.
3. **Given** a prompt that declares an `untrusted` or `external` variable with no guard configuration
   covering it, **When** the check runs, **Then** it fails, identifying the prompt and the uncovered
   field (provenance lint).
4. **Given** any registry, **When** the check runs, **Then** it performs pure analysis: it does not
   render, does not mutate any prompt, definition, or input, and produces no side effects.
5. **Given** a prompt with multiple variants, **When** the check runs, **Then** each variant's
   template is analyzed against the declared variables.

---

### User Story 4 - Compose a multi-message prompt (Priority: P3)

An application assembles a multi-message prompt (e.g. a few-shot sequence) from several prompt + vars
pairs in an explicit order, producing an ordered list of role-tagged messages it can hand to its model
call layer.

**Why this priority**: Composition is a real need (few-shot, system+user sequences) but additive on
top of single render. P3 because US1 delivers value without it.

**Independent Test**: Build an ordered sequence of (prompt, vars) pairs, resolve it, and confirm the
output is an ordered list of `{role, text}` messages matching the input order, each rendered with its
own validated vars.

**Acceptance Scenarios**:

1. **Given** an ordered sequence of (prompt, vars) entries, **When** resolved, **Then** the output is
   an ordered list of `{role, text}` messages in the same order, each rendered with its own vars.
2. **Given** the composition is built incrementally (append entries), **When** resolved, **Then** the
   order reflects append order.
3. **Given** one entry's vars fail validation, **When** resolved, **Then** a structured error
   identifies which entry/field failed; the partial result is not returned as if successful.
4. **Given** a fragment rendered with its own vars, **When** its output is passed into a parent prompt
   as a declared variable, **Then** composition-by-value works without any template-include mechanism.

---

### Edge Cases

- **Token counting**: out of scope for spec 003 (the hook was dropped — F4). The crate ships no token
  counter and exposes no token-count seam; token counting/budgeting is deferred to a later spec.
- **Output-model reference**: carried as metadata and echoed; never parsed or resolved by the crate.
- **Unknown variant / missing prompt at render**: surfaces as a structured error (wrapping the
  kernel's unknown-variant error / a registry lookup miss), never a panic.
- **Excluded template features / strict-undefined / render failure**: the kernel's loud errors are
  surfaced normalized, not swallowed.
- **Validation error detail leakage**: error normalization must not echo raw, potentially sensitive
  bound-value content into messages/logs (carried security concern from the kernel's error detail).
- **Empty registry / empty composition** (pinned — F7): an empty composition's `resolve()` returns
  `Ok(vec![])`; `check()` over an empty registry returns an empty `CheckReport` (pass). Never a panic.

## Requirements *(mandatory)*

### Functional Requirements

#### Typed Vars + validation (C-06)

- **FR-001**: The crate MUST let applications define typed input models in the native Rust validation
  system (garde) with custom field validators, rather than inventing a bespoke validation framework.
- **FR-002**: The crate MUST run validation **once, at render** (the whole input set validated
  together), before any templating occurs; if validation fails, no render is performed.
- **FR-003**: Validation MUST live in this consumer layer; the crate MUST pass only already-validated
  values to the kernel (the kernel stays validation-blind).
- **FR-003a**: After validation passes, the crate MUST bridge the caller's typed Vars to the kernel's
  value type by **serializing the validated struct** (the Vars model is serializable); the caller MUST
  NOT have to hand-build a value map. The standard pairing is serde-serialization of the same struct
  garde validated.
- **FR-004**: Native validator outputs (the garde report) MUST NOT be exposed on the crate's public
  API; they are normalized first (FR-014).

#### Dual-input loader (C-07)

- **FR-005**: The crate MUST accept prompt data pushed as a YAML document, as a JSON document, or as a
  programmatically constructed definition object, and normalize all three into one internal
  prompt-definition representation.
- **FR-006**: A prompt definition loaded from YAML and the equivalent loaded from JSON MUST produce
  identical internal representations and identical downstream behavior.
- **FR-007**: Malformed input (invalid YAML/JSON, or data that violates the prompt-definition shape)
  MUST produce a structured error; the crate MUST NOT partially load or silently coerce.
- **FR-008**: The crate MUST consume the spec-001 prompt-definition shape (owned/re-exported by the
  kernel) as its single definition representation and MUST NOT define a parallel shape.
- **FR-008a**: The crate MUST provide a **registry** — a library-owned collection mapping a prompt
  name to its loaded `PromptDefinition` — that the application loads prompts into (via the dual-input
  loader or constructed objects). `render(name, …)` resolves the prompt against this registry, and the
  agreement/provenance check (FR-016) runs over it. A name absent from the registry MUST surface as a
  structured error, not a panic.

#### Render, get-source & composition (C-01, C-06)

- **FR-009**: The crate MUST expose an idiomatic `render`-style operation that takes a prompt **name
  resolved against the registry** (the sole resolution path — F3; no separate "prompt handle")
  **and** the caller's typed Vars value together, validates the vars, then delegates rendering to the
  kernel, and returns the rendered text plus provenance (name, variant, `template_hash`, `render_hash`,
  and the optional guard field). The crate MUST NOT require pre-registering a Vars type per prompt; the
  caller supplies prompt + vars at the call site. Guard *expansion* behavior is owned and tested by the
  kernel (spec 002); this crate only plumbs `GuardConfig` through to the kernel and surfaces the
  resulting `guard` field — it does not re-test kernel guard logic (F5).
- **FR-010**: The crate MUST expose a `get_source`-style operation returning a prompt variant's
  unrendered template source, delegating to the kernel.
- **FR-011**: The crate MUST NOT reimplement rendering, agreement analysis, variant resolution, or
  hashing; these are kernel calls (no logic duplication).
- **FR-012**: The crate MUST support composing a multi-message prompt as an **explicit ordered
  sequence** of (prompt-ref, vars) entries that resolves to an ordered list of `{role, text}`
  messages, with native append-style construction.
- **FR-013**: The crate MUST NOT offer a fluent `.chain()` composition API.

#### Error normalization (C-06)

- **FR-014**: The crate MUST normalize both validation failures (garde report) and kernel errors into
  one common structured error shape — a list of `{field, code, message}` entries — at its public
  boundary; native error types MUST NOT leak.
- **FR-015**: Error normalization MUST NOT echo raw, potentially sensitive bound-value content into
  error messages or logs.

#### Agreement + provenance lint (C-04, C-09)

- **FR-016**: The crate MUST expose a single check operation, runnable as a CI/lint pass over a
  registry of prompts, that verifies for each prompt+variant that the template's referenced variables
  are a subset of that prompt's declared variables, and reports any variable that is referenced but
  not declared.
- **FR-017**: The check MUST obtain referenced variables from the kernel's analysis (the crate does
  not re-derive them) and MUST own the subset comparison. The authoritative "declared variables" set
  is the prompt **definition's `variables` block** (the spec-001 shape carried by the kernel) — the
  check is pure data and MUST NOT require introspecting the caller's typed Vars (garde) struct.
- **FR-018**: The check MUST include a provenance lint: a prompt that declares one or more `untrusted`
  or `external` variables (surfaced via the kernel's provenance view) but provides **no guard
  configuration covering them** MUST be reported. (Reframed — the spec-002 kernel has no in-template
  "guard position" concept; the implementable, useful C-09 lint is "you declared untrusted inputs and
  set up no guard for them.") The finding names the prompt and the uncovered field(s).
- **FR-019**: The check MUST be pure analysis — pass/fail — and MUST NOT mutate any prompt,
  definition, or input, render anything, or produce side effects.
- **FR-020**: Check failures MUST be actionable: each finding identifies the prompt, the variant where
  applicable, and the offending variable/field.

#### Token-count hook (C-03) — ~~DROPPED~~ (deferred to a later spec; analyze F4, 2026-06-26)

> The token-count hook is **removed from spec 003**. It was the only token-related surface, and per
> Principle III token counting/budgeting is deferred; exposing a bare seam (that the caller could
> trivially write) added little, and invoking it would have dragged a `model` param + a wrapper result
> type + the deferred budgeting feature into scope. Deferred to a future spec (roadmap note added).

- ~~**FR-021**: The crate MUST expose a pluggable `count_tokens(text, model) -> count` hook seam…~~ — **dropped (F4)**.
- ~~**FR-022**: Token counting MUST be available only when a caller supplies the hook…~~ — **dropped (F4)**.

#### Boundary & isolation (C-01, C-02, C-03)

- **FR-023**: The crate MUST remain FFI-free — no `pyo3`, `napi`, or other FFI binding dependency
  (the existing CI FFI-isolation gate MUST stay green).
- **FR-024**: The crate MUST NOT perform I/O (no file/network/database/environment access), make model
  calls, assemble provider request bodies, or parse model output; the output-model reference is
  carried as metadata only.

### Key Entities *(include if feature involves data)*

- **Typed Vars model**: an application-defined Rust struct carrying the prompt's inputs and their
  validators; serializable; validated as a whole at render, then serialized into the kernel's value
  type. Authored by the application, not the library; passed to `render` alongside the prompt.
- **Prompt definition (input)**: the spec-001 shape (owned by the kernel, re-exported here) — name,
  role, body, **declared `variables` (each with a provenance tag — the authoritative declared set for
  the agreement check)**, variants. Consumed, not redefined.
- **Registry**: a library-owned map of prompt name → loaded `PromptDefinition`. The application loads
  prompts into it; `render(name, …)` resolves against it and `check(registry)` lints over it.
- **Render result**: rendered text plus provenance (name, variant, `template_hash`, `render_hash`,
  optional guard), surfaced from the kernel as library-owned data.
- **Normalized error**: the common `{field, code, message}` structured shape that both validation
  failures and kernel errors are mapped into.
- **Message (composition output)**: an ordered `{role, text}` entry; a composition resolves to a list
  of these.
- ~~**Token-count hook**~~ — **dropped (F4)**; token counting is deferred to a later spec.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An application can define typed Vars with custom validators, load a prompt, and render —
  reaching no kernel type and no native validator type on the public API — in a single idiomatic call
  path (validate-then-render).
- **SC-002**: Invalid inputs are rejected before render in 100% of cases, with a structured error
  naming every offending field; a render is never performed on invalid input.
- **SC-003**: The same logical prompt loaded from YAML, from JSON, and from a constructed object
  yields identical internal representations and identical render output for identical inputs (100%
  parity across the three input forms).
- **SC-004**: The agreement check detects an undeclared-variable reference in 100% of seeded cases and
  reports the prompt/variant/variable; it passes clean prompts; it mutates nothing and renders nothing.
- **SC-005**: The provenance lint flags, in 100% of seeded cases, a prompt that declares an
  untrusted/external variable with no guard configuration covering it (naming the prompt + field).
- **SC-006**: No native error type (garde report) and no kernel error type appears on the crate's
  public API; every error surfaces as the common `{field, code, message}` shape.
- **SC-007**: The crate has zero `pyo3`/`napi`/FFI dependencies (the CI FFI-isolation gate passes) and
  contains no rendering/agreement/variant/hashing logic of its own (it delegates to the kernel).
- **SC-008**: A multi-message composition of N (prompt, vars) entries resolves to exactly N ordered
  `{role, text}` messages in input order, each rendered with its own validated vars.
- ~~**SC-009**: With no token-count hook supplied…~~ — **dropped (F4, token hook deferred to a later spec)**.

## Assumptions

- **garde is the Rust validation system** (per the resolved design; roadmap names garde 0.23). The
  exact current version and validator/report API are confirmed at planning time (verify-at-spec-time).
- **A maintained, pure-Rust YAML parser** backs the YAML input path; the specific crate is chosen at
  planning time and must keep the FFI gate green.
- **The kernel (spec 002) is the dependency** and provides render/get_source/required_roots/
  provenance_view + the result/error types; this crate wraps them and normalizes the closed
  `KernelError` enum.
- **"Declared variables" authority** (resolved, see Clarifications): the agreement check compares
  template-referenced roots against the prompt definition's **`variables` block** (the spec-001 shape),
  not the garde struct — keeping `check()` pure-data and CI-portable.
- **Registry shape** (resolved, see Clarifications): a library-owned map of prompt name →
  `PromptDefinition`, serving both render resolution and the lint.
- **Vars binding & bridge** (resolved, see Clarifications): the caller passes the prompt + typed Vars
  together at `render` (no per-prompt type registration); after garde validation the validated struct
  is serialized into the kernel's value type.
- **Three-sets invariant** (critique E1): the caller's garde Vars struct field names must agree with
  the prompt's declared `variables` block. `check()` validates the template-roots ⊆ `variables`
  agreement (a CI lint), and garde validates the struct's *values* — but the *struct↔`variables`*
  field-name agreement is the caller's responsibility. A mismatch (e.g. a misnamed struct field) is not
  silent: it surfaces as a loud strict-undefined render error from the kernel (FR-001a, spec 002),
  normalized to an `undefined_variable`-class `ConsumerError`. This is documented (rustdoc) and pinned
  by a test, not enforced by an extra check (closing it in-library would require the per-prompt type
  registration that Clarify Q3 deliberately rejected for v1).
- **No cross-language work**: the Python/TypeScript bindings, the FFI conformance corpus, and release
  packaging are later specs (004/005/007), out of scope here.

## Dependencies

- **Spec 002 (Engine kernel) — satisfied/merged**: provides the kernel API this crate wraps
  (`render`, `get_source`, `required_roots`, `provenance_view`), the result/`GuardConfig` types, the
  closed `KernelError` enum, and the re-exported `PromptDefinition` shape. Also the CI FFI-isolation +
  codegen-freshness gates this crate must keep green.
- **Spec 001 (Foundations) — satisfied/merged**: the prompt-definition JSON Schema + generated shape.

## Governance Alignment

Governed by constitution Principles **I** (shared core — no logic duplication), **II** (FFI isolation
— the consumer is FFI-free), **III** (minimal boundary — no I/O, no output parsing, no token counting;
token counting is deferred — F4), **VI** (per-language idiom — garde, `Vec`+`append_*` not `.chain()`,
normalized errors), and **VII** (JSON Schema single source — dual-input loader into the one shape), and
by roadmap decisions **C-03, C-06, C-07** (plus C-04/C-09 surfaced via the lint). No new pluggable
interface (C-08); no boundary-expanding capability added.
