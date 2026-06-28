# Verify Spec Report — 005-ts-binding (TypeScript binding `prompting-press-node`)

**Verdict: PASS**

Read-only acceptance gate (`/speckit.verify`) against `spec.md` (FR-001..FR-025, SC-001..SC-011),
`plan.md`, `tasks.md`, and the constitution. Tooling here is read-only (Read/Write only — no shell),
so executable verdicts (test pass counts, gate green/red) rely on the main-thread results stated in
the task brief plus direct reading of the test/code/gate sources for what they assert. No fabrication:
every finding cites a file:line read this session.

## Integrity check — PASSED (no tool-channel corruption)

| Check | Expected | Observed | Result |
|---|---|---|---|
| `specs/005-ts-binding/spec.md` lines | ~490 | 495 | match |
| `crates/prompting-press-node/src/render.rs` lines | ~387 | 387 | match |
| `packages/typescript/src/index.ts` lines | ~594 | 594 | match |

All three within tolerance; files are real and substantive. Proceeded with analysis.

## Verify Spec Summary
- Spec: 005-ts-binding
- Requirements checked: 36 (25 FR + 11 SC)
- Implemented: 36
- Partial: 0
- Missing: 0
- Diverged: 0
- Inconclusive: 0 (executable verdicts taken from main-thread results, code/tests read directly)

## Per-SC pass/fail

| SC | Status | Primary evidence |
|----|--------|------------------|
| SC-001 validate-then-render, no kernel/ZodError on surface | PASS | `render.test.mjs:100-123` (valid render → text+name+variant+hex hashes); facade validate-first at `index.ts:418-435` |
| SC-002 reject-before-render naming every field | PASS | `render.test.mjs:147-160` (both `name`+`count` rows); facade throws before addon at `index.ts:349-361,420` |
| SC-003 YAML/JSON/object parity | PASS | `loader.test.mjs:78-125` (3 forms → 1 text + 1 templateHash + 1 renderHash) |
| SC-004 undeclared-variable detection | PASS | `check.test.mjs:66-85` (prompt/variant/var named); Rust wiring `check.rs:170-204` |
| SC-005 untrusted-without-guard | PASS | `check.test.mjs:91-127` (prompt+field named; guard under meta/metadata clears) |
| SC-006 no native error type on API | PASS | `render.test.mjs:166-181` (`!(err instanceof z.ZodError)`, `constructor.name!=="ZodError"`) |
| SC-007 napi-only + no engine logic | PASS | gate `check-ffi-isolation.sh:28` `FFI_CRATES=("pyo3" "napi")` over kernel+consumer; delegation `render.rs:181`, `compose.rs:195`, `check.rs:144` |
| SC-008 N-ordered messages | PASS | `compose.test.mjs:286-307` (3 ordered role/text); `compose.rs:180-211` order-preserving loop |
| SC-009 build+import, hash parity | PASS (parity structural) | `package.json:36-46` ESM build/test; T026/T030 fresh-env import per brief; hashes structural via shared `from_serialize` (`marshal.rs:73-77`) |
| SC-010 codegen-fresh + no token surface | PASS | generated header `prompt-definition.ts:1-9`; `output_model` metadata-only `:42`; no `count_tokens`/`tokenCount` anywhere (T027 rg clean per brief) |
| SC-011 Node advisory gate | PASS | `check-advisories-node.sh:40` `pnpm audit --audit-level high`; registered `ci/moon.yml:55-60` |

## Per-FR coverage

