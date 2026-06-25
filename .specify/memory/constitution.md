<!--
SYNC IMPACT REPORT
==================
Version change: (template / unversioned) → 1.0.0
Rationale: Initial ratification. First concrete constitution, replacing the
  placeholder template. MAJOR baseline (1.0.0) per semantic-versioning of a
  newly adopted governance charter.

Principles defined (7):
  I.   Shared Core, Structural Parity
  II.  FFI Isolation
  III. Minimal Boundary (NON-NEGOTIABLE)
  IV.  Typed Input Is the Differentiator
  V.   Repo Is Canonical; Git Owns Versioning
  VI.  Per-Language Idiom Over Forced Uniformity
  VII. JSON Schema Is the Single Source of Truth

Added sections:
  - Scope Discipline (Section 2) — R1 outcome, eliminated interfaces, v1 surface
  - Development Workflow & Quality Gates (Section 3)
  - Governance

Removed sections: none (template placeholders all filled)

Templates requiring updates:
  ✅ .specify/memory/constitution.md (this file)
  ⚠ .specify/templates/plan-template.md — verify Constitution Check aligns
     (FFI-isolation, no-I/O boundary, conformance-corpus scope). Pending review.
  ⚠ .specify/templates/spec-template.md — no mandatory-section change required;
     confirm at first /speckit.specify. Pending review.
  ⚠ .specify/templates/tasks-template.md — ensure task categories cover codegen
     pipeline + conformance corpus + agreement-check lint. Pending review.

Deferred / TODO:
  - RATIFICATION_DATE set to 2026-06-25 (today) — first adoption; no prior date.
  - Go binding deferred (Principle I); revisit trigger documented in roadmap.
-->

# Prompting Press Constitution

Prompting Press is a typed, versioned, variant-aware **prompt-template** library — the prompt
analogue of a typed config system. It parses, validates, and renders prompt text and stamps
provenance, across Python, TypeScript, and Rust from one shared engine. This constitution is the
durable charter; it supersedes ad-hoc practice. Principles use MUST / MUST NOT / SHOULD with the
RFC-2119 sense and are written to be testable.

## Core Principles

### I. Shared Core, Structural Parity

One compiled Rust engine (`prompting-press-core`) is the single source of rendering behavior. Every
language binds that same engine; rendering, the agreement analysis, variant resolution, and hashing
are performed **once, in Rust**. Cross-language output equality is therefore a **structural property
by construction, not a behavior re-verified by tests** in each language.

- The engine kernel MUST NOT be reimplemented per language. A per-language reimplementation (e.g.
  an independent Go port) forfeits structural parity and is out of bounds for any binding claiming
  byte-identity.
- v1 ships bindings for **Rust, Python, and TypeScript**. **Go is deferred** — it has no native FFI
  that shares the Rust binary; it returns only via a verified cgo/WASM binding to the same core,
  never via reimplementation.

*Rationale:* parity that is structural cannot drift; parity that is test-enforced eventually does.
This is the entire reason for a Rust core over N native libraries.

### II. FFI Isolation

`pyo3` and `napi` are binding-layer dependencies and appear **only** in `prompting-press-py` and
`prompting-press-node` respectively.

- `prompting-press-core` (engine kernel) and `prompting-press` (Rust consumer crate) MUST NOT depend
  on `pyo3`, `napi`, or any FFI binding crate — verifiable by inspecting their `Cargo.toml`.
- The engine kernel is **binding-agnostic and validation-blind**: it receives already-validated
  values and knows nothing of Pydantic/Zod/garde, Python, or Node.
- Each binding crate contains **only** marshaling plus its typed-Vars facade — no rendering,
  agreement analysis, variant resolution, or hashing logic.

