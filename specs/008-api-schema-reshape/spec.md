# Feature Specification: Pre-publish API & schema reshape

**Feature Branch**: `008-api-schema-reshape`

**Created**: 2026-06-28

**Status**: Draft

**Input**: User description: "008 — Pre-publish API & schema reshape. The LAST pre-publish change to the public contract (blocks specs 007 and 010). One coordinated change: the per-variable `provenance` → `origin` schema rename, the schema-fixture move, and the prompt-as-object API redesign across all three bindings."

## Overview

This is the **last opportunity to change the public contract before the library is published** to crates.io,
PyPI, and npm (spec 007). After publish, every shape below becomes a breaking change governed by semver. The
work bundles three changes that are really **one decision about the library's core object model** (research
note: `docs/research/registry-value-and-object-model.md`, §8 pattern sweep + §9 resolved object model):

1. A per-variable **schema field rename** (`provenance` → `origin`).
2. A **schema-fixture directory move** (into a `tests/` subtree).
3. A **prompt-as-object API redesign** — a first-class immutable `Prompt` type across all three bindings,
   replacing the registry-keyed free-function surface.

The **rendering engine is not touched**. Rendering, agreement analysis, variant resolution, and hashing
behavior are unchanged (constitution Principle I); this reshapes the *public surface* and the *schema*, not the
engine. Cross-language output equality remains a structural property of the shared core, so this work does not
add or re-verify render-parity behavior — only the surface that wraps it.

The "users" of this contract are the **developers consuming the Rust, Python, and TypeScript packages**. The
success of this feature is measured by the shape, ergonomics, and safety of the surface they touch — and by the
existing CI + conformance gates staying green throughout.

## Clarifications

### Session 2026-06-28

- Q: Switch the TypeScript shape from a generated `interface` to a generated Zod schema (`json-schema-to-zod`)?
  → A: **Yes** — so the TS validating constructor has a runtime enforcer, at parity with Python (Pydantic) and
  Rust (struct + checks).
- Q: TypeScript construction failure mode, given `new` cannot return a result? → A: **`new Prompt({…})`
  throws** a structured error (carrying `[{field, code, message}]`); mirrors Python's Pydantic raise. (Native
  idiom, Principle VI.)
- Q: Ship the `validation_required` schema field in 008 or defer? → A: **Ship in 008**, reshaped: it is a
  **per-variable** optional boolean (a sibling of `type`/`origin` on each variable declaration), **orthogonal to
  `origin`** — it can mark any variable, not only `untrusted`/`external` ones.
- Q: Where does the native validator (Zod/Pydantic/garde) live? → A: **Bound at construction.** The `Prompt`
  holds its validator(s); at construction every variable marked `validation_required: true` must have a
  validator or construction fails. Validators are supplied as a **separate side input even when constructing
  from YAML/JSON/TOML text**, so the prompt *document* stays language-agnostic while the *validators* are native
  per language.
- Q: How does per-variable validator-coverage reconcile with Rust's garde (no runtime rule introspection)? → A:
  **Keep garde; Rust enforces at compile-time.** TS/Python introspect the supplied validator's per-field
  coverage and **throw** at construction if a `validation_required` variable is uncovered; in Rust,
  `validation_required` is **declarative metadata** and coverage is guaranteed **structurally at compile-time**
  (the developer wires garde rules onto the Vars type). This asymmetry requires a **MINOR amendment to
  constitution Principle VI** (see Dependencies).
- Q: Confirm the all-bindings reshape including a Rust `Prompt` wrapper? → A: **Yes** — all three bindings get a
  first-class `Prompt`; Rust wraps the generated struct with a generic `with` / `render::<V>`.
- Q: When a body cannot be statically analyzed (excluded `{% include/import/extends/macro/block %}` or a
  MiniJinja syntax error), what does construction do? → A: **Fail at construction.** Both categories are
  parse-time-detectable, so the validating constructor rejects them; a constructed `Prompt` is therefore always
  parseable and analyzable, and `check()`'s analysis-error finding is unreachable for a constructed `Prompt`.