| FR | Status | Evidence / Gap |
|----|--------|----------------|
| FR-001 Zod v4 + `.refine()`; plain typed also accepted | IMPLEMENTED | `zod 4.4.3` pinned `package.json:48`; schema-or-data duck-typed `index.ts:331-337,418-426`; static path tested `render.test.mjs:114-123` |
| FR-002 own validation at render boundary, once before templating | IMPLEMENTED | `validateOrThrow` `index.ts:349-361`; throws before `napiRender` `index.ts:420` |
| FR-003 only validated values cross FFI | IMPLEMENTED | facade passes plain value; kernel-direct render is validation-blind `render.rs:1-28,141-184` |
| FR-003a lossless marshal; undefined/absent→absent, null→null | IMPLEMENTED | `marshal.rs:73-77` (one `from_serialize` hop); rules documented `marshal.rs:25-43`; tests `marshal.rs:98-191` (null/bool/int/float/bigint/nested) |
| FR-004 ZodError not exposed | IMPLEMENTED | mapped to `PromptValidationError` `index.ts:354-360`; `render.test.mjs:166-181` |
| FR-005 dual-input loader reusing Rust consumer | IMPLEMENTED | `registry.rs:60-113` (loadYaml/loadJson marshal text; insert→JSON→load_json) |
| FR-006 YAML==JSON internal rep | IMPLEMENTED | `registry.rs:219-240` (re-serialize equality); `loader.test.mjs:78-109` |
| FR-007 malformed→structured error, nothing partial | IMPLEMENTED | `registry.rs:181-194,242-256`; `loader.test.mjs:173-236` (incl. atomic re-load) |
| FR-008 codegen'd TS shape, not hand-maintained | IMPLEMENTED | `prompt-definition.ts:1-9` GENERATED header; freshness-gated `schemas:codegen-check` |
| FR-008a registry; absent name→structured error | IMPLEMENTED | `registry.rs:30-33,130-132`; `render.rs:167-171` → `UnknownPromptError` (`render.test.mjs:324-335`) |
| FR-009 render(name, vars) → text+provenance; plumb guard | IMPLEMENTED | `render.rs:157-184`; `RenderResult` getters `render.rs:82-126`; guard plumb-through tested `render.test.mjs:286-318` |
| FR-010 getSource unrendered source | IMPLEMENTED | `render.rs:195-200`; `render.test.mjs:341-365` |
| FR-011 no reimplemented render/agreement/variant/hash | IMPLEMENTED | kernel-direct delegation `render.rs:181`; check delegates `check.rs:144`; marshal is value-only `marshal.rs:73-77` |
| FR-012 composition as explicit ordered array → {role,text} | IMPLEMENTED | `compose.rs:103-211`; `index.ts:492-578`; `compose.test.mjs:84-142,286-307` |
| FR-013 no `.chain()` | IMPLEMENTED | `append` returns void `compose.rs:145-148`, `index.ts:529-541`; `compose.test.mjs:234-241` asserts absence |
| FR-014 normalize ZodError+Rust errors → `[{field,code,message}]` JS Error hierarchy; closed vocab | IMPLEMENTED | hierarchy `index.ts:77-114`; `subclassForCode` `index.ts:140-159`; Rust payload `error.rs:95-149`; closed vocab `crates/prompting-press/src/error.rs:43-74` |
| FR-015 SEC-004 scrub; ZodError mapper copies message+path only | IMPLEMENTED | consumer scrub `crates/prompting-press/src/error.rs:178-211`; node routes raw KernelError through scrubber first `error.rs:151-160`; Zod mapper uses `issue.message`+`path` only `index.ts:354-360`; JS-surface no-leak `render.test.mjs:187-250` |
| FR-016 single check over registry; referenced ⊆ declared | IMPLEMENTED | `check.rs:141-145`; `check.test.mjs:66-85` |
| FR-017 referenced vars + provenance from core; declared set = `variables` | IMPLEMENTED | `check.rs:133-145` delegates; no Zod introspection (check takes only `reg`) |
| FR-018 provenance lint: untrusted/external without guard | IMPLEMENTED | `check.rs:127-130` (`untrusted_without_guard`); `check.test.mjs:91-127` |
| FR-019 pure analysis, no mutation/render | IMPLEMENTED | `&Registry` shared borrow `check.rs:143`; purity tested `check.test.mjs:171-197` |
| FR-020 actionable findings; reserved/analysis as distinct kinds | IMPLEMENTED | `kind_discriminant` 4 kinds `check.rs:124-131`; `check.test.mjs:133-165` (reserved + analysis_error) |
| FR-021 napi-rs ESM-only addon, optionalDeps platform binaries | IMPLEMENTED | `package.json:6,18-25,38-39` (`--esm`); `engines.node>=20` `:15-17`; napi metadata `:32-35` |
| FR-022 napi only in `-node`; kernel+consumer FFI-free | IMPLEMENTED | gate asserts both `check-ffi-isolation.sh:22-28`; napi declared only in `crates/prompting-press-node/Cargo.toml:63-68` |
| FR-023 no I/O/LLM/request-body/token-count; outputModel metadata only | IMPLEMENTED | no token surface (SC-010); `output_model` metadata-only `prompt-definition.ts:39-42`; guard never concatenated `render.rs:84` / `render.test.mjs:301-306` |
| FR-024 codegen freshness gate | IMPLEMENTED | `schemas:codegen-check` (existing); generated file present + headered `prompt-definition.ts:1-9` |
| FR-025 Node advisory gate | IMPLEMENTED | `check-advisories-node.sh`; registered `ci/moon.yml:55-60` |

