# Feature Specification: Engine kernel (`prompting-press-core`)

**Feature Branch**: `002-engine-kernel`

**Created**: 2026-06-26

**Status**: Refined

**Refined**: 2026-06-26 — FR-010 corrected to match the authoritative 001 schema (root `body` is always the default arm; removed the structurally-unreachable "missing-default" error path). Propagated to US1 scenario 4 and SC-004. Downstream plan/research/data-model already reflect this reading. Follow-up (same day, surfaced by the requirements-quality checklist): FR-028 cleaned of the deleted missing-default error and the strict-undefined (FR-001a) error class added; the "Resolved variant" entity reworded off the old "explicit default" model. Follow-up 2 (same day, from critique + security review): added FR-016a (agreement analysis must error on a non-parseable template, never return an empty required-roots set — research D2).

**Input**: User description: "Engine kernel (`prompting-press-core`): the binding-agnostic, validation-blind Rust engine that turns already-validated values + a prompt definition into rendered text + provenance. Render path (interpolation/conditionals/loops only), the sound agreement analysis, variant resolution, hashing, var-provenance plumbing + opt-in additive guard expansion, and a small engine-regression render-fixture set."

## Overview

The engine kernel is the single place where Prompting Press *does its work*. Given a prompt
definition (the schema-defined shape from spec 001) and a set of already-validated input values, the
kernel renders the prompt to text and returns provenance describing exactly what ran. Every language
binding (Rust consumer, Python, TypeScript) will call **this** engine, so the behavior defined here is
the behavior all of them inherit — byte-for-byte, by construction (constitution Principle I / C-01).

The kernel is deliberately narrow (Principle III / C-03): it receives values that are *already valid*
and knows nothing about how they were validated; it performs no file/network/database access, makes no
model calls, assembles no request body, counts no tokens, and parses no model output. It turns
*typed inputs + a template* into *rendered text + provenance*. Nothing else.

This spec covers the kernel's four capabilities: (1) the render path, (2) the sound agreement
analysis — the library's headline differentiator, (3) variant resolution and content-addressed
provenance, and (4) var-provenance plumbing with an opt-in, additive guard expansion. The typed-Vars
facade, the dual-input loader, and the CI lint entry points are the *consumer's* concern (spec 003)
and are explicitly out of scope here.

## Clarifications

### Session 2026-06-26

- Q: At render time, how should the kernel treat a variable referenced in the template but absent from
  the supplied values? → A: **Strict — render errors.** Undefined variable use causes a loud render
  error (defense-in-depth backstop to the static agreement check); intentional-optional references
  must use an explicit `is defined`-style guard.
- Q: What granularity should the agreement analysis expose for a prompt's required root variables? →
  A: **Per resolved variant.** The kernel reports the required-root set for one template/variant source
  at a time; aggregating across a prompt's variants is the consumer's concern.
- Q: Where should the opt-in guard instruction be placed relative to the rendered body? → A: **As a
  separate field on the render result, configurable with a default.** The guard text is returned as a
  distinct field (never concatenated into the rendered body); the guard template is caller-configurable
  with a provided default. Placement into a final prompt is the caller's decision.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Render a prompt to text with content-addressed provenance (Priority: P1)

A consumer of the kernel (initially the Rust consumer crate, later the language bindings) holds a
prompt definition and a set of already-validated values. It asks the kernel to render a named variant
(or the default) and receives back the rendered text plus provenance — the variant that ran and two
content hashes that identify the exact template text and the exact filled-in output.

**Why this priority**: This is the foundational, viable slice — the kernel's reason to exist at the
most basic level. Without it there is nothing to analyze, hash, or guard. It also establishes the
single rendering site that makes cross-language byte-identity structural rather than tested.

**Independent Test**: Provide a prompt definition (with and without variants) plus values and confirm
the kernel returns the correct rendered text, resolves the right variant (implicit default,
explicitly named, or a loud error when a multi-variant prompt has no default), and emits a stable
`template_hash` and `render_hash`. Fully testable with no other capability present.

