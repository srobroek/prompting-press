# Verify Spec Report — 001 Foundations

**Spec**: 001-foundations — Layout, Schema, Codegen, CI Guardrails
**Branch**: `001-foundations`
**Mode**: adversarial, read-only verification of implemented code vs FR-*/SC-*
**Date**: 2026-06-25
**Verifier note**: No Bash/exec tool was available in this verification context, so
gate *behavior* (does the FFI gate actually fail when pyo3 is added; does the
freshness gate actually fail on a stale shape) is assessed from gate *logic* +
wiring, not from a live run. Those items are tagged "logic-verified; live-run
UNVERIFIED in this context." Local `mise exec -- moon run ...` execution is the
recommended confirmation step before merge.

## Verify Spec Summary
- Spec: 001-foundations
- Requirements checked: 27 FR + 7 SC = 34
- Implemented (PASS): 31
- Partial: 3
- Missing: 0
- Diverged: 0
- Inconclusive: 0 (3 gate-behavior items are PASS-on-logic with live-run unverified, noted inline)

**Overall verdict: PASS with minor gaps.** Every functional requirement and success
criterion is met by committed code/config. The three PARTIALs are documentation/process
gaps (a missing committed negative-scope checklist artifact, and gate-behavior that is
logic-correct but not live-run in this context), not requirement violations.

## Requirement Details

### Layout (FR-001..007)

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| FR-001 | PASS | `crates/prompting-press-core/Cargo.toml` — deps are only `serde`/`serde_json`; no pyo3/napi. `src/lib.rs` is a pure-Rust stub exposing `version()`. | — |
| FR-002 | PASS | `crates/prompting-press/Cargo.toml` deps `prompting-press-core` (path) + serde; no FFI. `src/lib.rs` re-exports kernel + generated shape; `core_version()` exercises the edge. | — |
| FR-003 | PASS | `crates/prompting-press-py/Cargo.toml` (`crate-type=["cdylib"]`, pyo3 0.29) and `crates/prompting-press-node/Cargo.toml` (`crate-type=["cdylib"]`, napi/napi-derive 3) each dep core+consumer; pyo3/napi confined to these two crates. | — |
| FR-004 | PASS | `packages/python/pyproject.toml` (maturin → `crates/prompting-press-py`) and `packages/typescript/package.json` (napi-rs → `crates/prompting-press-node`). Wrappers distinct from binding crates. | — |
| FR-005 | PASS | `packages/go/README.md` — reserved placeholder, no `go.mod`, explicitly excluded from workspace + moon graph. `mise.toml` pins `go = "1.26.2"` but it is unwired (reserved for spec 006); README states packages/go stays toolchain-free in 001. | — |
| FR-006 | PASS | `.moon/workspace.yml` enumerates projects explicitly (no globs); `packages/go` intentionally absent. `:build` fans out to the 4 crates + non-crate projects exclude cargo build/test (`schemas/`, `ci/`, `packages/*` set `inheritedTasks.exclude: [build,test]`). Go excluded. | — |
| FR-007 | PASS | No `packages/rust` directory remains (read returns not-found). `packages/` now holds `python/`, `typescript/`, `go/` only. T004 recorded the reorg. | — |