## Findings By Severity

### Must Fix Before Proceeding
- None.

### Should Address
- None blocking. The implementation matches spec intent on every FR/SC examined.

### Notes
- **`napi-derive` pin diverges from the plan text (benign).** `plan.md:49-50` and `tasks.md:38-39`
  say pin `napi`/`napi-derive` exact to **3.9.4**; the manifest pins `napi = 3.9.4`
  (`Cargo.toml:64`) but `napi-derive = 3.5.7` (`Cargo.toml:68`), with an in-file comment
  (`Cargo.toml:51-54`) explaining the two crates version independently and 3.5.7 is the
  crates.io-verified latest for `napi-derive`. Both are pinned **exact**, so FR-022 intent and the
  `ci:check-floating-versions` gate are satisfied. This is a doc-vs-manifest wording drift, not a
  spec violation — worth reconciling the plan/tasks note in cleanup, not a gate failure.
- **The 005-specific seams the critique flagged are correctly built.** (1) The napi→facade
  structured-error channel: Rust encodes `{code, errors:[...]}` JSON into `napi::Error.reason`
  (`error.rs:75-149`), and the facade `decodeAddonError` `JSON.parse`s it, reads top-level `code`,
  and picks the subclass via `subclassForCode` (`index.ts:140-215`) — kernel codes route to
  `PromptRenderError`, with a defensive base-class wrapper for any non-conforming throw
  (`index.ts:200-205`) so a raw napi error never escapes. (2) The **public** `Composition` is the
  TS-facade class (`index.ts:492-578`) that Zod-validates each entry before delegating to the
  low-level addon `Composition` (imported as `NapiComposition`), exactly as the critique required;
  validated by `compose.test.mjs:148-174` (rejected append stores nothing).
- **SEC-004 holds end-to-end at the JS surface.** Two independent JS-surface assertions: a
  Zod-rejected secret (`render.test.mjs:187-215`, checks `.message`/`.stack`/rows) and a real
  kernel render-error secret (`render.test.mjs:217-250`, asserts only `code:["render"]` +
  `"render error"` survive). The Rust side proves the scrubber runs before encoding
  (`error.rs:151-160,187-259`; consumer scrub `crates/prompting-press/src/error.rs:191-208`). The
  Zod mapper provably copies only `issue.message` + `issue.path` (`index.ts:354-358`).
- **Delegation / zero-engine-logic is real (C-02 / Principle I).** render/compose call
  `prompting_press_core::render` directly (`render.rs:181`, `compose.rs:195`) because the consumer's
  generic-`V` render needs a garde type the binding lacks (documented `render.rs:1-28`,
  `compose.rs:13-23`); check/getSource/loaders reuse the `prompting-press` consumer
  (`check.rs:144`, `render.rs:197`, `registry.rs:62-112`). `marshal.rs` is a pure two-hop value
  bridge, no logic.
- **Hash parity (SC-009 last clause) is structural, not re-tested in TS** — correct per Principle I.
  The binding feeds the kernel a value built by the same `minijinja::Value::from_serialize` path the
  Rust consumer uses (`marshal.rs:73-77`), so byte-identity with Python/Rust is by construction. The
  TS tests assert hash *shape* (64-hex) and *within-binding* equality across input forms
  (`loader.test.mjs:102-108`), which is the right scope.

## Verification Commands
Read-only environment — none run by this agent. Verdicts below are from the main-thread results
stated in the task brief; the code/tests/gates they exercise were read and confirmed to assert what
each SC/FR requires:
- `cargo test -p prompting-press-node` (36 pass): not run here; covers `render.rs`/`error.rs`/
  `registry.rs`/`check.rs`/`compose.rs`/`marshal.rs` `#[cfg(test)]` suites — read and consistent.
- `pnpm -C packages/typescript test` (57 pass, node:test): not run here; covers the 4 `.test.mjs`
  files — read and consistent with SC-001/002/003/004/005/006/008.
- `moon run ci:check-ffi` (SC-007 / FR-022): not run; gate script reads `FFI_CRATES=("pyo3" "napi")`.
- `moon run ci:check-advisories-node` (SC-011 / FR-025): not run; script + moon task present.
- `moon run schemas:codegen-check` (SC-010 / FR-024): not run; generated file present + headered.
- `cargo clippy` clean, fresh-env `pnpm pack` import+render: not run; per brief, pass.
