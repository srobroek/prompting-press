# Feature memory — 008 Pre-publish API & schema reshape

Feature-local notes prepared before `/speckit.specify` (memory-md `before_specify` hook). Source: direct-read
fallback over the governance layer + `docs/memory/` + project memory — the `speckit_memory_*` MCP tools and the
`.spec-kit-memory/` SQLite cache are NOT present this session (confirmed: no `.spec-kit-memory/` dir, no MCP
tools registered).

## What 008 is

The LAST pre-publish change to the public contract (blocks 007 v1-release and 010 docs-site). ONE coordinated
change, ~174 files (verified `rg -l provenance | wc -l` = 174). Three bundled changes, decided as ONE
object-model decision (research note `docs/research/registry-value-and-object-model.md` §8 + §9).

## Governing constraints (from the governance layer — must hold)

- **Principle I / C-01** — kernel rendering/agreement/hashing behavior is UNCHANGED. Cross-binding parity is
  STRUCTURAL (one Rust core), not re-tested per language. The object model lands in ALL THREE bindings.
- **Principle III** — minimal boundary: no I/O, no LLM, no request-body assembly. The reshape must not breach it.
- **Principle IV** — the sound agreement check is the differentiator. Construction enforces every *decidable*
  invariant; the un-analyzable residue stays a `check()` finding.
- **Principle VI / C-06 / C-11** — native validators (Pydantic/Zod/garde), errors normalized to
  `[{field,code,message}]` at each boundary (native error types MUST NOT leak across FFI); options-object /
  receiver call shape. The amendment (DECISIONS.md, v1.1.0) set per-language thresholds: TS/Python strict,
  Rust keeps a single `Option<T>` positional (struct only at 2+ optionals).
- **Principle VII / C-07** — JSON Schema is the single source of truth; the 3 per-language shapes are
  code-generated, never hand-mirrored. The conformance corpus guards FFI marshaling + schema round-trip.
- **C-08 (Scope Discipline)** — the registry keep/drop IS this discipline applied to the registry abstraction.
  Registry DROPPED from the object model; query-capable registry is a Deferred wishlist, gated on a real consumer.
- **C-09** — origin tag is DECLARATIVE metadata only; no runtime enforcement of the tag itself.

## Reused technical decisions (docs/memory/)

- **A1 (architecture) — three validity layers are NOT equivalent.** (1) JSON-Schema validation (strict:
  `validate_fixtures.py`), (2) binding loaders = serde shape-deserialization only (accept what serde can shape,
  reject unknown keys/bad enums), (3) `check()` = semantic rules the loader can't. Consequence for 008:
  `Prompt.fromYaml/fromJson` are the *loader* layer (serde shape), and the validating constructor must layer the
  decidable agreement check on top; the un-analyzable residue is `check()`. A `default`-named variant is
  schema-invalid but loader-ACCEPTED → caught by `check()`. The conformance manifest already excludes
  `variant-named-default` from the loader round-trip set with a documented note — **the fixture move (change 2)
  must preserve that manifest note + the exclusion.**
- **D1 (decision) — cross-binding parity tested via canonical serialized form.** date/decimal pinned by the
  serialized string the kernel sees, NOT a native `datetime`/`Date`/`Decimal` (they recanonicalize: Pydantic
  `Z`/`1E-17`, JS `Date` `.000Z`). Consequence for 008: when conformance fixtures move/are touched, do NOT
  "fix" a runner to build native objects — that breaks the gate by design.

## As-built surfaces 008 reshapes (project memory — verify before editing)

- **Kernel (spec 002)**: `provenance.rs` (`provenance_view`, `ProvenanceView`, `VariableDeclProvenance`,
  `GuardConfig`, `build_guard_text`). `render`/`get_source`/`required_roots`/`provenance_view` are the public
  kernel fns. `KernelError` is a CLOSED enum (keep exhaustive). Guard text is a SEPARATE `RenderResult.guard`.
  The generated `PromptDefinition` shape lives IN the kernel (`src/generated/prompt_definition.rs`), consumer
  re-exports. **Rename touches `VariableDeclProvenance` → the codegen'd enum name changes when the schema field
  renames.**
- **Rust consumer (spec 003)**: `registry.rs` (`load_json`/`load_yaml` = serde only), `check.rs`
  (`ReservedVariantName` + agreement + provenance findings), error normalization, `Composition`. The current
  `render<V: Serialize + Validate>` injects a garde validator (Strategy). The object model adds a Rust `Prompt`
  wrapping the generated struct with a generic `with`/`render::<V>`.
- **Python binding (spec 004)**: PyO3 0.29 + Pydantic, abi3-py310. Exception hierarchy via `create_exception!`
  (NOT `extends=PyException`). SEC-004 scrub: route `KernelError` through the consumer's `From` scrubber FIRST;
  Pydantic mapper copies `msg`/`loc` ONLY, never `input`/`ctx`. Composition binding-owned. render/compose call
  `prompting_press_core::render` DIRECTLY (kernel-direct, not the consumer generic).
- **TS binding (spec 005)**: napi-rs 3.9.4 + napi-derive 3.5.7 + Zod 4.4.3, ESM-only Node 20+. Rust-addon /
  TS-facade SPLIT — Zod can't live in Rust, so the napi crate marshals + delegates and the TS facade owns Zod
  `safeParse`-at-render + the `PromptingPressError` hierarchy + `decodeAddonError` (napi JSON → subclass). napi
  error transport = JSON `{code,errors:[...]}` in `napi::Error.reason`. Codegen currently emits a TS `interface`
  (`json-schema-to-typescript`) — OPEN: switch to Zod (`json-schema-to-zod`) for a runtime enforcer.

## Conflicts / things to watch

- **NO conflict** between the resolved object model and the governance layer — the registry-drop, immutability,
  and `with`-as-sole-mutator are consistent with C-08 + Principle VI. The reshape is explicitly the
  "last pre-publish" window the roadmap reserves.
- **`provenance` naming collision (must not over-rename):** the rename targets ONLY the per-variable
  `VariableDecl.provenance` input-trust tag. The render-result provenance concept (the `template_hash`/
  `render_hash` on the return value, Principle V) keeps its name. `rg provenance` hits BOTH; the spec must scope
  the rename precisely so the hash/return-value provenance is not renamed.
- **Examples repo** (`/Users/sjors/personal/dev/prompting-press-examples`) is a SEPARATE local repo, intentionally
  unpushed — NOT in 008's file count; do not touch from this repo.

## Open questions → carried to `/speckit.clarify` (do NOT resolve in the spec; flag [NEEDS CLARIFICATION])

1. TS codegen `interface` → Zod schema (runtime enforcer for the TS validating constructor). Strong lean YES.
2. `validation_required: true` schema boolean — ship in 008 or defer?
3. Confirm all-bindings appetite incl. Rust `Prompt` wrapping the generated struct (generic `with`/`render::<V>`).
4. Un-analyzable-template handling = build-succeeds / `check()` reports `AnalysisError` (lean YES).
5. `Prompt.fromToml(text)` — ship in 008 or defer? (cost = pinned TOML parser per binding).
6. TS constructor shape — `new Prompt({...})` THROWS on invalid vs `Prompt.create({...})` returns a result.

See [[spec-002-engine-kernel]], [[spec-004-python-binding]], [[spec-005-ts-binding]], [[speckit-workflow-gotchas]].