### Schema (FR-008..013)

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| FR-008 | PASS | `schemas/jsonschema/prompt-definition.schema.json` — `$schema` = Draft 2020-12, stable `$id` `https://prompting-press.dev/schemas/...`. Meta-validation gate `schemas/jsonschema/scripts/meta_validate.py` calls `Draft202012Validator.check_schema`. | — |
| FR-009 | PASS | `role` is `enum: [system,user,assistant]`. Generated to Rust `PromptDefinitionRole`, Python `Role`, TS union. | — |
| FR-010 | PASS | Root sealed (`additionalProperties:false`), `required:[name,role,body]`, plus `variables`, `variants`, `output_model`, `metadata`, `meta`. No `model` field anywhere. | — |
| FR-010a | PASS | `$defs/VariableDecl` requires `type` + `provenance` (enum trusted/untrusted/external) and carries `format`/`pattern`/`enum`/`minimum`/`maximum`/`minLength`/`maxLength`/`description` — generate-then-extend ready. Pydantic emits `min_length`/`ge` constraints; TS/Rust carry the fields (constraints dropped by their type systems, expected per research D1). | — |
| FR-011 | PASS | Root `body` is required; no `default:` marker in schema. Default-as-root is structural. (The `is_default`/`default` *surfacing* is an API concern for specs 002+, correctly not built here — data-model.md notes it is structural, not a schema field.) | — |
| FR-011a | PASS | `$defs/Variant` is sealed (`additionalProperties:false`), `required:[body]`, allows only `body`+`meta`. Reject fixture `variant-redefines-role.json` (variant with `role`) confirms. | — |
| FR-011b | PASS | `variants.propertyNames.not.const = "default"`. Reject fixture `variant-named-default.json` confirms. Note: this key is stripped before `cargo-typify` (documented workaround in `crates/prompting-press/scripts/codegen.sh`) because typify cannot parse `not`/`const`; sound because it is a validation-only constraint no generated type can encode, and it is enforced by the validation gate + reject fixture. | — |
| FR-011c | PASS | `Variant.meta` and root `meta`/`metadata` are open objects (`additionalProperties:true`); no schema-enforced selection semantics. | — |
| FR-012 | PASS | Schema expresses every v1 field the roadmap names (role, body, variables+provenance+constraints, variants, output_model, metadata, meta). See SC-006. | — |
| FR-013 | PASS | 3 accept fixtures (`single-body.json`, `multi-variant.json`, `variant-with-meta.json`) + 6 reject fixtures (`bad-role.json`, `bad-provenance.json`, `variant-named-default.json`, `variant-redefines-role.json`, `extra-root-key.json`, `not-json.txt`). Covers every case the spec enumerates incl. the FR-011a/b rejections and the non-parseable-doc edge case (distinct from schema-invalid, handled in `validate_fixtures.py`). | Plan mentioned `*.{yaml,json}`; only `.json` (+ `not-json.txt`) exist. Not a gap — FR-013 doesn't mandate YAML and the validator (`json.loads`) matches the actual fixture set. |

### Codegen (FR-014..017)

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| FR-014 | PASS | Three committed shapes faithfully represent the schema: Rust `crates/prompting-press/src/generated/prompt_definition.rs` (serde structs, `deny_unknown_fields`, role/provenance enums, `serde_json::Map` for meta/metadata); Python `packages/python/python/prompting_press/generated/prompt_definition.py` (Pydantic v2, `extra='forbid'`, `Enum`s); TS `packages/typescript/src/generated/prompt-definition.ts` (sealed interfaces, role/provenance unions, `[k:string]: unknown` for open objects). | — |
| FR-015 | PASS (logic) | Determinism designed in: `rust-toolchain.toml` pins 1.95.0 (rustfmt fixed); Python `--disable-timestamp --formatters builtin` + static header; TS static banner + LF/trailing-newline normalization + pinned prettier 3.8.4; Rust static header + rustfmt pass. All tools exact-pinned in `mise.toml`/lockfiles. | Twice-run byte-identical NOT live-verified in this context (no exec). Verified statically that no time/host-varying content is emitted. |
| FR-016 | PASS | All three committed; each carries a "GENERATED — DO NOT EDIT" header; segregated dirs (`crates/.../generated/`, `packages/python/.../generated/`, `packages/typescript/src/generated/`); `.gitignore` explicitly tracks the generated paths and ignores build output. Rust uses hand-written `generated.rs` mod wrapper so the generated file stays clean. | — |
| FR-017 | PASS | Single command: each project defines a `codegen` task (`crates/prompting-press/moon.yml`, `packages/python/moon.yml`, `packages/typescript/moon.yml`); `moon run :codegen` fans out to all three. `:build` on the consumer crate `deps: [codegen]`, so codegen runs before build (the generated module must exist to compile). | — |

