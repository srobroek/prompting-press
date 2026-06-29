# Implementation Plan: Pre-publish API & schema reshape

**Branch**: `008-api-schema-reshape` | **Date**: 2026-06-28 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/008-api-schema-reshape/spec.md`

## Summary

The last pre-publish reshape of the public contract, delivered as ONE coordinated change driven from the JSON
Schema (the single source of truth). Three bundles: (1) rename the per-variable `provenance` tag to `origin`;
(2) move the schema fixtures into a `tests/` subtree; (3) replace the registry-keyed free-function surface with
a first-class **immutable `Prompt`** object in all three bindings (validating construction, `with`-overlay as the
sole mutator, `fromYaml`/`fromJson`/`fromToml` text factories, render/getSource/check on the object, Composition
over objects, Registry dropped). The reshape also adds a per-variable `validation_required` boolean with
construction-time validator binding (constitution v1.2.0, Principle VI), raises the Python floor to 3.12, and
switches the TS shape from a generated `interface` to a generated **Zod schema** for a runtime enforcer. The
**kernel is not touched** (Principle I): rendering, agreement, variant resolution, and hashing are reused
verbatim, so cross-binding output parity remains structural and is re-proven only by the existing conformance
corpus.

## Technical Context

**Language/Version**: Rust 1.95.0 (pinned via mise); Python **≥3.12** (raised from 3.10 this spec — see
research R5); TypeScript 5.9.2 / Node 20+ (ESM). MiniJinja 2.21 (kernel, unchanged).

**Primary Dependencies** (existing, unchanged): PyO3 0.29 (`abi3-py312` — raised from `abi3-py310`), napi-rs
3.9.4 / napi-derive 3.5.7, Zod 4.4.3, garde (Rust validator), Pydantic v2, serde. **Codegen toolchain**:
cargo-typify 0.7.0 (Rust), datamodel-code-generator (Python/Pydantic v2), and — CHANGED — `json-schema-to-zod`
replacing `json-schema-to-typescript` 15.0.4 for the TS shape. **New (this spec)**: a TOML parser per binding
(Rust `toml`; Python stdlib `tomllib`, free at the 3.12 floor; a Node ESM TOML parser). Exact pins in the
Dependency Pins table below (filled from research R1–R4).

**Storage**: N/A (Principle III — the library does no I/O; prompt data is pushed in).

**Testing**: `cargo test` (kernel + consumer + both addon crates), `pytest` (Python), `node --test` (TS), the
JSON-Schema fixture validator (`validate_fixtures.py`), and the cross-language conformance corpus
(`conformance/`). All are existing CI gates that MUST stay green.

**Target Platform**: cross-platform library; three published packages (crates.io / PyPI / npm).

**Project Type**: polyglot library monorepo (moon-orchestrated), kernel + Rust consumer + 2 FFI bindings + their
language facades.

**Performance Goals**: N/A — no new hot path; the reshape is structural. The kernel's render path is unchanged.

**Constraints**: byte-stable codegen (the determinism freshness gate); exact version pins (no floating ranges);
no FFI deps in kernel/consumer; the conformance corpus must continue to prove byte-identical hashes across
bindings; SEC-004 (no bound-value leakage in errors) holds across the new construction/`with` paths.

**Scale/Scope**: ~174 files touch the `provenance` token alone; the full reshape additionally rewrites the
public surface of the consumer + both bindings + their facades + all examples + the conformance runners.

## Constitution Check

*GATE: re-checked after Phase 1 design (below). Constitution is at **v1.2.0** — the Principle VI amendment that
this spec depends on is already ratified (DECISIONS.md, 2026-06-28); this plan VERIFIES alignment, it does not
re-request the amendment.*

| Principle | Gate | Verdict |
|-----------|------|---------|
| **I. Shared Core, Structural Parity** | Kernel rendering/agreement/hashing unchanged; parity stays structural | ✅ PASS — only the generated shape's field name changes (via codegen); `provenance.rs` is renamed to `origin` vocabulary but its logic is byte-for-byte the same; no algorithm touched. The conformance corpus re-proves parity. |
| **II. FFI Isolation** | No `pyo3`/`napi` in kernel or consumer | ✅ PASS — `Prompt` wrapper in the consumer uses no FFI; the per-binding `Prompt` facades live in the binding crates/facades. `ci:check-ffi` unchanged and must stay green. |
| **III. Minimal Boundary** | No I/O, LLM, request-body assembly, token counting, output parsing | ✅ PASS — `Prompt`/`with`/`fromToml` add no I/O (text is pushed in by the caller); validators run in-process; the TOML parser only parses caller-supplied text, same as the existing YAML/JSON loaders. |
| **IV. Typed Input Is the Differentiator** | Agreement check sound; pure analysis | ✅ PASS — the agreement check moves *onto* construction (stricter: un-analyzable bodies now fail at construction) using the unchanged kernel `required_roots`; `check()` stays pure analysis, demoted to advisory. |
| **V. Repo Canonical; Git Owns Versioning** | No managed version axis; provenance = data on the return value | ✅ PASS — no version axis added; the render-result provenance (`template_hash`/`render_hash`) is explicitly NOT renamed (only the per-variable trust tag is). |
| **VI. Per-Language Idiom** | Native validators; normalized errors; options-object shape; **v1.2.0 validator binding** | ✅ PASS — Pydantic/Zod/garde retained; validators bound at construction; per-variable `validation_required` with the asymmetric enforcement codified in v1.2.0 (TS/Python throw at construction, Rust compile-time/structural); errors normalized to `[{field,code,message}]`, no native error type crosses FFI; `new Prompt` throws (TS idiom). |
| **VII. JSON Schema Single Source** | Shapes codegen'd, never hand-mirrored; conformance guards FFI + schema round-trip | ✅ PASS — the rename + the new `validation_required` field are made IN THE SCHEMA and regenerated into all 3 shapes; the codegen-freshness gate enforces it; conformance re-runs over the moved fixtures. |
| **Scope Discipline (C-08)** | No unearned pluggable seam | ✅ PASS — the Registry is DROPPED (not generalized); a query-capable registry stays a Deferred wishlist. No new seam introduced. |

**No violations.** Complexity Tracking table is empty.

## Project Structure

### Documentation (this feature)

```text
specs/008-api-schema-reshape/
├── plan.md              # This file
├── spec.md              # Feature spec (clarified)
├── memory.md            # Feature memory notes
├── memory-synthesis.md  # Planning synthesis
├── research.md          # Phase 0 — dependency pins + the with/validator semantics (this command)
├── data-model.md        # Phase 1 — Prompt object model, schema deltas (this command)
├── quickstart.md        # Phase 1 — per-language validation scenarios (this command)
├── contracts/           # Phase 1 — the Prompt surface contract per binding + the schema delta
└── tasks.md             # Phase 2 (/speckit.tasks — NOT created here)
```

### Source Code (repository root) — areas this reshape touches

```text
schemas/jsonschema/
├── prompt-definition.schema.json     # B1: provenance→origin; B2-adjacent: + validation_required
├── tests/fixtures/{valid,invalid}/   # B2: MOVED here from fixtures/{valid,invalid}/
└── scripts/validate_fixtures.py      # B2: fixture-path ref updated