*Rationale:* an FFI dep in the core drags CPython/Node linkage into every consumer (including
pure-Rust users and the engine's own tests) and couples the kernel to one runtime. Verified standard
practice (polars, oxc, biome).

### III. Minimal Boundary (NON-NEGOTIABLE)

The library turns *typed inputs + a template* into *rendered text + provenance*. Nothing else.

- It MUST NOT perform I/O: no file reads, no database/Redis/S3/network access, no storage layer. The
  caller **pushes** prompt data in.
- It MUST NOT call an LLM, assemble a provider request body (the `system`/`messages` split, content
  blocks), count tokens, or parse/coerce model output.
- Token counting is exposed only as a pluggable `count_tokens(text, model) -> int` **hook**; no
  built-in estimate ships (accurate offline multi-vendor counting does not exist, and a built-in
  estimate is most wrong for the primary consumer's model).
- The output-model is carried as a metadata **reference** only; the library never parses against it.

*Rationale:* every capability outside this boundary is per-vendor, I/O-bound, or framework-coupled —
exactly the things that destroy reusability. A narrow boundary is what keeps the library orthogonal
to LangChain/OpenRouter/any call layer.

### IV. Typed Input Is the Differentiator

The headline guarantee is the **sound agreement check**: a template's referenced variables MUST be a
subset of its declared, typed Vars-model fields, caught as an error rather than a silent empty
render.

- The check is computed over the engine's AST via MiniJinja's stable `Template::undeclared_variables`
  with **`nested = false`** (root variable names; deep field shape is the type system's job) minus a
  known globals/filters allowlist. It MUST exclude loop locals, `{% set %}` targets, and block
  locals (it is strictly more sound than `jinja2.meta`).
- The check runs as a **CI/lint pass** (`check(registry)`); it is pure analysis and **MUST NOT
  mutate** templates, vars, or output.
- Soundness is preserved by **excluding `{% include %}` / `{% import %}` / `{% extends %}`, macros,
  and inheritance** from v1 templates (these would force the unstable AST API and a cross-template
  graph walker). v1 template features are **interpolation, conditionals, and loops** only.

*Rationale:* this is the BAML-equivalent static guarantee no file-based library provides, and it is
the verified reason this library exists. Cutting includes is what keeps the guarantee airtight with
zero unstable dependencies.

### V. Repo Is Canonical; Git Owns Versioning

Prompts are in-repo, PR-gated artifacts. The library does **not** reimplement git.

- There MUST be no managed version axis: no `versions: {}` storage, no `version=` pin in the render
  API. Evolution, history, and pinning are git's (and the consuming app's deploy's) responsibility.
- **Variants** are named, parallel, coexisting alternatives — the one multi-template need git cannot
  express. **Selection is caller-owned** (`render(name, variant="...")`); the library validates the
  name, renders it, and stamps it. It MUST NOT own experiment-assignment logic or a deterministic
  selector. A prompt with no variants has an implicit `default`; a multi-variant prompt MUST declare
  an explicit default or a no-variant render is an error.
- Provenance is **data on the return value** (no telemetry sink, no OTel coupling) and carries two
  hashes, **per resolved variant, each over a string**: `template_hash = SHA256(variant template
  source)` and `render_hash = SHA256(rendered output)`. There is no `vars_hash`.

*Rationale:* in a git-canonical design a managed version axis is reinventing git, badly.
`template_hash`/`render_hash` supply the only property git can't surface at render time — content
identity in a trace — for free, because the shared core makes the rendered string byte-identical
across languages.

### VI. Per-Language Idiom Over Forced Uniformity

The *capability* is uniform across languages; the *idiom* is native and may differ — and that is
correct, not a defect.

- Typed Vars and custom validators use each language's native system: **Pydantic** (Python), **Zod**
  (TypeScript), **garde** (Rust). Validators are attached to fields and run in one `validate()` at
  render. The library MUST NOT invent its own validation framework.
- Composition of multi-message prompts is an **explicit ordered array** of `(prompt-ref, vars)`
  resolving to `[{role, text}, ...]`, with native construction sugar per language. A fluent
  `.chain()` MUST NOT be the API (it cannot cross PyO3/napi and collides with `Iterator::chain`).
- Validation and error types MUST be **normalized to a common structured shape**
  (`[{field, code, message}]`) at each consumer boundary; native error types (garde `Report`,
  Pydantic/Zod errors) MUST NOT leak across FFI.

*Rationale:* forcing one shape across three ecosystems produces an alien API in at least two of them.
Uniform capability + native idiom is what makes each binding feel first-class.

### VII. JSON Schema Is the Single Source of Truth

The prompt-definition shape is defined **once** as a published JSON Schema.

- Per-language prompt-definition shapes (Pydantic models / TS types / Rust structs) MUST be
  **code-generated** from that schema at build time — never hand-maintained in parallel. (The
  agreement check covers template↔Vars, not schema↔shape; codegen is what prevents schema↔shape
  drift.)
- Prompt data is **pushed in as YAML or JSON** (one data model, one internal representation) or as a
  constructed shape object; a single dual-input loader normalizes both. Programmatic/code-defined
  prompts are first-class but language-local by nature; canonical shared prompts use the portable
  YAML/JSON form.
- The conformance corpus guards the **FFI boundary** (identical marshaling of dates, decimals, nested
  models, null/undefined, int/float → identical render and hashes) and **schema round-trip**
  (identical accept/reject across languages), **not** render parity (which Principle I makes
  structural). A small render-fixture set remains an engine regression guard only.

*Rationale:* with one schema and N generated shapes, "give data OR give an object, same loader" holds
without drift, and a prompt doc is language-neutral — so the cross-language definition fork does not
exist.

## Scope Discipline (R1)

v1 ships **one concrete path per concern and no speculative extension points.** The original design
brief proposed five pluggable interfaces; all five are eliminated as v1 public seams:

- **Store** → eliminated (push model; the library does no I/O — Principle III).
- **Loader** → eliminated as a user interface (the library parses its own schema; dual-input is
  internal — Principle VII).
- **VariantSelector** → eliminated (selection is caller-owned — Principle V).
- **ProvenanceSink** → eliminated (provenance is data on the return value — Principle V).
- **Type system** → not a core seam; it lives in the consumer layer (Principle VI).

A new pluggable interface MUST NOT be introduced until a second concrete implementation actually
exists to exercise it. Generality is earned by a real second consumer, not anticipated. The library's
surface stays concentrated on its one verified differentiator: typed input + the sound agreement
check.

## Development Workflow & Quality Gates

- **Crate layout is load-bearing and enforced**: `prompting-press-core` (kernel),
  `prompting-press` (Rust consumer), `prompting-press-py`, `prompting-press-node`. CI MUST fail if a
  forbidden FFI dependency appears in the kernel or Rust consumer crate (Principle II).
- **Codegen runs in the build pipeline** for all three packages (orchestrated by moon); a schema
  change that is not regenerated into the per-language shapes is a build failure (Principle VII).
- **The agreement check and the provenance lint are CI gates**, not optional tooling; a template
  whose referenced vars exceed its declared Vars fields, or that uses an untrusted-tagged field
  outside a declared guard position, fails CI (Principles IV; var-provenance is metadata + lint +
  opt-in, never silent mutation).
- **The conformance corpus is a CI gate** scoped to FFI marshaling and schema round-trip (Principle
  VII); cross-language render parity is assumed structural and is not the corpus's burden.
- Feature work follows the SpecKit workflow (spec → clarify → plan → tasks → implement → verify);
  spec artifacts are authored via the SpecKit skills, never by hand.

## Governance

This constitution supersedes ad-hoc practice. It is the durable charter: light edits are expected
after the first feature lands; major rewrites should be rare and recorded in
`.specify/memory/DECISIONS.md`.

- **Amendments** require: a written rationale, a version bump per the policy below, and propagation to
  dependent artifacts (`plan-template.md`, `spec-template.md`, `tasks-template.md`, and any runtime
  guidance docs). A removed or redefined principle MUST note the migration in `DECISIONS.md`.
- **Versioning policy** (semantic): **MAJOR** = a principle removed or redefined in a
  backward-incompatible way; **MINOR** = a principle/section added or materially expanded; **PATCH** =
  clarifications and non-semantic refinements.
- **Compliance review**: every plan's Constitution Check and every PR review MUST verify adherence to
  these principles. A deviation MUST be justified in writing or the change MUST be revised; complexity
  that violates Scope Discipline MUST be removed or explicitly ratified by amendment.
- **Boundary defense**: any proposal to add I/O, LLM calls, request-body assembly, token counting,
  output parsing, a managed version axis, or a new pluggable interface is presumed out of scope and
  requires an amendment (with rationale) before work begins.

**Version**: 1.0.0 | **Ratified**: 2026-06-25 | **Last Amended**: 2026-06-25