**Acceptance Scenarios**:

1. **Given** a prompt with a single (default) body `"Hello {{ name }}"` and values `{name: "Ada"}`,
   **When** the kernel renders with no variant specified, **Then** it returns text `"Hello Ada"`, the
   resolved variant `default`, a `template_hash` over the body source, and a `render_hash` over
   `"Hello Ada"`.
2. **Given** the same prompt rendered twice with identical values, **When** both renders complete,
   **Then** the two outputs are byte-identical and both hashes are equal across the two renders.
3. **Given** a prompt with variants `{concise, verbose}` and an explicit default of `concise`,
   **When** the kernel renders with no variant specified, **Then** it renders `concise` and stamps
   `variant = concise`.
4. **Given** a prompt with variants `{a, b}` and a root `body`, **When** the kernel renders with no
   variant specified, **Then** it deterministically renders the root `body` as variant `default` (the
   root body is always the default; it MUST NOT silently pick among the named arms `a`/`b`).
5. **Given** a prompt with variants `{a, b}`, **When** the kernel is asked to render variant `c`,
   **Then** it returns an "unknown variant" error naming `c`.
6. **Given** a template using a conditional and a loop over a provided list, **When** rendered with
   matching values, **Then** the output reflects the conditional branch and loop iterations.
7. **Given** two different variants of the same prompt, **When** each is rendered, **Then** each
   carries its own `template_hash` (computed over that variant's own source).
8. **Given** template `"Hello {{ name }}"` and values `{}` (no `name` provided), **When** rendered,
   **Then** the kernel returns a loud render error rather than `"Hello "` (strict undefined handling).

---

### User Story 2 - Sound agreement analysis: report a template's required variables (Priority: P2)

A consumer wants to catch, *before* render, the class of bug where a template references a variable
the caller never declared — which would otherwise render to a silent empty string. The kernel
analyzes a template and reports the set of **root** variable names the template requires, soundly
excluding names that are locally bound inside the template (loop variables, `{% set %}` targets, block
locals) and engine-provided globals/filters. The consumer then compares this set against its declared
typed-Vars fields; the kernel itself only reports the referenced set.

**Why this priority**: This is the verified differentiator — the BAML-equivalent static guarantee no
file-based prompt library provides (constitution Principle IV / C-04). It is P2 only because it is
analytically independent of and buildable after the render path; in product terms it is the headline
feature.

**Independent Test**: Feed the kernel templates exercising interpolation, conditionals, loops,
`{% set %}`, and global/filter usage, and assert the reported required-root-variable set is exactly
the externally-supplied roots — excluding loop locals, set targets, block locals, and the
globals/filters allowlist. Testable with no rendering performed.

**Acceptance Scenarios**:

1. **Given** template `"{{ greeting }}, {{ user.name }}"`, **When** analyzed, **Then** the required
   roots are exactly `{greeting, user}` (nested field `name` is not a root — deep shape is the type
   system's job).
2. **Given** template `"{% for item in items %}{{ item }}{% endfor %}"`, **When** analyzed, **Then**
   the required roots are exactly `{items}` — the loop local `item` is excluded.
3. **Given** template `"{% set x = 1 %}{{ x }}{{ y }}"`, **When** analyzed, **Then** the required
   roots are exactly `{y}` — the `{% set %}` target `x` is excluded.
4. **Given** a template using an engine global or built-in filter (e.g. a range or a default filter),
   **When** analyzed, **Then** the global/filter name does not appear in the required-root set.
5. **Given** any template, **When** analyzed, **Then** the analysis returns without modifying the
   template, the values, or any output (pure analysis).
6. **Given** a template that references a variable the caller did not provide, **When** the consumer
   compares required roots against declared fields, **Then** the missing variable is identifiable as a
   reported requirement (rather than surfacing as a silent empty render).

---

### User Story 3 - Var-provenance plumbing and opt-in guard expansion (Priority: P3)

A prompt declares per-variable provenance tags (`trusted | untrusted | external`). The kernel carries
these tags through as data and exposes which fields are untrusted/external. On an explicit, per-render
opt-in, the kernel produces a configurable guard instruction naming the untrusted/external fields and
returns it as a separate field on the result — additive and non-mutating, never altering the rendered
body and never stripping or sanitizing values.

**Why this priority**: It is the security-oriented capability built on top of render + provenance
(constitution Principle IV / C-09). It is additive and opt-in, so it is the last independent slice;
the prior two stories are fully viable without it.

**Independent Test**: Provide a prompt whose variables carry mixed provenance tags; confirm the kernel
exposes the untrusted/external field names, that a render without the opt-in is unchanged, and that a
render with the opt-in appends the (default or overridden) guard text naming exactly those fields,
with the original body left intact.

**Acceptance Scenarios**:

1. **Given** a prompt with fields tagged `{q: untrusted, ctx: external, sys: trusted}`, **When** the
   consumer queries untrusted/external fields, **Then** the kernel reports `{q, ctx}`.
2. **Given** that prompt rendered **without** opt-in guard expansion, **When** rendered, **Then** the
   rendered body equals the plain render and the guard field is absent/empty.
3. **Given** that prompt rendered **with** opt-in guard expansion and the default guard template,
   **When** rendered, **Then** the result carries a separate guard field naming `q` and `ctx`, and the
   rendered body is byte-identical to the plain render (the guard is a separate field, not appended to
   the body).
4. **Given** that prompt rendered with a caller-overridden guard template, **When** rendered, **Then**
   the guard field contains the override text in place of the default.
5. **Given** any render, **When** guard expansion is applied, **Then** no untrusted/external value is
   stripped, escaped-away, or otherwise mutated (the values pass through unchanged).

---

### Edge Cases

- **Excluded template features**: a template containing `{% include %}`, `{% import %}`,
  `{% extends %}`, a macro definition, or template inheritance MUST be rejected with a clear error
  rather than silently rendering or silently passing the agreement analysis — this exclusion is what
  keeps the agreement check sound (C-04).
- **Variant literally named `default`**: the reserved name `default` always maps to the root body;
  the kernel's own logic enforces this (the generated type does not encode it).
- **Empty template body**: renders to an empty string with valid hashes; the agreement analysis
  reports an empty required-root set.
- **Undefined variable at render**: referencing a variable absent from the supplied values surfaces a
  loud render error (strict undefined), not a silent empty string; intentionally-optional references
  must use an explicit defined-check in the template (FR-001a).
- **Render-time failure inside an allowed feature** (e.g. iterating a value that is not iterable):
  surfaces as a structured render error, not a panic.
- **Conservative analysis cases** (documented, benign): dynamic subscripts (`obj[key]`) are reported
  conservatively, and flow-insensitivity may miss a use-before-`set` — both are false-negatives, not
  false-positives, and are acceptable per the design (render-smoke fixtures catch them).
- **Unicode / multibyte content**: rendering and hashing operate over the string faithfully; hashes
  are over the UTF-8 string content.

## Requirements *(mandatory)*

### Functional Requirements

#### Render path

- **FR-001**: The kernel MUST render a prompt's selected template to text using a Jinja-family
  template engine restricted to **interpolation, conditionals, and loops only**.
- **FR-001a**: The kernel MUST use **strict undefined handling**: referencing a variable that is
  absent from the supplied values MUST cause a loud render error, never a silent empty substitution
  (defense-in-depth backstop to the static agreement analysis). Intentionally-optional references MUST
  be expressed with an explicit defined-check (e.g. `is defined`) in the template.
- **FR-002**: The kernel MUST reject templates that use `{% include %}`, `{% import %}`,
  `{% extends %}`, macros, or template inheritance, surfacing a clear error (these features are
  excluded to preserve agreement-check soundness).
- **FR-003**: Rendering MUST be deterministic: the same prompt definition, same values, and same
  resolved variant MUST produce byte-identical output every time.
- **FR-004**: The kernel MUST be validation-blind — it receives already-validated values and performs
  no type validation, coercion, or constraint checking of its own.
- **FR-005**: The kernel MUST NOT perform any I/O (no file, network, database, or environment access),
  make any model/LLM call, assemble any provider request body, count tokens, or parse model output.
- **FR-006**: The kernel MUST expose the unrendered source of a resolved variant (a `get_source`-style
  operation) in addition to rendering.

#### Variant resolution

- **FR-007**: A prompt with no declared variants MUST expose an implicit variant named `default` whose
  source is the prompt's root body.
- **FR-008**: Variant selection MUST be caller-owned: the kernel renders the variant named by the
  caller (or the default when none is named) and MUST NOT implement any experiment-assignment,
  weighting, or deterministic-selection logic.
- **FR-009**: When asked to render a variant name that does not exist, the kernel MUST return an
  "unknown variant" error that names the requested variant.
- **FR-010**: Every prompt's root `body` is its default arm (the `body` field is required by the
  prompt-definition schema). A render with no variant named MUST deterministically resolve to that root
  body (surfaced as the reserved variant `default`) — for single- and multi-variant prompts alike.
  There is therefore no "missing default" condition and no separate missing-default error path; the
  kernel MUST NOT silently pick among *named* arms (the default is always the root body, never a chosen
  alternative). The only variant-resolution error is unknown-variant (FR-009).
- **FR-011**: The name `default` MUST be reserved and always resolve to the root body; the kernel MUST
  enforce this in its own resolution logic (the generated type does not encode it).

#### Hashing & provenance

- **FR-012**: For each resolved variant, the kernel MUST emit `template_hash = SHA256(variant template
  source)`, computed over the exact source string returned by FR-006.
- **FR-013**: For each resolved variant, the kernel MUST emit `render_hash = SHA256(rendered output)`,
  computed over the exact rendered text string.
- **FR-014**: The kernel MUST NOT compute or emit a `vars_hash` or any hash over structured input
  values.
- **FR-015**: Provenance MUST be returned as plain data on the render result (at minimum: rendered
  text, prompt name, resolved variant name, `template_hash`, `render_hash`, and — when guard expansion
  is opted in — the separate guard field per FR-022); the kernel MUST NOT emit to any telemetry sink or
  couple to a tracing framework.

#### Sound agreement analysis

- **FR-016**: The kernel MUST expose, **per resolved variant** (i.e. for a single template/variant
  source at a time), the set of **root** variable names that template references (the "required
  roots"), using the engine's stable undeclared-variables analysis in its **non-nested** mode (root
  names only; deep field shape is out of scope here). Aggregating required roots across a prompt's
  several variants is the consumer's concern, not the kernel's.
- **FR-016a**: The agreement analysis MUST first ensure the template parses successfully and MUST
  return a parse / excluded-feature error for a non-parseable template — it MUST NOT return an empty
  (or partial) required-roots set for a template that failed to parse. (Rationale: the underlying
  stable analysis yields an empty set on parse failure, which would otherwise let a broken or
  excluded-feature template masquerade as "requires no variables" and silently pass the headline
  guarantee. See research D2.)
- **FR-017**: The required-roots set MUST exclude names locally bound within the template — loop
  variables, `{% set %}` targets, and block locals — and MUST exclude a known allowlist of
  engine-provided globals and filters.
- **FR-018**: The agreement analysis MUST be pure: it MUST NOT mutate the template, the values, or any
  output, and MUST NOT render as a side effect.
- **FR-019**: The kernel MUST expose the per-variant required-roots set as its output; comparing
  required roots against a set of declared variable fields ("referenced ⊆ declared") is the consumer's
  responsibility and is NOT performed by the kernel.
- **FR-020**: The globals/filters allowlist MUST be derived from the specific pinned engine version
  the kernel depends on (not assumed from another version).

#### Var-provenance plumbing & guard expansion

- **FR-021**: The kernel MUST carry each variable's provenance tag (`trusted | untrusted | external`)
  through as data and MUST expose which fields are tagged `untrusted` or `external`.
- **FR-022**: The kernel MUST support an **opt-in, per-render** guard expansion. When opted in, the
  kernel MUST produce a guard instruction naming the untrusted/external fields and return it as a
  **separate field on the render result** (distinct from the rendered body); it MUST NOT concatenate
  the guard text into the rendered body. When not opted in, the guard field MUST be absent/empty and
  the rendered body MUST be identical to a plain render.
- **FR-023**: The guard expansion MUST be **additive and non-mutating** — producing the guard field
  MUST NOT modify the template, the values, or the rendered body content. Where the guard text is
  ultimately placed in a final prompt is the caller's decision.
- **FR-024**: The guard instruction text MUST be configurable: a default guard template is provided and
  MUST be fully overridable by the caller per render.
- **FR-025**: The kernel MUST NOT sanitize, strip, escape-away, or otherwise mutate untrusted/external
  values; provenance handling is metadata + lint + opt-in guard only.

#### Structure, errors, and regression guard

- **FR-026**: The kernel MUST be implemented in the `prompting-press-core` crate and MUST NOT depend on
  `pyo3`, `napi`, or any FFI binding crate (verifiable from its dependency manifest; the existing CI
  FFI-isolation gate MUST stay green).
- **FR-027**: The kernel MUST consume the spec-001 generated prompt-definition Rust shape as its input
  contract and MUST NOT redefine that shape.
- **FR-028**: The kernel MUST surface structured errors distinguishing at least: unknown variant
  (FR-009), use of an excluded template feature (FR-002), template parse failure, strict-undefined
  variable use at render (FR-001a), and other render-time failure. (There is no missing-default error —
  the root `body` is always the default per FR-010.)
- **FR-029**: A small engine-regression render-fixture set MUST exist that pins representative
  template → output results as a regression guard for the kernel only (it is NOT a cross-language
  render-parity corpus — parity is structural per C-01).

### Key Entities *(include if feature involves data)*

- **Prompt definition (input)**: the schema-defined shape from spec 001 — root body, role, name,
  declared variables (each with a type and a provenance tag), and named variants (each differing only
  in body). Consumed, not redefined, by the kernel.
- **Resolved variant**: the variant selected for a render — either an explicitly named arm, or the
  `default` (the root body, always present; resolved when no variant is named). Carries the source
  string that `template_hash` is computed over.
- **Render result (output / provenance)**: rendered text plus provenance data — prompt name, resolved
  variant name, `template_hash`, `render_hash`, and (when guard expansion is opted in) a separate guard
  field. Returned to the caller as plain data.
- **Required-roots set**: the per-variant set of root variable names a template references, with local
  bindings and the globals/filters allowlist excluded. The output of the agreement analysis.
- **Provenance tag**: a per-variable label (`trusted | untrusted | external`) carried as data and used
  to drive the opt-in guard expansion.
- **Guard field**: an optional, separate field on the render result holding the configurable guard
  instruction (naming the untrusted/external fields) when guard expansion is opted in; never
  concatenated into the rendered body, and placed into a final prompt at the caller's discretion.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Re-rendering any prompt with identical inputs yields byte-identical output and identical
  `template_hash`/`render_hash` across 100% of repeated renders (determinism).
- **SC-002**: For every fixture template, the reported required-root set excludes 100% of loop
  variables, `{% set %}` targets, block locals, and allowlisted globals/filters — i.e. the analysis is
  demonstrably more sound than a naive undeclared-variable scan that leaks those four classes.
- **SC-003**: A template referencing an undeclared variable is detectable via the reported
  required-roots set in 100% of cases, rather than rendering to a silent empty value.
- **SC-004**: Variant resolution behaves correctly across all declared cases: default (no variant
  named → root `body`, for single- and multi-variant prompts alike), named-variant selection, and
  unknown-variant error — with zero cases of a silently-chosen named arm.
- **SC-005**: Opting out of guard expansion produces a rendered body byte-identical to a plain render
  with no guard field; opting in returns a separate guard field naming exactly the untrusted/external
  fields while the rendered body stays byte-identical to the plain render (guard is never appended to
  the body).
- **SC-009**: A template referencing a variable absent from the supplied values produces a loud render
  error in 100% of cases — never a silent empty substitution (strict undefined).
- **SC-006**: The agreement analysis and the provenance handling mutate nothing — verified by
  confirming template, values, and output are unchanged after analysis across all fixtures.
- **SC-007**: The `prompting-press-core` crate has zero `pyo3`/`napi`/FFI dependencies (the CI
  FFI-isolation gate passes) and the kernel builds with no I/O, model-call, request-body, token-count,
  or output-parsing capability present.
- **SC-008**: Every excluded template feature (`include`, `import`, `extends`, macros, inheritance) is
  rejected with a clear error in 100% of fixture cases — none renders silently and none passes the
  agreement analysis as if benign.

## Assumptions

- **Already-validated inputs**: the kernel assumes values handed to it have already been validated by a
  consumer layer (spec 003+); it neither re-validates nor coerces. The shape of "values" at the kernel
  boundary (a serde-compatible value map) is settled in planning.
- **Engine choice and version**: the Jinja-family engine is MiniJinja (per the resolved design). The
  exact version is pinned during planning, and the stable non-nested undeclared-variables API plus the
  globals/filters allowlist are re-confirmed against that pinned version (the design reference was
  MiniJinja 2.21; re-confirmation is a planning task — roadmap Q3).
- **Excluded features are rejected loudly**: the kernel actively rejects excluded template features
  rather than relying on incidental engine behavior — chosen to make the soundness guarantee explicit
  and testable (confirmed; see Clarifications and FR-002).
- **Strict undefined handling** (confirmed, see Clarifications): undefined variable use at render is a
  loud error, not a silent empty string (FR-001a). Intentionally-optional template references rely on
  an explicit defined-check.
- **Guard text is a separate result field** (confirmed, see Clarifications): the guard instruction is
  returned as a distinct, caller-configurable field with a provided default — never concatenated into
  the rendered body. The exact wording/format of the default guard template is a design detail settled
  in planning; the invariant is that it is additive and non-mutating (FR-022..FR-024).
- **Required-roots granularity** (confirmed, see Clarifications): the agreement analysis reports
  required roots **per resolved variant**; how the consumer aggregates across a prompt's variants is the
  consumer's concern.
- **No cross-language work here**: bindings (Python/TS), the typed-Vars facade, the dual-input loader,
  composition sugar, the CI lint entry point, and the FFI conformance corpus are out of scope and land
  in specs 003+.
- **Reserved-name enforcement**: because the generated type does not encode the reserved-`default` rule
  (typify stripped `propertyNames` in spec 001), the kernel enforces variant-naming/default semantics
  in its own logic.

## Dependencies

- **Spec 001 (Foundations) — satisfied/merged**: provides the crate layout, the prompt-definition JSON
  Schema, the generated Rust shape the kernel consumes, and the CI FFI-isolation + codegen-freshness
  gates the kernel must keep green.

## Governance Alignment

This feature is governed by constitution Principles **I** (shared core / structural parity), **III**
(minimal boundary), **IV** (typed input / sound agreement check), and **V** (repo-canonical;
provenance hashes), and by roadmap decisions **C-01, C-03, C-04, C-05, C-09**. No new pluggable
interface is introduced (C-08), and no boundary-expanding capability (I/O, LLM calls, request-body
assembly, token counting, output parsing, managed version axis) is added (C-03 / boundary defense).