crates/prompting-press-core/          # KERNEL — behavior unchanged
├── src/generated/prompt_definition.rs  # B1: regenerated (origin + validation_required)
└── src/provenance.rs → origin.rs?      # B1: renamed to origin vocabulary (logic identical)

crates/prompting-press/               # Rust CONSUMER
├── src/registry.rs                   # B3: Registry removed; loaders move under Prompt factories
├── src/prompt.rs                     # B3: NEW — the Rust `Prompt` (Facade over generated struct)
├── src/render.rs / check.rs / compose.rs  # B3: render/check/compose move onto/around Prompt
└── src/lib.rs                        # B3: re-exports reshaped (Prompt in, Registry out)

crates/prompting-press-py/  + packages/python/   # Python binding + facade
crates/prompting-press-node/ + packages/typescript/   # TS binding + facade (+ Zod codegen)
packages/python/scripts/codegen.sh    # B1+R5: --target-python-version 3.12
packages/typescript/scripts/codegen.mjs # B3: json-schema-to-zod (was json-schema-to-typescript)
conformance/                          # B1+B2: field rename + fixture-path refs in manifest
```

**Structure Decision**: The existing four-crate + two-facade layout is unchanged (it is constitution-enforced).
This reshape edits within it; the only *new* source files are the per-binding `Prompt` (Rust `prompt.rs`, a
Python pyclass, a TS class) and the relocated fixture tree. No new crate or package.

## Implementation Phasing (dependency-ordered — informs `/speckit.tasks`)

The schema is the source of truth, so the change flows strictly outward; each stage's gate must pass before the
next:

1. **Schema + codegen** (B1 rename + `validation_required` add; B2 fixture move; R5 Python-floor bump). Edit the
   schema, regenerate all 3 shapes, move fixtures + fix the 3 refs, bump the Python floor. Gate: `check-schema`,
   `validate-fixtures`, `codegen-check` (determinism) all green.
2. **Kernel rename** — `provenance.rs` → `origin` vocabulary (`ProvenanceView`→`OriginView`,
   `provenance_view`→`origin_view`, `VariableDeclProvenance`→the regenerated enum). Behavior identical. Gate:
   `cargo test -p prompting-press-core`, `ci:check-ffi`.
3. **Rust consumer** — introduce `Prompt` (Facade), migrate render/getSource/check onto it, implement
   `with(overlay)`, drop `Registry`, fold the dual-input loaders into `from_yaml`/`from_json`/`from_toml`.
   Gate: `cargo test -p prompting-press`.
4. **Python binding + facade** — `Prompt` pyclass, validators bound at construction (Pydantic), per-variable
   coverage throw, `fromToml` via stdlib `tomllib`. Gate: `cargo test -p prompting-press-py` + `pytest`.
5. **TS binding + facade** — Zod-schema codegen, `Prompt` class (`new` throws), validator coverage via
   `schema.shape`, `fromToml` via the pinned Node parser. Gate: `cargo test -p prompting-press-node` +
   `node --test`.
6. **Conformance + docs + examples** — field rename + fixture paths in the conformance manifest/runners; sweep
   every doc/README/quickstart for the old surface (per the spec-005 mid-amendment lesson). Gate:
   `ci:conformance`, full CI.

## Dependency Pins (verified main-thread 2026-06-28 — see research.md)

All pins below were verified by **direct registry/source fetch** (npm registry, crates.io, the upstream
README), NOT via a research subagent — two subagent attempts fabricated contradictory numbers and are
discarded (research.md §Provenance). The strict no-floating-version gate requires exact pins.

| Dependency | Role | Pin | Key fact (verified) |
|-----------|------|-----|---------------------|
| `json-schema-to-zod` | TS codegen (replaces `json-schema-to-typescript`) | **2.8.1** | CLI `-i/-o` emits committed `.ts`; **`--zodVersion` defaults to `4`**; `--type` exports the `z.infer` type; `--module esm`; dev-tested against `zod ^4.1.3`. |
| `zod` | per-variable coverage introspection + runtime enforce | **4.4.3** (already pinned) | `ZodObject.shape` → field-name record; `name in schema.shape` is the coverage check. |
| Rust `toml` | Rust `Prompt::from_toml` | **1.1.2** | serde-native (`toml::from_str::<T>()`); the crate is at 1.x (NOT 0.8.x). |
| Node TOML parser `smol-toml` | TS `Prompt.fromToml` | **1.7.0** | native ESM (`type: module`, Node ≥18), TS types bundled, zero deps; `parse(str)`. |
| Python `tomllib` | Python `Prompt.from_toml` | stdlib @ 3.12 | free at the raised 3.12 floor (decision [A8-5]); no `tomli` backport dep. |

**Determinism note (TS codegen freshness gate):** `json-schema-to-zod` ships no bundled formatter (unlike
`json-schema-to-typescript`'s prettier). The codegen script (`codegen.mjs`) must therefore run the output
through the project's pinned Biome (already a dep) — or a fixed normalization pass — to keep the committed file
byte-stable, exactly as the existing script normalizes newlines + banner. Validate the twice-run byte-identical
check in the schema+codegen task (Phase 1 gate).

**Schema-vocabulary caveat:** the prompt-definition schema is Draft 2020-12 but uses only `type`/`properties`/
`required`/`enum`/`additionalProperties`/`$defs`/`$ref`/`propertyNames`/`oneOf` — all within
`json-schema-to-zod`'s supported set. The one known sharp edge is `variants.propertyNames` (the reserved-
`default` rule), which already requires a `jq` strip for Rust's `cargo-typify`; verify whether the Zod codegen
also needs it stripped (likely yes — `propertyNames` with a `not`/`const` is niche). Capture in research.md R1.

## Post-Design Constitution Re-Check

Re-evaluated after Phase 1 (research + data-model + contracts + quickstart). **Still no violations.** The
design introduces no I/O, no new pluggable seam, no kernel change; it reuses `required_roots` for the
construction-time agreement check (Principle IV/I), keeps validators in the binding layer (Principle II/VI), and
regenerates all shapes from the schema (Principle VII). The one governance dependency — the Principle VI
expansion — is already ratified (v1.2.0) and the design conforms to it (validators bound at construction;
asymmetric coverage enforcement). Complexity Tracking remains empty.

Two design facts worth flagging for `/speckit.tasks` (not violations): (a) the TS Zod codegen needs an explicit
deterministic-format step (no bundled formatter) to satisfy the freshness gate; (b) the `variants.propertyNames`
`jq` strip may need extending to the Zod codegen. Both are tasks, not constitutional issues.

## Complexity Tracking

*No constitution violations — table intentionally empty.*