### CI guardrails (FR-018..020)

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| FR-018 | PASS (logic) | `scripts/ci/check-ffi-isolation.sh` runs `cargo tree -p <crate> -i pyo3`/`napi` over an explicit `COVERED_CRATES=(prompting-press-core, prompting-press)`; fails if either FFI crate appears (catches transitive). Wired via `ci:check-ffi` moon task + `.github/workflows/ci.yml` gates job. | Live-run (does it fail when pyo3 is added to the kernel) not executed here — see SC-004. |
| FR-019 | PASS (logic) | `schemas/scripts/codegen-check.sh`: `git add -N` (catches partial/new-file regen) + `git diff --exit-code` over the three generated paths. Moon `schemas:codegen-check` `deps:[<three> :codegen]` so it regenerates first then diffs — a stale committed shape (schema changed, no regen) surfaces as a diff. Wired in CI gates job ("T029: Codegen freshness"). | Uses git-diff for all three rather than `datamodel-codegen --check` for Python (plan D2 alternative). Benign divergence — git-diff satisfies FR-019 and the partial-regen edge case identically. Live-run not executed here — see SC-005. |
| FR-020 | PASS | Both gate scripts emit a message naming the violated invariant + location: FFI gate prints "Principle II / C-02" and the offending `<crate> depends on <ffi>`; freshness gate lists each drifted file path and the remediation (`moon run :codegen`). Floating-version lint (`check-floating-versions.sh`) cites SEC-003 + manifest. | — |

### Scope boundary (FR-021..022)

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| FR-021 | PASS | No render/validate/agreement/variant-resolution/hashing/template-engine/typed-Vars code in any crate. Kernel `lib.rs` = `version()` stub; consumer `lib.rs` = re-export + `core_version()`; binding crates = single `core_version()` passthrough. Generated shapes are data types only (serde/Pydantic/TS), no behavior. | — |
| FR-022 | PASS | No I/O, LLM, request-body, token-counting, or output-parsing code. The only I/O in the repo is in *build/codegen tooling scripts* (reading the schema, writing generated files) and *validation gate scripts* — not in the library/runtime crates. TS package `include` is generated-only; Python `__init__` is a version marker. | — |

## Success Criteria

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| SC-001 | PASS (logic) | `moon run :build` defined for 4 active crates; consumer build deps codegen; no per-crate manual setup (mise provides toolchain). | Live build not executed in this context. |
| SC-002 | PASS (logic) | `validate_fixtures.py` asserts every `valid/` accepts and every `invalid/` rejects; 3 valid + 6 invalid fixtures present and correct by inspection (each invalid fixture violates exactly the intended constraint). | Live run not executed; logic + fixtures confirmed by reading. |
| SC-003 | PASS (logic) | `schemas:codegen-check` = regenerate + `git diff --exit-code`. Determinism flags present in all three codegen scripts (see FR-015). | Twice-run zero-diff + single-field-change-propagates not live-executed here. |
| SC-004 | PASS (logic); live-run UNVERIFIED | FFI gate logic is correct (`cargo tree -i`, explicit covered-crate list, clean message). T031 claims scratch-branch verification was done. | The "add pyo3 → gate fails" toggle was not re-run in this verification context. Recommend one local `cargo add pyo3 -p prompting-press-core && mise exec -- moon run ci:check-ffi` to re-confirm before merge. |
| SC-005 | PASS (logic); live-run UNVERIFIED | Freshness gate logic correct (regenerate-then-diff, `git add -N` for partial regen). | "Hand-edit a generated shape → gate fails" not re-run here. Recommend local confirmation. |
| SC-006 | PASS | Schema field set covers every v1 field the roadmap names (role/body/variables+provenance+constraints/variants/output_model/metadata/meta); no model axis; provenance + output_model declared-not-consumed. No reason-known-today to churn the field set for specs 002–007. | — |
| SC-007 | PARTIAL | Negative scope is clean *by code inspection* (FR-021/FR-022 PASS — no render/validate/IO/LLM/token/parse code anywhere in the crates). | T033 called for an **auditable committed negative-scope checklist** asserting each forbidden capability absent individually. No such artifact file exists under `specs/001-foundations/` (no `negative-scope-review.md` / validation-run record found). The *outcome* holds; the *audit artifact* deliverable is missing. |

