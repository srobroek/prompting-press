# Quickstart / Validation — Spec 002 (Engine kernel)

How to prove the kernel works end-to-end. This is a **validation guide**, not implementation — the
real tests live in `crates/prompting-press-core/` (unit + a small fixture-backed regression set) and
run under the existing moon/CI pipeline. Scenario IDs map to the spec's user stories and SCs.

## Prerequisites

- Toolchain via `mise` (Rust `1.95.0` pinned; `cargo-typify 0.7.0`) — already configured (spec 001).
- The relocated generated shape compiles in the kernel (research D6): `mise exec -- cargo build -p prompting-press-core`.

## Build & test

```bash
# Kernel builds (incl. relocated generated shape)
mise exec -- cargo build -p prompting-press-core

# Kernel unit + regression-fixture tests
mise exec -- cargo test -p prompting-press-core

# Full workspace gate suite (must stay green — spec 001 gates + new kernel tests)
mise exec -- moon run :build
mise exec -- moon run schemas:codegen-check    # freshness gate, now pointing at the kernel path
mise exec -- moon run ci:check-ffi              # SC-007: kernel still FFI-free with minijinja+sha2 added
```

## Validation scenarios

### US1 — Render + provenance (P1)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V1.1 | Render single-body `"Hello {{ name }}"` with `{name:"Ada"}`, no variant | `text="Hello Ada"`, `variant="default"`, hashes present | FR-001, FR-007, FR-012/13 |
| V1.2 | Render the same twice | byte-identical `text` + equal hashes both times | FR-003, **SC-001** |
| V1.3 | Multi-variant prompt, render variant `"concise"` | renders `concise` arm, `variant="concise"`, its own `template_hash` | FR-008, FR-012 (per-variant) |
| V1.4 | Render unknown variant `"nope"` | `Err(UnknownVariant{requested:"nope"})` | FR-009, **SC-004** |
| V1.5 | Render `None` on a multi-variant prompt | renders root `body` as `default` (root body is always the default) | FR-007/011, research §contradiction |
| V1.6 | Conditional + loop template over a provided list | output reflects branch + iterations | FR-001 |
| V1.7 | Render `"Hello {{ name }}"` with `{}` (no `name`) | `Err(UndefinedVariable)`, NOT `"Hello "` | FR-001a, **SC-009** |
| V1.8 | `get_source(def, Some("concise"))` | returns the unrendered `concise` body; `SHA256` of it == `template_hash` from V1.3 | FR-006, FR-012 |

### US2 — Sound agreement analysis (P2)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V2.1 | `required_roots` of `"{{ greeting }}, {{ user.name }}"` | `{greeting, user}` (not `name`) | FR-016 |
| V2.2 | `required_roots` of `"{% for item in items %}{{ item }}{% endfor %}"` | `{items}` (not `item`) | FR-017, **SC-002** |
| V2.3 | `required_roots` of `"{% set x = 1 %}{{ x }}{{ y }}"` | `{y}` (not `x`) | FR-017, **SC-002** |
| V2.4 | `required_roots` of a template using `range`/`namespace` global | global name absent from result | FR-017 (env-derived allowlist) |
| V2.5 | After analysis, inspect inputs | `def`, values, output all unchanged | FR-018, **SC-006** |
| V2.6 | `required_roots` of a template referencing undeclared `foo` | `foo` appears in the set (detectable, not silent) | FR-016, **SC-003** |

### US3 — Provenance + guard expansion (P3)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V3.1 | `provenance_view` of `{q:untrusted, ctx:external, sys:trusted}` | `untrusted={q}`, `external={ctx}` | FR-021 |
| V3.2 | Render with `GuardConfig{enabled:false}` | `guard=None`, `text` == plain render | FR-022, **SC-005** |
| V3.3 | Render with `GuardConfig{enabled:true, template:None}` | `guard=Some(<default naming q, ctx>)`, `text` byte-identical to plain render | FR-022/23, **SC-005** |
| V3.4 | Render with `GuardConfig{enabled:true, template:Some("X {fields}")}` | `guard` uses the override text | FR-024 |
| V3.5 | Compare untrusted value before/after guard render | value passes through unchanged | FR-025 |

### Excluded features (edge / SC-008)

| Scenario | Action | Expected | Proves |
|---|---|---|---|
| V4.1 | Add/render a template with `{% include "x" %}` | `Err(ExcludedFeature\|Parse)` | FR-002, **SC-008** |
| V4.2 | `{% extends %}`, `{% import %}`, `{% macro %}`, `{% block %}` (one fixture each) | each errors at add/parse | FR-002, **SC-008** |
| V4.3 | `required_roots` on an excluded-feature template | `Err`, never an empty successful analysis | FR-028, research D2 |

## Fixture regression set (FR-029)

A small set of `(template, values) → expected output` pairs lives under
`crates/prompting-press-core/tests/fixtures/` (engine regression guard only — **not** cross-language
parity, which is structural per C-01). Reuse the spec-001 schema fixtures
(`schemas/jsonschema/fixtures/valid/*.json`) as prompt-definition inputs where useful (e.g.
`multi-variant.json` for V1.3/V1.5). Design the fixture loader so it does not match its own
`{{ … }}`-containing corpus when a gate greps (spec-001 lesson: self-referential-string false positives).

## Done = green

All scenarios pass, `moon run :build` + `schemas:codegen-check` + `ci:check-ffi` green, and the kernel
crate shows zero `pyo3`/`napi` in `cargo tree`. Cross-language parity is **not** validated here (no
bindings exist yet; parity is structural).
