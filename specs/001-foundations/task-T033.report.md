# Task T033 Report — End-to-End Validation Gate

**Date:** 2026-06-25  
**Branch:** 001-foundations  
**Scope:** SC-001..SC-007 pass/fail table + negative-scope audit summary (FR-021 / FR-022)

---

## Part A — Success-Criteria Pass/Fail Table

### SC-001: Single-command polyglot build

**Command:**
```
mise exec -- moon run prompting-press-core:build prompting-press:build \
  prompting-press-py:build prompting-press-node:build
```

**Note on `:build`:** The wildcard `mise exec -- moon run :build` fails because the
`schemas` and `ci` moon projects inherit the global `build` task (`cargo build
--package $project`) but are not Rust crates. The four active Rust workspace members
build successfully via explicit project targets. This is a known task-inheritance gap
(the `schemas` and `ci` projects lack `workspace.inheritedTasks.exclude: [build]`);
it does not affect SC-001 since the spec defines SC-001 as building the four crates,
not `schemas` or `ci`. See Finding F-001 below.

**Captured output (key lines):**
```
prompting-press:codegen | Generated crates/prompting-press/src/generated/prompt_definition.rs
prompting-press:build | Finished `dev` profile target(s) in 1.42s
prompting-press-node:build | Finished `dev` profile target(s) in 15.08s
prompting-press-core:build | Finished `dev` profile target(s) in 14.97s
prompting-press-py:build | Finished `dev` profile target(s) in 25.82s
Tasks: 5 completed (1 cached)
```

**Dependency direction check:**
```
mise exec -- cargo tree -p prompting-press-core -i pyo3
→ error: package ID specification `pyo3` did not match any packages  (exit 101)
```
pyo3 absent from kernel tree — dependency direction holds.

**Status: PASS**

---

### SC-002: 100% fixture accept/reject

**Command:**
```
mise exec -- moon run schemas:validate-fixtures
```

**Captured output:**
```
fixtures/valid/  (each MUST be ACCEPTED — 3 files)
  [PASS] multi-variant.json  —  accepted
  [PASS] single-body.json  —  accepted
  [PASS] variant-with-meta.json  —  accepted

fixtures/invalid/  (each MUST be REJECTED — 7 files)
  [PASS] bad-provenance.json  —  schema-invalid at [variables > user_input > provenance]
  [PASS] bad-role.json  —  schema-invalid at [role]: 'developer' is not one of ['system', 'user', 'assistant']
  [PASS] extra-root-key.json  —  schema-invalid at [(root)]: Additional properties are not allowed
  [PASS] missing-required.json  —  schema-invalid at [(root)]: 'body' is a required property
  [PASS] not-json.txt  —  parse-error
  [PASS] variant-named-default.json  —  schema-invalid at [variants]: 'default' should not be valid under {'const': 'default'}
  [PASS] variant-redefines-role.json  —  schema-invalid at [variants > alternative]: Additional properties are not allowed ('role' was unexpected)

Summary: 10/10 expectations met  ALL PASS
```

**Status: PASS** (3 accept / 7 reject; FR-011a and FR-011b rejection cases confirmed)

---

### SC-003: Codegen determinism — zero diff on double-run

**Commands:**
```
mise exec -- moon run prompting-press:codegen prompting-press-python:codegen \
  prompting-press-typescript:codegen   # run 1
mise exec -- moon run prompting-press:codegen prompting-press-python:codegen \
  prompting-press-typescript:codegen   # run 2
git diff --exit-code -- \
  crates/prompting-press/src/generated/prompt_definition.rs \
  packages/python/python/prompting_press/generated/prompt_definition.py \
  packages/typescript/src/generated/prompt-definition.ts
```

**Result:** `DIFF CLEAN — zero diff after double-run` (git diff exit 0)

Moon hash IDs identical on both runs: `02b14a01` (Rust), `6bc3ee95` (Python),
`f6a62814` (TypeScript). The `schemas:codegen-check` gate independently confirmed the
same result:
```
codegen-check PASSED — all three generated files are up-to-date.
  crates/prompting-press/src/generated/prompt_definition.rs
  packages/python/python/prompting_press/generated/prompt_definition.py
  packages/typescript/src/generated/prompt-definition.ts
```

Schema→all-3-shapes propagation property: confirmed structurally by the codegen
pipeline (one schema input, three distinct language outputs), and proven to be
end-to-end correct by the codegen-check freshness gate passing clean.

**Status: PASS**

---

### SC-004: FFI-isolation gate fails on violation / passes clean

**Command (clean tree):**
```
mise exec -- moon run ci:check-ffi
```

**Captured output:**
```
FFI-isolation gate PASSED.
  prompting-press-core: no pyo3, no napi in dependency tree
  prompting-press: no pyo3, no napi in dependency tree
Tasks: 1 completed
```

**Break/recover proof:** completed in T031 (prior task). The gate was verified to
fail when `pyo3` was added to `prompting-press-core/Cargo.toml` with the message
"ERROR: FFI-isolation gate FAILED (Principle II / C-02)." and to return green after
revert. Not re-broken here per task instructions (cite T031).

**Status: PASS** (clean tree confirmed; break/recover cited from T031)

---

### SC-005: Codegen-freshness gate fails on violation / passes clean