- Q: Ship `Prompt.fromToml(text)` in 008 or defer? → A: **Ship in 008** — `fromYaml` / `fromJson` / `fromToml`
  as a complete text-factory set (one pinned TOML parser per binding).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Construct and use a prompt as a first-class object (Priority: P1)

A developer holds a prompt as an object and operates on it directly, instead of inserting it into a
name-keyed registry and calling free functions against the registry. They construct a `Prompt` from a native
shape object or from prompt text, then call render / inspect source / check **on the prompt itself**.

**Why this priority**: This is the spine of the reshape and the headline ergonomic change. Every other story
builds on the `Prompt` object existing. It is the v1 surface developers will first meet, and it is what makes
the library feel idiomatic ("the operation lives where its data lives") in all three ecosystems.

**Independent Test**: In each binding, construct a valid `Prompt` from a shape object, render it with valid
variables, and read back its source — all without any registry type existing. Delivers the complete
single-prompt workflow on its own.

**Acceptance Scenarios**:

1. **Given** a valid prompt shape object, **When** the developer constructs a `Prompt` from it via the primary
   constructor, **Then** an immutable `Prompt` is returned (or, on invalid input, a structured
   error/raise/throw — never a panic/crash).
2. **Given** a valid `Prompt`, **When** the developer renders it with valid variables, **Then** the rendered
   text + provenance (`template_hash`, `render_hash`, optional guard) are returned, byte-identical to what the
   pre-reshape registry-keyed render produced for the same inputs.
3. **Given** a valid `Prompt`, **When** the developer reads its source (default or a named variant), **Then**
   the unrendered template source is returned.
4. **Given** a `Prompt`, **When** the developer reads any field (name, role, body, variables, variants,
   metadata), **Then** a read-only accessor returns it and **no setter exists** to mutate it.

---

### User Story 2 - Derive a varied prompt without mutation (Priority: P1)

A developer needs a modified copy of a prompt — a new variant added, a changed body, even a different name —
without mutating the original. They call a single copy-with-overlay operation that validates the merged result
and returns a new immutable prompt.

**Why this priority**: With setters gone, this is the *only* way to vary a prompt, so it is as load-bearing as
construction itself. It replaces the previous `withVariant` and every per-field mutator with one primitive.

**Independent Test**: Take a valid `Prompt`, derive a new one that adds a variant (by spreading the current
variants into the overlay), and confirm the original is unchanged and the derived one carries the new variant.

**Acceptance Scenarios**:

1. **Given** a valid `Prompt`, **When** the developer applies an overlay of one or more top-level fields,
   **Then** a new immutable `Prompt` reflecting the merged definition is returned and the original is unchanged.
2. **Given** a valid `Prompt`, **When** the overlay supplies a top-level field that the prompt already has,
   **Then** the overlay **replaces that whole top-level field** (shallow replace, not a deep/recursive merge).
3. **Given** a valid `Prompt`, **When** the developer adds a single variant by spreading the current variants
   into the overlay (`with({ variants: { …current, terse: {body} } })`), **Then** the derived prompt has both
   the original variants and the new one.
4. **Given** a valid `Prompt`, **When** the overlay changes the `name`, **Then** a new, differently-named prompt
   is produced (a distinct identity).
5. **Given** a valid `Prompt`, **When** an overlay produces a merged definition that is invalid (e.g. a `body`
   referencing an undeclared variable), **Then** the operation returns a structured error and **no** prompt is
   produced — the merged whole is validated, not just the overlay.

---

### User Story 3 - Construction validates every decidable invariant up front (Priority: P1)

A developer expects an invalid prompt to fail at construction, not silently later. Construction enforces every
invariant that can be decided from the definition alone; only the genuinely un-analyzable residue is deferred to
an explicit `check()`.

