# Feature Specification: Foundations — Layout, Schema, Codegen, CI Guardrails

**Feature Branch**: `001-foundations`

**Created**: 2026-06-25

**Status**: Implemented (2026-06-26 — all 35 tasks shipped, Phase-3 QA complete, CI green)

**Input**: Roadmap ledger entry 001 (`.specify/memory/roadmap.md`); constitution v1.0.0
(`.specify/memory/constitution.md`); resolved scope (`docs/research/feature-scope.md`).

This is the first spec for Prompting Press. It builds the structural spine that specs 002–007
depend on: the crate/package layout, the prompt-definition JSON Schema (the single source of truth),
the schema→shape codegen pipeline, and the CI guardrails that enforce the constitution's structural
invariants. **No prompt renders in this spec** — there is no engine, no validation, no rendering. The
deliverable is a buildable, governed skeleton with the contract authored and enforced.

## Clarifications

### Session 2026-06-25

- Q: Where do per-field variable-provenance tags live, and is the typed Vars model hand-authored or
  generated? → A: **Generate-then-extend.** The schema's `variables` block carries, per variable,
  name + type + provenance tag + JSON-Schema validation constraints. Codegen emits a real
  Pydantic/Zod/garde model from it (type-safe; native validators like `format: email`, `minimum`,
  `pattern` come through), which the user *extends* in-language for validation JSON Schema can't
  express (cross-field, arbitrary predicates). This **reverses the grilling session's "hand-authored
  Vars" lean** — Pydantic/Zod/garde remain the validation runtime and the generation *target*, not a
  replacement. (Per-prompt Vars-model generation is a spec 003+ consumer concern; spec 001 only makes
  the schema's `variables` block rich enough to support it.)
- Q: What shape is the output-model reference? → A: **Optional opaque string** (e.g. `"NodeOutput"`).
  Stored and echoed; never resolved, loaded, or parsed (the only form that is both portable across
  languages and boundary-safe per Principle III).
- Q: What differs per variant vs. is shared? → A: **Only the template body differs per variant.**
  Role, the `variables`/provenance set, and the output-model reference are shared (they define the
  prompt's identity and input contract; one Vars model, one agreement check). **No model concept** —
  the library records no model; model-specific phrasing (e.g. an Opus arm vs a Haiku arm) is achieved
  by naming variants and having the caller select them (selection is caller-owned, per the
  constitution).
- Q: How is the default variant modeled? → A: **Root `body` is the default arm.** A prompt definition
  carries its default template at the root `body` field; `variants:` holds named override objects
  (each `{ body, meta? }`). There is no `default:` marker — the default is structural, so a definition
  can never have zero or two defaults. The default arm is surfaced with the reserved name `default`
  and an `is_default: true` flag (name = how you select it; flag = how you identify the fallback). A
  `variants:` entry literally named `default` MUST be rejected (collision with the root).
- Q: How is programmatic variant selection (round-robin, A/B, grouping) supported? → A: **The library
  exposes selection metadata; the caller selects (Branch A).** Each variant — and the default — may
  carry an optional, free-form, **library-opaque** `meta` object (e.g. weight, group, tags). The
  library stores and exposes it, and exposes the full ordered arm list (default included), so the
  caller can implement round-robin, weighted A/B, grouping, or any programmatic selection. The library
  never interprets `meta`, never selects, and holds no selection state — it stays a stateless, pure
  renderer (Principles III and V; this does not re-introduce the eliminated `VariantSelector` seam).

## User Scenarios & Testing *(mandatory)*

The "users" of this foundational spec are **library contributors** (who build, extend, and add the
later specs' logic) and the **CI system** (which enforces the constitution mechanically). Each story
is an independently demonstrable slice of the spine.

### User Story 1 - Buildable polyglot workspace with the load-bearing layout (Priority: P1)

A contributor clones the repository and builds every workspace member with one orchestrated command.
The layout reflects the architecture: a binding-agnostic engine kernel, a Rust consumer layer, two
binding crates (Python, TypeScript), published-package wrappers, and a reserved Go placeholder.

**Why this priority**: nothing else can exist until the crates and packages it lives in exist. This
is the foundation the schema, codegen, and all later engine/binding work attach to.

**Independent Test**: clone fresh, run the single orchestrated build/test command, and observe that
every workspace member resolves and builds (each crate compiles as a stub; each package's build
runs), with no member depending on another in a way that violates the layered structure.

**Acceptance Scenarios**:

1. **Given** a fresh clone, **When** the contributor runs the orchestrated build, **Then** the
   engine-kernel crate, the Rust consumer crate, the Python binding crate, and the TypeScript binding
   crate all build successfully (as stubs — no behavior yet).
2. **Given** the workspace, **When** a contributor inspects the dependency graph, **Then** the Rust
   consumer crate depends on the engine kernel, and each binding crate depends on the kernel/consumer
   — but the engine kernel depends on neither binding nor any FFI crate.
3. **Given** the layout, **When** a contributor looks for the Go target, **Then** a clearly-marked
   reserved placeholder exists with no Go toolchain wired and no build expectation (Go is deferred).
4. **Given** the bootstrap's original flat `packages/{python,typescript,go,rust}` layout, **When**
   this spec is complete, **Then** that flat layout has been replaced by the load-bearing structure
   and no orphaned/duplicate skeleton remains.

---

### User Story 2 - Prompt-definition JSON Schema as the single source of truth (Priority: P2)

A contributor (and, later, every binding) reads one authoritative schema that defines the shape of a
prompt definition: its role, template body, the set of named variants and their default rules, free
metadata, an output-model reference, and per-field variable-provenance tags. The schema expresses
everything the later specs will need, even though nothing consumes most of those fields yet.

**Why this priority**: the schema is the contract from which all per-language shapes are generated
(Principle VII). Getting its shape right now prevents churning the single source of truth — and every
downstream language artifact — later.

**Independent Test**: validate a set of well-formed example prompt-definition documents against the
schema (all accepted) and a set of malformed documents (each rejected for the expected reason —
unknown role, multi-variant set with no declared default, invalid provenance tag, etc.).

**Acceptance Scenarios**:

1. **Given** the schema, **When** validated as a JSON Schema document itself, **Then** it is a valid,
   self-consistent schema with a stable identifier.
2. **Given** a well-formed prompt-definition document (single variant, valid role, valid provenance
   tags), **When** validated against the schema, **Then** it is accepted.
3. **Given** a prompt-definition document declaring multiple variants but no default, **When**
   validated, **Then** it is rejected. **Given** the same with an explicit default named, **Then** it
   is accepted.
4. **Given** a document with an unrecognized role, an invalid provenance tag, or a structurally
   malformed variant set, **When** validated, **Then** it is rejected with a locatable error.
5. **Given** the schema, **When** a contributor reads it, **Then** it expresses all v1 fields the
   roadmap names — role (`system|user|assistant`), template body, variant set with default rules,
   metadata, output-model reference, and 3-way per-field provenance tags
   (`trusted|untrusted|external`) — without yet requiring any consumer to use them.

---

### User Story 3 - Codegen pipeline: schema → per-language shapes (Priority: P3)

A contributor regenerates the per-language prompt-definition shapes (Python, TypeScript, Rust) from
the JSON Schema with one command. The generated artifacts are committed to the repository, and
regeneration is deterministic.

**Why this priority**: codegen is what makes "one schema, N language shapes" hold without hand-sync
drift (Principle VII). It is also a build-pipeline dependency for every later binding spec.

**Independent Test**: run the codegen command, observe three language artifacts produced; run it again
and observe byte-identical output (deterministic); change one field in the schema, regenerate, and
observe the corresponding change appear in all three language artifacts.

**Acceptance Scenarios**:

1. **Given** the schema, **When** the codegen command runs, **Then** a Python shape, a TypeScript
   shape, and a Rust shape are produced, each faithfully representing the schema's prompt-definition
   structure.
2. **Given** generated artifacts already committed, **When** codegen is re-run with an unchanged
   schema, **Then** the output is byte-identical to what is committed (deterministic, no diff).
3. **Given** the schema is edited (e.g., a metadata field added), **When** codegen is re-run, **Then**
   all three language artifacts reflect the change.
4. **Given** the generated artifacts, **When** a contributor inspects them, **Then** they are clearly
   marked as generated (not hand-edited) and live at a predictable per-language location.

---

### User Story 4 - CI guardrails enforce the constitution's structural invariants (Priority: P1)

The CI system mechanically enforces two non-negotiable invariants: (a) the engine kernel and Rust
consumer crate never depend on an FFI binding crate, and (b) the committed generated shapes always
match what the schema would regenerate.

**Why this priority**: these are the constitution made executable (Principles II and VII). Without
them, the load-bearing guarantees erode silently over time as contributors add code. P1 because a
guardrail that lands late has already failed to protect the earliest commits.

**Independent Test**: in a scratch branch, add an FFI dependency to the engine-kernel crate and
observe CI fail with a clear message; separately, hand-edit a generated shape (or the schema without
regenerating) and observe the freshness check fail.

**Acceptance Scenarios**:

1. **Given** CI on a clean tree, **When** it runs, **Then** the FFI-isolation check and the
   codegen-freshness check both pass.
2. **Given** a change that adds `pyo3` or `napi` (or any binding/FFI dependency) to the engine-kernel
   crate or the Rust consumer crate, **When** CI runs, **Then** it fails, citing the violated
   invariant (Principle II / C-02).
3. **Given** a committed generated shape that no longer matches schema regeneration (schema changed
   without regenerating, or a generated file hand-edited), **When** CI runs, **Then** the freshness
   check fails (Principle VII / C-07).
4. **Given** a guardrail failure, **When** a contributor reads the CI output, **Then** the message
   identifies which invariant failed and where, so the fix is obvious.

---

### Edge Cases

- **A new crate is added later that should also be FFI-free** (e.g., a future shared utility crate):
  the FFI-isolation check's scope must be explicit about which crates it covers, so adding a crate
  doesn't silently fall outside the guardrail. (This spec covers the kernel and Rust consumer crate;
  the check's crate list is itself a reviewable artifact.)
- **The schema is edited but only some language artifacts are regenerated**: the freshness check must
  catch a partial regeneration, not just a wholly-stale one.
- **A malformed prompt-definition document that is valid JSON but violates the schema** must be
  rejected by validation, distinctly from a document that is not valid JSON/YAML at all.
- **Go placeholder**: must not be picked up by the orchestrated build as a buildable member, and must
  not cause CI to expect a Go toolchain.
- **Generated artifact location collides with hand-written code**: generated files must be
  segregated so a contributor never accidentally hand-edits one (and the freshness check would catch
  it if they did).

## Requirements *(mandatory)*

### Functional Requirements

**Layout**

- **FR-001**: The repository MUST provide an engine-kernel crate that is binding-agnostic and depends
  on no FFI binding crate.
- **FR-002**: The repository MUST provide a Rust consumer crate that depends on the engine kernel and
  is the user-facing Rust library surface; it MUST NOT depend on any FFI binding crate.
- **FR-003**: The repository MUST provide a Python binding crate and a TypeScript binding crate, each
  depending on the kernel/consumer, and each the only place its respective FFI dependency may appear.
- **FR-004**: The repository MUST provide published-package wrapper locations for Python and
  TypeScript, distinct from the binding crates.
- **FR-005**: The repository MUST provide a reserved, clearly-marked Go placeholder with no Go
  toolchain wired and no build expectation.
- **FR-006**: A single orchestrated command MUST build and test all active workspace members; the Go
  placeholder MUST be excluded from it.
- **FR-007**: The bootstrap's original flat `packages/{python,typescript,go,rust}` skeleton MUST be
  replaced by this layout, leaving no orphaned duplicate.

**Schema**

- **FR-008**: The repository MUST contain a prompt-definition JSON Schema, with a stable identifier,
  that is itself a valid JSON Schema.
- **FR-009**: The schema MUST express a prompt's role constrained to `system`, `user`, or `assistant`.
- **FR-010**: The schema MUST express, at the prompt-definition root: a required `name` (the logical
  prompt's reference key), a required `role`, a default template `body`, a `variables` block, an
  optional output-model reference, and an optional named `variants` map. It MUST NOT define any model
  field (the library records no model — Clarifications).
- **FR-010a**: The `variables` block MUST express, per variable: a name, a type, JSON-Schema
  validation constraints (e.g. `format`, `minimum`, `pattern`, `enum`), and a provenance tag
  constrained to `trusted`, `untrusted`, or `external` — rich enough for a later spec to generate a
  typed Vars model from it (generate-then-extend).
- **FR-011**: The schema MUST model the default variant structurally: the root `body` IS the default
  arm. There MUST be no `default:` marker. The default arm is surfaced under the reserved name
  `default` with an `is_default: true` indicator. A definition therefore can never have zero or two
  defaults.
- **FR-011a**: Each `variants` entry MUST be an object carrying its own `body` (the only field that
  differs per variant) and an optional, free-form, **library-opaque** `meta` object; the root
  (default) arm MAY likewise carry `meta`. Role, `variables`/provenance, and the output-model
  reference are shared across all arms and MUST NOT be redefined per variant.
- **FR-011b**: The schema MUST reject a `variants` entry literally named `default` (collision with the
  structural default arm).
- **FR-011c**: The schema MUST treat each variant's `meta` as opaque free-form data — it carries no
  schema-enforced selection semantics (weight, group, etc. are conventions the caller interprets, not
  the library).
- **FR-012**: The schema MUST be expressive enough to represent every prompt-definition field the
  roadmap names for v1, even where no consumer uses the field yet, to avoid later churn of the single
  source of truth.
- **FR-013**: The repository MUST include example prompt-definition documents — both well-formed
  (accepted) and malformed (rejected) — that exercise the schema's constraints and serve as the
  validation fixtures. Coverage MUST include: a single-body (no `variants`) definition; a multi-variant
  definition; an invalid role; an invalid provenance tag; a `variants` entry named `default`
  (rejected, FR-011b); and a variant attempting to redefine role/variables (rejected, FR-011a).

**Codegen**

- **FR-014**: A codegen step MUST generate a Python shape, a TypeScript shape, and a Rust shape from
  the JSON Schema, each faithfully representing the prompt-definition structure.
- **FR-015**: Codegen MUST be deterministic: regenerating from an unchanged schema MUST produce
  byte-identical output.
- **FR-016**: Generated artifacts MUST be committed to the repository, marked as generated, and live
  at predictable per-language locations segregated from hand-written code.
- **FR-017**: Codegen MUST be runnable via a single command and wired into the build pipeline.

**CI guardrails**

- **FR-018**: CI MUST fail if the engine-kernel crate or the Rust consumer crate gains a dependency on
  any FFI binding crate (e.g., `pyo3`, `napi`).
- **FR-019**: CI MUST fail if any committed generated shape does not match what regenerating from the
  current schema would produce (codegen-freshness), including partial regeneration.
- **FR-020**: Each guardrail failure MUST produce a message identifying the violated invariant and its
  location.

**Scope boundary (negative requirements)**

- **FR-021**: This spec MUST NOT introduce any rendering, validation, agreement-check,
  variant-resolution, or hashing logic, nor any template-engine integration or typed-Vars facade —
  those belong to specs 002+.
- **FR-022**: The library MUST NOT perform any I/O, LLM calls, request-body assembly, token counting,
  or output parsing in this spec (and by the constitution, ever) — the foundations introduce none of
  these.

### Key Entities *(include if feature involves data)*

- **Prompt Definition**: the schema-defined shape of a single prompt — its role, a default template
  `body`, a `variables` block, an optional output-model reference (opaque string), optional opaque
  metadata, and an optional `variants` map. The contract from which all per-language shapes are
  generated. No model field. (Defined here; consumed in later specs.)
- **Variant**: a named alternative within a prompt definition, differing **only** in template `body`
  (role, variables/provenance, and output-ref are shared). The root `body` is the implicit `default`
  arm (`is_default: true`); `variants:` holds named override objects. Each arm (default included) may
  carry an opaque `meta` object for caller-driven selection.
- **Variable declaration**: a `variables`-block entry — name + type + JSON-Schema constraints +
  provenance tag (`trusted` | `untrusted` | `external`) — rich enough to generate a typed Vars model
  (Pydantic/Zod/garde) from in a later spec (generate-then-extend). (Shape defined here; generation is
  a later spec.)
- **Selection metadata (`meta`)**: optional, free-form, **library-opaque** data on any arm. The
  library stores and exposes it (and the ordered arm list) but never interprets it; the caller uses it
  for round-robin / A/B / grouping / programmatic selection.
- **Workspace member**: a crate or package in the layout (engine kernel, Rust consumer, Python
  binding, TypeScript binding, package wrappers, reserved Go placeholder), each with a defined role
  and dependency direction.
- **Generated shape**: a per-language artifact (Python / TypeScript / Rust) produced from the schema;
  committed, marked generated, and freshness-checked.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A contributor can clone the repository and build every active workspace member with a
  single orchestrated command, with zero manual per-crate setup steps.
- **SC-002**: 100% of the well-formed example prompt-definition documents validate successfully
  against the schema, and 100% of the malformed examples are rejected.
- **SC-003**: Running codegen twice on an unchanged schema produces zero diff (deterministic), and a
  single-field schema change is reflected in all three language artifacts after one regeneration.
- **SC-004**: Introducing an FFI dependency into the engine kernel or Rust consumer crate causes CI to
  fail; a clean tree passes — verifiable by toggling the dependency in a scratch branch.
- **SC-005**: Hand-editing a generated shape, or editing the schema without regenerating, causes the
  codegen-freshness CI check to fail; a freshly-regenerated tree passes.
- **SC-006**: The schema represents every v1 prompt-definition field named in the roadmap, such that
  specs 002–007 can proceed without modifying the schema's field set for reasons known today.
- **SC-007**: No prompt rendering, validation, or engine behavior exists after this spec — confirming
  the spine is purely structural (verifiable by the absence of those code paths and the passing of
  the negative-scope review).

## Assumptions

- **Codegen tooling is a planning decision, not a spec decision.** The specific generators for each
  language (Python / TypeScript / Rust) are deliberately unspecified here and will be selected and
  pinned during `/speckit.plan`, verified against current (2026) tooling — this is roadmap Open
  Question Q1. The spec constrains only the *outcome* (deterministic, schema-faithful, committed,
  freshness-checked), not the tool.
- **"Published" schema means a committed schema with a stable identifier**, not publication to an
  external registry or URL endpoint. External publication (if ever) is out of scope and would be
  separate.
- **Generated shapes are checked into the repository** (not generated only at build time); the
  freshness CI check exists precisely because they are committed. This is the standard pattern that
  makes the "give data OR give an object, same loader" promise verifiable.
- **Stub crates are acceptable**: workspace members may contain minimal/no logic in this spec — the
  requirement is that they build and have correct dependency directions, not that they do anything.
- **Go is reserved only**: a placeholder directory/marker, no `go.mod`, no toolchain, excluded from
  the orchestrated build and from CI's language expectations.
- **The orchestration tool (moon) and Cargo workspace are pre-existing** from the bootstrap; this spec
  wires them to the new layout rather than introducing new orchestration technology.
- **Registry-name reservation (crates.io / PyPI / npm) is out of scope for this spec** — it belongs to
  the v1 release spec (007); 001 only establishes local crate/package names.
- **All three language shapes (Python/TS/Rust) are generated in 001, not just Python** (decision on
  critique finding X1, 2026-06-25). Although the TS/Rust *bindings* don't exist until specs 004/005,
  generating all three now is the faithful demonstration of C-07 ("one schema, N languages") and C-01
  (structural parity — the project's differentiator), and gives the codegen-freshness gate real
  cross-language coverage from day one, de-risking the pipeline before three binding specs depend on
  it. The generated TS/Rust shapes land in skeleton packages with no consumer yet — accepted.