## Findings By Severity

### Must Fix Before Proceeding
- None. No FR or SC is violated by the implemented code.

### Should Address
- **SC-007 / T033 audit artifact missing.** The negative-scope review was specified as an
  *auditable checklist* (T033: "assert each forbidden capability is absent individually").
  The condition is genuinely satisfied in code, but there is no committed artifact recording
  the checklist outcome. Either commit the checklist (e.g. `specs/001-foundations/negative-scope-review.md`)
  or accept that the audit lives only in this verify-report. Process gap, not a code gap.
- **Re-confirm gate behavior locally (SC-004, SC-005).** Both gates are logic-correct and
  wired into CI, and tasks.md marks T031 as done, but the failure paths were not re-exercised
  in this read-only verification. Before relying on them, run once locally:
  `cargo add pyo3 -p prompting-press-core; mise exec -- moon run ci:check-ffi` (expect fail),
  and hand-edit `crates/prompting-press/src/generated/prompt_definition.rs` then
  `mise exec -- moon run schemas:codegen-check` (expect fail). Revert both.

### Notes
- **FR-019 mechanism divergence from plan D2 (benign).** Plan specified `datamodel-codegen --check`
  for the Python leg; implementation uses the unified `git add -N` + `git diff --exit-code` over all
  three generated paths. This satisfies FR-019 (including the partial-regeneration edge case) at
  least as well as the planned split, with one consistent mechanism. No action needed.
- **FR-011b typify workaround (sound).** `cargo-typify` 0.7.0 panics on `variants.propertyNames`
  (`not`/`const`), so `codegen.sh` strips only that key from a scratch copy before generating Rust
  (on-disk schema untouched). Correct: `propertyNames` is a validation-only constraint no generated
  type in any language can encode; the reserved-`default` rule is enforced by the validation gate +
  `variant-named-default.json` reject fixture. Documented in the script + generated-file header.
- **Determinism is the load-bearing property and is statically sound** (no timestamps/banners/host
  data in any of the three outputs; toolchains exact-pinned in `mise.toml` + lockfiles; `LC_ALL`
  not set in scripts but rustfmt/prettier/builtin-formatters are deterministic regardless). The only
  residual risk is environmental (a CI runner missing a pinned tool), mitigated by the mise-action
  install step in the gates job.
- **Go reservation correct.** `packages/go` is a README-only placeholder, excluded from workspace
  members (`crates/*` only) and from the moon project map. `mise.toml` pins a Go version but it is
  reserved for spec 006 and unwired; CI build matrix runs `cargo build --workspace` only (no Go leg).
- **CI matrix on GitHub (esp. Windows pyo3 abi3 link) is a post-push item**, out of local scope per
  the task brief. The workflow installs Python 3.12 on all legs before `cargo build --workspace`
  specifically for the Windows pyo3 import-lib link; correctness of that leg is verifiable only on
  the GitHub runners.

## Verification Commands
- `mise exec -- moon run :build` — not run (no exec tool in this context); logic verified
- `mise exec -- moon run schemas:check-schema` — not run; logic verified (meta_validate.py)
- `mise exec -- moon run schemas:validate-fixtures` — not run; logic + 9 fixtures verified
- `mise exec -- moon run schemas:codegen-check` — not run; determinism/freshness logic verified
- `mise exec -- moon run ci:check-ffi` — not run; FFI-isolation logic verified
- `mise exec -- moon run ci:check-floating-versions` — not run; lint logic verified (tree is clean)
- `cargo tree -p prompting-press-core -i pyo3` — not run; expected "did not match any packages"