**Why this priority**: This is the library's headline differentiator (the sound agreement check, Principle IV).
Moving it onto construction is what makes a constructed `Prompt` a trustworthy value object. Getting the
boundary between "fail at construction" and "report at `check()`" right is essential.

**Independent Test**: Attempt to construct a prompt whose analyzable body references an undeclared variable;
confirm construction fails with a structured error. Separately, construct a prompt whose body is un-analyzable
(parse failure or an excluded template feature) and confirm construction succeeds but `check()` reports the
analysis finding.

**Acceptance Scenarios**:

1. **Given** a prompt whose analyzable template references a variable not in its declared variables, **When**
   the developer constructs it, **Then** construction fails with a structured agreement error.
2. **Given** a prompt whose referenced variables are all declared, **When** the developer constructs it, **Then**
   construction succeeds.
3. **Given** a constructed `Prompt`, **When** the developer calls `check()`, **Then** it returns the
   per-prompt findings (agreement + origin) as pure analysis, mutating nothing.
4. **Given** a prompt whose body cannot be statically analyzed (a MiniJinja parse failure, or an excluded
   `{% include %}` / `{% import %}` / `{% extends %}` / macro / block feature), **When** the developer
   constructs it, **Then** construction **fails** with a structured error (both categories are parse-time
   detectable). Consequently a constructed `Prompt` is always parseable and analyzable, and `check()`'s
   analysis-error finding is unreachable for a constructed `Prompt`.

---

### User Story 4 - Compose a multi-message prompt from prompt objects (Priority: P2)

A developer assembles an ordered multi-message prompt (e.g. a system + user turn) by aggregating `Prompt`
**objects** (each with its own variables/variant), resolving to an ordered list of role-tagged messages.

**Why this priority**: Composition is a real capability of the library, but it sits on top of the `Prompt`
object existing (Story 1) and is used by fewer consumers than single-prompt render. It changes from
aggregating *names* (resolved against a registry) to aggregating *objects*.

**Independent Test**: Build a composition from two `Prompt` objects with their variables, resolve it, and
confirm the output is an ordered `[{role, text}, …]` matching each prompt's rendered body in order.

**Acceptance Scenarios**:

1. **Given** two valid `Prompt` objects with their variables, **When** the developer composes them in order and
   resolves the composition, **Then** an ordered list of `{role, text}` messages is returned, one per entry, in
   declaration order.
2. **Given** a composition entry whose variables fail validation, **When** the developer resolves it, **Then** a
   structured error identifying the failing entry is returned and no partial composition is emitted.
3. **Given** the composition API, **When** a developer inspects it, **Then** it aggregates `Prompt` objects
   directly and requires **no registry** to resolve references.

---

### User Story 5 - Migrate from the registry-keyed surface (Priority: P2)

A developer (and every in-repo example, test, and the conformance runners) updates from the old registry-keyed
free-function surface (`Registry`, `render(reg, name, …)`, `check(reg)`) to the prompt-as-object surface. The
`Registry` type and its name-keyed lookup are gone.

**Why this priority**: The reshape is only complete when every consumer of the old surface is migrated and the
old surface is removed; a half-migrated surface would ship two ways to do everything. It is P2 because it is
mechanical follow-through once Stories 1–4 define the new surface.

**Independent Test**: Grep the codebase (and the conformance runners) for the `Registry` type and the
registry-keyed free functions; confirm none remain and every example/test exercises the object surface instead.

**Acceptance Scenarios**:

1. **Given** the reshaped library, **When** a developer looks for the `Registry` type or a name-keyed prompt
   lookup, **Then** none exists in the public surface of any binding.
2. **Given** the in-repo examples, binding tests, and conformance runners, **When** the suite runs, **Then** all
   pass against the object surface and exercise no removed symbol.
3. **Given** the migrated library, **When** CI runs, **Then** the FFI-isolation gate, the codegen-freshness
   gate, the agreement/origin lint gate, and the conformance gate are all green.

---

