# Quickstart / Validation Guide: Foundations

How to prove spec 001 works end-to-end. This is a **validation guide**, not implementation —
commands and expected outcomes only. Maps to the spec's user stories (US1 layout, US2 schema, US3
codegen, US4 CI guardrails) and success criteria (SC-001..007).

> Tooling fixed in Phase 0 research (`research.md`): `datamodel-code-generator` 0.65.1 (Python),
> `json-schema-to-typescript` 15.0.4 (TS), `cargo-typify` 0.7.0 (Rust). Contract:
> `contracts/prompt-definition.schema.json` (impl copy at `schemas/jsonschema/`).

## Prerequisites

- Rust toolchain pinned via `rust-toolchain.toml` (determinism for `cargo-typify`)
- Node + the pinned codegen/format tools; Python + uv with pinned `datamodel-code-generator`
- moon (already bootstrapped)

## US1 — Buildable polyglot workspace (SC-001)

```
moon run :build        # or the orchestrated build task across active members
```
Expected: `prompting-press-core`, `prompting-press`, `prompting-press-py`, `prompting-press-node`
all build (as stubs). `packages/go` is NOT built (reserved placeholder, excluded). Dependency
direction holds: `cargo tree -p prompting-press-core` shows no binding/FFI crate.

## US2 — Schema validates the right things (SC-002, SC-006)

```
# meta-validate the schema itself (FR-008)
<json-schema validator> --check schemas/jsonschema/prompt-definition.schema.json

# run the accept/reject fixtures (FR-013; see data-model.md matrix)
<validate fixtures against schema>
```
Expected: schema is a valid Draft 2020-12 document; 100% of well-formed fixtures accept, 100% of
malformed fixtures reject — including the multi-variant-with-named-`default` rejection (FR-011b) and
the variant-redefining-role rejection (FR-011a).

## US3 — Codegen is deterministic and faithful (SC-003)

```
moon run :codegen      # schema -> Pydantic / TS / Rust shapes
moon run :codegen      # run twice
git diff --exit-code -- <generated paths>   # expect: no diff (deterministic)
```
Expected: three generated shapes produced, each marked generated and in its predictable location;
re-running yields zero diff. Editing one schema field then regenerating changes all three shapes.

## US4 — CI guardrails fail on violation (SC-004, SC-005)

```
# FFI-isolation (FR-018): introduce a forbidden dep in a scratch branch
cargo add pyo3 -p prompting-press-core         # then:
mise exec -- moon run ci:check-ffi             # EXPECT: fail, cites Principle II / C-02

# codegen-freshness (FR-019): hand-edit a generated file or edit schema without regen
mise exec -- moon run schemas:codegen-check    # EXPECT: fail (git diff --exit-code over the 3 shapes)
```
(As-implemented task names. The clean tree also passes `mise exec -- moon run ci:check-floating-versions`
— the SEC-003 floating-version lint, and `schemas:check-schema` / `schemas:validate-fixtures` for US2.)
Expected: clean tree passes both gates; each violation fails with a message naming the invariant and
location (SC-004, SC-005). Revert the scratch change → green.

## Negative-scope check (SC-007)

Confirm no rendering/validation/engine/IO code exists: there is no template engine, no `render`, no
validation runtime, no I/O. The spine is purely structural.