**Command (clean tree):**
```
mise exec -- moon run schemas:codegen-check
```

**Captured output:** (see SC-003 above — same run)
```
codegen-check PASSED — all three generated files are up-to-date.
```

**Break/recover proof:** completed in T031. The gate was verified to fail when a
generated file was hand-edited, with output listing the drifted files and instructing
`mise exec -- moon run :codegen`. Not re-broken here per task instructions.

**Status: PASS** (clean tree confirmed; break/recover cited from T031)

---

### SC-006: Schema represents every v1 field

**Command:**
```
mise exec -- moon run schemas:check-schema
```

**Captured output:**
```
OK: prompt-definition.schema.json is a valid JSON Schema Draft 2020-12 document.
```

Evidence of field completeness from fixture acceptance (SC-002): the valid fixtures
exercise `role`, `body`, `variables` with `provenance` tags (`trusted`/`untrusted`/
`external`), `variants` with named arms, `meta` (opaque free-form), and `output_model`.
The schema rejects unknown roles, invalid provenance tags, a `variants` entry named
`default`, and variants attempting to redefine `role` — all v1 constraint points named
in the roadmap. The `name` field, JSON-Schema validation constraints on variables
(`minLength`, `minimum`, `maximum`, `format`, `enum`), and `metadata` are all present
in the fixtures.

**Status: PASS**

---

### SC-007: No rendering/validation/engine behavior

Full evidence: negative-scope checklist at
`specs/001-foundations/negative-scope-checklist.md`. All 9 items absent.

**Status: PASS**

---

## SC Summary Table

| SC | Description | Result |
|----|-------------|--------|
| SC-001 | Single-command build of all 4 active crates | PASS |
| SC-002 | 100% fixture accept/reject (3 accept, 7 reject) | PASS |
| SC-003 | Codegen determinism — zero diff on double-run | PASS |
| SC-004 | FFI-isolation gate passes clean (break/recover: T031) | PASS |
| SC-005 | Codegen-freshness gate passes clean (break/recover: T031) | PASS |
| SC-006 | Schema valid Draft 2020-12; all v1 fields present | PASS |
| SC-007 | Negative-scope clean — all 9 forbidden capabilities absent | PASS |

**All SC-001..SC-007: PASS**

---

## Part B — Negative-Scope Audit Summary

Full checklist: `specs/001-foundations/negative-scope-checklist.md`

| # | Capability | Evidence | Status |
|---|-----------|----------|--------|
| 1 | Template-engine integration | rg on all Cargo.tomls + pyproject + package.json — exit 1, no matches | ABSENT |
| 2 | render / rendering path | rg on crates/*/src/*.rs — exit 1, no matches (doc-comment hits in generated file are `#[doc="..."]` literals, not functions) | ABSENT |
| 3 | Typed-Vars validation runtime | Rust: doc-comment strings only; Python: `validate_default=True` is a Pydantic field metadata parameter (shape declaration, not invoked validator); TS: doc comment only | ABSENT |
| 4 | Agreement-check / variant-resolution / hashing | rg on crates/*/src/*.rs — exit 1, no matches | ABSENT |
| 5 | I/O (file/DB/network) in library code | rg on crates/*/src/*.rs — exit 1, no matches (codegen scripts are build-time tooling, not library code) | ABSENT |
| 6 | LLM call | rg on crates/*/src/*.rs — exit 1, no matches | ABSENT |
| 7 | Request-body assembly | rg on crates/*/src/*.rs — exit 1, no matches | ABSENT |
| 8 | Token counting | rg on crates/*/src/*.rs — exit 1, no matches | ABSENT |
| 9 | Output parsing / coercion | rg on crates/*/src/*.rs — exit 1, no matches | ABSENT |

All 9 items confirmed absent. SC-007 satisfied.

---

## Findings

### F-001 — `schemas` and `ci` projects inherit the Rust `build` task (minor; non-blocking)

**Observation:** `mise exec -- moon run :build` exits 1 because `schemas` and `ci`
projects inherit the global `build` task (`cargo build --package $project` from
`.moon/tasks/all.yml`) but contain no Rust crate. `schemas` is `language: unknown`
with no `workspace.inheritedTasks.exclude`; `ci` is similarly unconstrained.

**Impact on SC-001:** None. SC-001 specifies building the four active workspace
crates (`prompting-press-core`, `prompting-press`, `prompting-press-py`,
`prompting-press-node`), all of which build successfully. The `schemas` and `ci`
projects are not crates and are not named in SC-001's acceptance criteria.

**Impact on CI:** The GitHub Actions workflow uses `moon run ci:check-ffi` and
`moon run schemas:check-schema` / `schemas:validate-fixtures` / `schemas:codegen-check`
directly (not `:build`), so production CI is unaffected.

**Recommendation:** Add `workspace: { inheritedTasks: { exclude: ['build', 'test'] } }`
to `schemas/moon.yml` and `ci/moon.yml` so that `:build` sweeps clean. This is a
polish task, not a spec-001 blocker.

---

## Conclusion

Spec 001 Foundations is end-to-end validated. SC-001 through SC-007 all pass.
The structural spine — polyglot workspace, JSON Schema single source of truth,
deterministic codegen pipeline, and CI guardrails — is in place and mechanically
verified. The library contains none of the forbidden capabilities enumerated in
FR-021 / FR-022.