### User Story 6 - Read and write prompts under the renamed `origin` field (Priority: P1)

A developer authors a prompt document (YAML/JSON) or shape object using the field name `origin` for a
variable's input-trust tag, where the previous contract used `provenance`. The accepted values
(`trusted` | `untrusted` | `external`) are unchanged.

**Why this priority**: The rename is a breaking change to the document/shape contract and must land before
publish. It cascades from the schema through codegen to every binding, fixture, and doc, so it is foundational
and P1.

**Independent Test**: Load/construct a prompt whose variable declares `origin: untrusted`; confirm it is
accepted, that the field is surfaced under the name `origin` in every binding's shape, and that a document still
using the old `provenance` key is rejected as an unknown field.

**Acceptance Scenarios**:

1. **Given** a prompt document/shape declaring a variable with `origin: untrusted`, **When** it is loaded or
   constructed, **Then** it is accepted and the variable's trust tag is readable under the name `origin`.
2. **Given** a prompt document still using the old `provenance` key, **When** it is loaded/constructed, **Then**
   it is rejected as an unknown field (the schema is closed; `additionalProperties: false`).
3. **Given** a prompt with `origin`-tagged untrusted/external variables, **When** the opt-in guard is requested
   at render, **Then** the guard text names exactly those fields — identical behavior to the pre-rename
   `provenance` tag (only the field name changed; the enum values and all guard/exposure behavior are unchanged).
4. **Given** the rename, **When** a developer inspects the render-result provenance (the `template_hash` /
   `render_hash` on the return value), **Then** that concept is **unchanged and not renamed** — only the
   per-variable input-trust tag was renamed.

---

### User Story 7 - Bind per-variable validators at construction (Priority: P1)

A developer marks certain variables `validation_required: true` and supplies native validators (a Zod schema /
Pydantic model / garde-derived Vars type). The `Prompt` binds the validator at construction and reuses it at
render; if a required variable has no validator, construction fails (Python/TypeScript) or fails to compile
(Rust). Validators are supplied as a side input even when the prompt is loaded from YAML/JSON/TOML text.

**Why this priority**: `validation_required` is a contract field that must ship pre-publish, and binding the
validator at construction is what makes it enforceable. It closes the static-render bypass while keeping prompt
*documents* language-agnostic. It is foundational to the value-validation half of the library's guarantee.

**Independent Test**: Construct a prompt with one `validation_required` variable, once with a covering validator
(succeeds) and once without (Python/TS raise/throw at construction; Rust fails to compile). Confirm a
YAML-loaded prompt accepts validators via the side input.

**Acceptance Scenarios**:

1. **Given** a prompt whose variable is marked `validation_required: true` and a validator covering it, **When**
   the developer constructs the `Prompt`, **Then** construction succeeds and the validator is bound for render.
2. **Given** a prompt whose variable is marked `validation_required: true` and **no** covering validator,
   **When** the developer constructs it in Python/TypeScript, **Then** construction raises/throws a structured
   error naming the uncovered variable; in Rust the equivalent guarantee holds at compile-time.
3. **Given** a prompt loaded from YAML/JSON/TOML text, **When** the developer supplies validators as the side
   input, **Then** the prompt document remains format-portable and the validators bind natively per language.
4. **Given** a `validation_required` variable that is **not** `untrusted`/`external`, **When** the prompt is
   constructed, **Then** the requirement is enforced — `validation_required` is orthogonal to `origin`.

---

### Edge Cases

- **Overlay that empties a collection**: `with({ variants: {} })` replaces the variants with an empty map
  (shallow replace makes this expressible; there is no "remove one variant" sentinel — the caller spreads the
  subset they want to keep).
- **Overlay that produces a duplicate/invalid name**: validated as part of the merged whole; a merged definition
  that violates a schema/agreement rule yields a structured error, not a prompt.
- **Construction from text that is not valid YAML/JSON**: the text factory returns a structured load error (the
  serde shape layer), distinct from an agreement error.
- **A `default`-named variant**: schema-invalid but loader-accepted (the serde shape layer cannot model the
  `propertyNames` rule); surfaced by `check()` as a reserved-name finding — this three-layer distinction
  (architecture memory A1) must be preserved through the reshape and the fixture move.
- **Round-trip parity across bindings**: a prompt carrying date/decimal values is pinned by canonical serialized
  string in the conformance corpus (decision memory D1); the fixture move must not tempt a runner into building
  native objects.
- **Empty / no-variable prompt**: constructs and renders unchanged.
- **TS construction of an invalid prompt**: per FR-014, `new Prompt({…})` throws a structured error.
- **`with(overlay)` and bound validators**: when an overlay changes `variables` (e.g. adds a
  `validation_required` variable), the merged prompt is re-validated through the same constructor — so the
  derived prompt must be supplied a validator covering any newly-required variable, or its construction fails.
  The carry-forward semantics of the *original* prompt's bound validators across `with` is a design detail for
  the plan phase (assumption: validators for unchanged variables carry; see Open Design Items log).

## Requirements *(mandatory)*

### Functional Requirements

#### Schema rename (`provenance` → `origin`)

- **FR-001**: The JSON Schema MUST rename the per-variable input-trust field from `provenance` to `origin` in
  `schemas/jsonschema/prompt-definition.schema.json`. The enum values `trusted | untrusted | external` MUST be
  unchanged.
- **FR-002**: The rename MUST apply **only** to the per-variable `VariableDecl` input-trust tag. The
  render-result provenance concept (the `template_hash` / `render_hash` carried on the render return value,
  Principle V) MUST NOT be renamed.
- **FR-003**: The three per-language shapes (Rust struct, Python Pydantic model, TypeScript type/schema) MUST be
  **regenerated** from the renamed schema — never hand-edited to match (Principle VII / C-07). The
  codegen-freshness CI gate MUST pass.
- **FR-004**: The kernel's origin-exposure surface (the view/struct/enum currently named around `provenance` in
  `crates/prompting-press-core/src/provenance.rs` — `provenance_view`, `ProvenanceView`,
  `VariableDeclProvenance`) MUST be renamed consistently to the `origin` vocabulary, with rendering, guard, and
  exposure **behavior unchanged**.
- **FR-005**: All downstream consumers MUST be updated to the renamed field: the Rust consumer crate, the Python
  binding, the TypeScript binding, all schema/render fixtures, the conformance corpus, and all docs/READMEs.
- **FR-006**: A prompt document or shape using the old `provenance` key MUST be rejected (the schema stays
  closed, `additionalProperties: false`); there is no compatibility alias.

#### Fixture move

- **FR-007**: The schema fixtures MUST move from `schemas/jsonschema/fixtures/{valid,invalid}/` to
  `schemas/jsonschema/tests/fixtures/{valid,invalid}/`.
- **FR-008**: The three references to the old fixture path MUST be updated: the fixture validator
  (`validate_fixtures.py`), the `schemas:validate-fixtures` moon-task input globs, and the conformance schema
  manifest (`conformance/schema/manifest.json`).
- **FR-009**: The fixture move MUST preserve the conformance manifest's documented loader-exclusion of
  `variant-named-default` (it is schema-invalid but loader-accepted — architecture memory A1); the three-layer
  validity distinction MUST remain correct after the move.

#### Prompt-as-object API

- **FR-010**: Each binding MUST expose a first-class **immutable** `Prompt` type that wraps the code-generated
  prompt shape (Facade). A `Prompt` MUST expose **read-only accessors** for its fields and MUST expose **no
  setters**.
- **FR-011**: Construction MUST be a **validating** operation that yields an immutable `Prompt` or a structured
  error, and MUST NOT panic/crash on invalid input.
- **FR-012**: The **primary constructor** MUST take the native shape object directly (`new Prompt({…})` in
  TypeScript / `Prompt(shape)` in Python / `Prompt::new(shape) -> Result` in Rust). Constructing from the
  native shape MUST NOT be a named `.fromObject`-style factory.
- **FR-013**: Named factories MUST parse foreign **text** into the same validating constructor:
  `Prompt.fromYaml(text)`, `Prompt.fromJson(text)`, and `Prompt.fromToml(text)`. Each binding pins a TOML
  parser (Rust `toml`, Python stdlib `tomllib`, a pinned JS TOML library). All three return/raise/throw a
  structured error on malformed text, never a panic.
- **FR-014**: Each binding MUST surface construction failure in its **native idiom**, with the failure carrying
  the normalized structured error shape `[{field, code, message}]` (native error types MUST NOT leak across FFI
  — C-06). Specifically: (a) the TypeScript shape MUST be a generated **Zod schema** (`json-schema-to-zod`), not
  a bare `interface`, so the TS validating constructor enforces at runtime; (b) the TS primary constructor
  `new Prompt({…})` MUST **throw** a structured error on invalid input (a TS `new` cannot return a result),
  mirroring Python's Pydantic raise; Rust returns `Result`.
- **FR-015**: `render`, `getSource`, and `check` MUST move **onto** the `Prompt` object as single-prompt
  operations. Their optional/config parameters MUST follow the options-object / keyword-only / Rust-threshold
  call shape (C-11).
- **FR-016**: The render output and hashing of a `Prompt.render(…)` MUST be **byte-identical** to the previous
  registry-keyed `render(reg, name, …)` for the same definition, variables, variant, and guard config — the
  kernel is unchanged (Principle I).
- **FR-017**: The **only** way to vary a prompt MUST be a copy-with-overlay operation `with(overlay) -> Result`
  that: (a) accepts a partial overlay of any top-level field(s); (b) **shallow-replaces** each supplied
  top-level field (no deep merge); (c) allows the overlay to change `name`; (d) validates the **merged whole**
  through the same validating constructor; (e) returns a **new** immutable `Prompt` (or error) and **never
  mutates the original**. This replaces `withVariant` and all per-field mutators.
- **FR-018**: Composition MUST aggregate `Prompt` **objects** (each with its variables/variant), resolving to an
  ordered list of `{role, text}` messages. It MUST require no registry to resolve references.
- **FR-019**: The `Registry` type and all name-keyed prompt-lookup free functions MUST be **removed** from the
  public surface of every binding. (A query-capable registry is explicitly deferred — Deferred wishlist, gated
  on a real consumer per C-08 — and is **out of scope** here.)
- **FR-020**: Construction MUST enforce every **decidable** invariant — including (a) that the body parses
  (a MiniJinja syntax error or an excluded `{% include/import/extends/macro/block %}` feature fails
  construction), and (b) that the parsed template's referenced variables are a subset of the declared variables
  (the sound agreement check, Principle IV). A constructed `Prompt` is therefore always parseable and
  analyzable; `check()`'s analysis-error finding is unreachable for a constructed `Prompt`.
- **FR-021**: `check()` MUST remain **pure analysis** — it MUST NOT mutate the template, variables, or output
  (Principle IV). After this reshape its hard invariants (parse + agreement + required-validator coverage) are
  enforced at construction, so `check()` survives as the **advisory lint** — its remaining live finding is the
  origin/guard advisory (a prompt declaring `untrusted`/`external` variables without a guard is *valid* but
  flagged).

#### Per-variable validators and `validation_required`

- **FR-022**: The schema MUST gain an optional per-variable boolean `validation_required` on each variable
  declaration (a sibling of `type` and `origin`). It is **orthogonal to `origin`** — it MAY mark any variable,
  not only `untrusted`/`external` ones. It is declarative metadata; it does not itself perform validation.
- **FR-023**: Validators MUST be **bound to the `Prompt` at construction** (the `Prompt` holds its
  validator(s) and reuses them at render), not supplied per render call. Validators MUST be acceptable as a
  **separate side input even when constructing from YAML/JSON/TOML text** (e.g. `Prompt.fromYaml(text,
  validators)`), so the prompt *document* stays language-agnostic while the *validators* are native per language.
- **FR-024**: At construction, every variable marked `validation_required: true` MUST have a corresponding
  validator, enforced per the language's idiom:
  - **TypeScript / Python** MUST **throw / raise** at construction if a `validation_required` variable is not
    covered by the supplied validator (introspecting the Zod schema's shape / the Pydantic model's fields).
  - **Rust** treats `validation_required` as **declarative metadata**; validator coverage is guaranteed
    **structurally at compile-time** (garde rules wired onto the `V: Validate` Vars type) — Rust does not
    perform a runtime coverage throw. This per-language asymmetry is intentional (native idiom, Principle VI)
    and requires the Principle VI amendment noted in Dependencies.
- **FR-025**: The kernel MUST remain **validation-blind** (C-06): per-variable validators run only in the
  binding/consumer layer; no validation logic, and no `validation_required` enforcement, enters
  `prompting-press-core`.

#### Cross-cutting (scope)

- **FR-026**: The prompt-as-object reshape MUST land in **all three bindings** (Rust, Python, TypeScript) so the
  library "feels like the same library" everywhere (Principle I / C-01) — including a Rust `Prompt` that wraps
  the generated struct with a generic `with` / `render::<V>`.
- **FR-027**: The kernel MUST NOT change: no new or altered rendering, agreement, variant-resolution, or hashing
  behavior (Principle I). The reshape is confined to the schema, codegen output, and the binding/consumer
  surfaces.
- **FR-028**: All existing CI gates MUST stay green throughout: FFI isolation (no `pyo3`/`napi` in kernel or
  consumer), codegen freshness, the agreement + origin lint, and the conformance corpus (FFI marshaling +
  schema round-trip).

### Key Entities

- **Prompt**: The first-class immutable object wrapping the code-generated prompt shape. Carries name, role,
  body, declared variables (each with type + `origin` tag), variants, and opaque metadata. Behavior:
  validating construction, read-only accessors, `render` / `getSource` / `check`, and `with` (copy-with-overlay).
- **Variable declaration**: A declared input variable carrying its JSON-Schema type, its **`origin`** tag
  (`trusted` | `untrusted` | `external`) — renamed from `provenance`; declarative metadata only (C-09) — and an
  optional **`validation_required`** boolean (orthogonal to `origin`).
- **Validator**: A native per-language value-validator (garde-derived Vars type in Rust, a Pydantic model in
  Python, a Zod schema in TypeScript), bound to the `Prompt` at construction. Covers render-time variable
  *values* — distinct from the serde document-shape layer and from `check()`'s static agreement analysis
  (architecture memory A1's three layers).
- **Overlay**: A partial set of top-level prompt fields supplied to `with`; each supplied field shallow-replaces
  the corresponding field on the source prompt; the merged whole is validated.
- **Composition**: An ordered aggregation of `Prompt` objects (+ their variables/variant) resolving to an
  ordered list of `{role, text}` messages. Holds objects, not names.
- **Origin tag**: The per-variable input-trust classification (the renamed field). Distinct from render-result
  provenance (the hashes on the return value), which is **not** renamed.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In all three bindings, a developer can construct a `Prompt` from a shape object and from
  YAML/JSON text, render it, read its source, derive a variant via `with`, and check it — using **no `Registry`
  type** (it does not exist in any public surface).
- **SC-002**: The published contract uses the field name `origin` everywhere; **zero** occurrences of the old
  per-variable `provenance` field name remain in the schema, the three generated shapes, the kernel
  origin-exposure surface, the bindings, the fixtures, the conformance corpus, or the docs. (The render-result
  provenance concept and its hash names are intentionally retained.)
- **SC-003**: For any prompt + variable set, `Prompt.render(…)` produces a `template_hash` and `render_hash`
  **byte-identical** to the pre-reshape registry-keyed render, and identical across all three bindings (the
  conformance corpus proves cross-binding identity; the kernel is unchanged).
- **SC-004**: A `with` overlay never mutates the source prompt: after any `with` call, re-reading the original
  prompt's fields yields its original values, and an overlay producing an invalid merged definition returns a
  structured error and no prompt — verified in every binding.
- **SC-005**: An invalid prompt fails at the right layer: a decidable agreement violation fails **construction**
  with a structured error; an un-analyzable-template residue is reported by **`check()`** (per the Q4 decision),
  and `check()` mutates nothing.
- **SC-006**: All schema fixtures resolve under `schemas/jsonschema/tests/fixtures/…`; the fixture validator,
  the `schemas:validate-fixtures` task, and the conformance manifest all reference the new path and pass; the
  `variant-named-default` loader-exclusion note is preserved.
- **SC-007**: Every in-repo example, binding test suite, and conformance runner passes against the object
  surface, and all CI gates (FFI isolation, codegen freshness, agreement/origin lint, conformance) are green.
- **SC-008**: Construction failures, render-validation failures, and load failures surface in each language's
  native idiom while carrying the normalized `[{field, code, message}]` structured error shape; no native error
  type (garde `Report`, Pydantic/Zod error) leaks across the FFI boundary.
- **SC-009**: A prompt with a variable marked `validation_required: true` cannot be constructed in
  Python/TypeScript without a validator covering that variable (construction raises/throws); in Rust the same
  guarantee holds at compile-time. A prompt body using an excluded template feature or containing a syntax error
  cannot be constructed in any binding (construction fails).
- **SC-010**: `Prompt.fromYaml`, `Prompt.fromJson`, and `Prompt.fromToml` each construct an equivalent `Prompt`
  from the same logical document expressed in that format, accepting validators as a side input; malformed text
  in any format yields a structured load error, never a panic.

## Assumptions

- **All decisions resolved at clarify** (2026-06-28 session): all-bindings incl. Rust `Prompt` (FR-026);
  construction fails on un-analyzable bodies (FR-020); TypeScript uses a generated Zod schema and `new Prompt`
  throws (FR-014); `validation_required` ships as a per-variable field with validators bound at construction
  (FR-022–025); `fromToml` ships (FR-013). See the Clarifications section.
- **The rendering engine, hashing, and agreement algorithm are reused unchanged** (Principle I); no kernel
  behavior is in scope. Cross-binding render parity is structural and is not re-verified by new tests.
- **The examples repo** (`prompting-press-examples`, a separate unpushed local repo) is **not** part of this
  spec's file set and is not modified from this repository.
- **No managed version axis, no I/O, no LLM/request-body assembly, no token counting** is introduced — the
  minimal boundary (Principle III) is unchanged.

## Dependencies

- **Depends on** spec 006 (the conformance corpus references the fixtures and the renamed field).
- **Blocks** spec 007 (v1 release — publish the final contract) and spec 010 (docs site — document the final
  API + field name).
- **Requires a constitution amendment** (MINOR, Principle VI) before or during the plan phase: the validator is
  bound at construction (not only at render); construction enforces per-variable validator coverage; and the
  enforcement mechanism is intentionally asymmetric (TypeScript/Python throw at runtime via validator
  introspection, Rust guarantees coverage structurally at compile-time and treats `validation_required` as
  declarative). This MUST be routed through `/speckit.constitution` (recorded in `DECISIONS.md`), not authored
  ad hoc. The plan's Constitution Check surfaces it.

## Out of Scope

- Any change to rendering, agreement, variant-resolution, or hashing **behavior** (Principle I).
- The query-capable prompt **Registry** (Deferred wishlist; C-08-gated on a real consumer).
- Adversarial hardening / fuzzing (spec 009) and the documentation site (spec 010).
- Runtime **enforcement** of the `origin` tag or `validation_required` beyond "a validator ran" — the origin tag
  stays declarative metadata (C-09).
