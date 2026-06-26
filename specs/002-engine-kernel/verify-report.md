# Verification Report — Spec 002 (Engine kernel `prompting-press-core`)

**Date**: 2026-06-26
**Branch**: `002-engine-kernel`
**Mode**: read-only adherence gate (FR/SC + Constitution)
**Scope**: 30 FRs (FR-001..029 + FR-001a + FR-016a), 9 SCs (SC-001..009), 7 constitution principles.

## Summary

The implementation is a faithful, high-fidelity realization of the spec. Every functional
requirement has direct, intent-matching code evidence, and every code-verifiable success criterion is
backed by a test. The two load-bearing risk areas the spec calls out — strict-undefined backstop and
the `undeclared_variables`-empty-set-on-parse-error footgun (FR-016a) — are both correctly closed in
code with dedicated tests. No constitution conflict found.

One **should-address** gap: the dedicated render-regression fixture set (FR-029) exists as data and a
loader (`common::load_regression_case` / `RegressionCase`), but **no test consumes it** — the loader is
under `#![allow(dead_code)]` and the `tests/fixtures/render/*.json` pairs are never asserted. The
*behavior* FR-029 targets (pinned template→output) is still covered via the `load_def_fixture` path in
`render.rs` (V1.6 conditional-loop, V1.1 interpolation), so this is a coverage-wiring gap, not a missing
capability.

## Findings

| ID | Category | Severity | Location | Summary | Recommendation |
|----|----------|----------|----------|---------|----------------|
| F-01 | FR-029 / test wiring | MEDIUM (Should address) | `crates/prompting-press-core/tests/common/mod.rs:63` (`load_regression_case`); `tests/fixtures/render/{interpolation,conditional-loop}.json` | The dedicated render-regression fixture loader (`RegressionCase` / `load_regression_case`) is `#[allow(dead_code)]` and is called by **no** test. The `render/*.json` template→expected pairs are inert: they guard nothing on their own. FR-029 wants a *regression guard*, not just fixture data on disk. | Add a tiny `render_regression.rs` test that iterates `tests/fixtures/render/*.json`, renders each `template` with `values`, and asserts `== expected`. Equivalent behavior is currently covered by `render.rs` V1.1/V1.6, so this is wiring the explicit guard, not new coverage. |
| F-02 | FR-001a / error precision | LOW (Note) | `crates/prompting-press-core/src/engine.rs:215-222` | `KernelError::UndefinedVariable.name` is best-effort: MiniJinja's `UndefinedError` carries no variable-name payload, so `name` is the error `detail` or `Display`, not necessarily the bare root name. Spec/contract acknowledge this; tests only assert the variant, not the name value. Acceptable, but a consumer must not rely on `name` being the literal variable identifier. | None required. Documented in code + data-model. Optionally note in spec-003 normalization that `name` is advisory. |
| F-03 | FR-002 / heuristic coupling | LOW (Note) | `crates/prompting-press-core/src/engine.rs:252-260` | `looks_like_excluded_feature` keys off the exact phrase `"unknown statement <kw>"` for 6 keywords. It is NOT a no-op (verified by `engine::tests::excluded_feature_detail_is_recognised` + the tightness test + `excluded_features::*classify_precisely*`). Robustness of the SC-008 gate does not depend on it — both `ExcludedFeature` and `Parse` are accepted rejections. The precise-classification test will fail loudly on a MiniJinja wording change, which is the intended drift alarm. | None. The drift alarm (`excluded_features_classify_precisely_as_excluded_feature`) is the correct guard; ensure it stays bound to the roadmap-Q3 bump obligation (already in `deny.toml` / `check-advisories.sh`). |
| F-04 | FR-022 / guard-empty edge | LOW (Note) | `crates/prompting-press-core/src/provenance.rs:146-148` | `build_guard_text` returns `None` when `enabled` but the untrusted∪external union is empty. Spec FR-022 says when opted in, produce a guard "naming the untrusted/external fields"; with none to name, `None` is the only sensible output and is not contradicted by any SC. No test pins this exact edge (enabled + no untrusted/external fields → `None`). | Optional: add a one-line test for `enabled:true` over an all-`trusted` def → `guard == None`. Behavior is correct as-is. |

No HIGH or CRITICAL findings.

## Per-FR coverage

| FR | Status | Evidence |
|----|--------|----------|
| FR-001 (render interp/cond/loop) | IMPLEMENTED | `engine::render` (engine.rs:154); minijinja `default-features=false` + `builtins`,`adjacent_loop_items` (Cargo workspace dep). Tests: render.rs V1.1/V1.6. |
| FR-001a (strict undefined → loud) | IMPLEMENTED | `build_environment` sets `UndefinedBehavior::Strict` (engine.rs:50); `UndefinedError`→`UndefinedVariable` (engine.rs:215). Test: render_errors.rs V1.7; engine::tests::builds_with_strict_undefined. |
| FR-002 (reject include/import/extends/macro/block) | IMPLEMENTED | Enforced structurally by `macros`/`multi_template` OFF (parse errors), labeled via `map_minijinja_error`. Tests: excluded_features.rs (6 fixtures, render + agreement paths, both rejection + precise-classification). |
| FR-003 / SC-001 (determinism) | IMPLEMENTED | `BTreeSet` ordering, `sha2`, no time/random; per-render fresh env. Test: hashing.rs V1.2. |
| FR-004 (validation-blind) | IMPLEMENTED | Kernel takes `minijinja::Value`; no type/constraint inspection anywhere. lib.rs invariants doc. |
| FR-005 (no I/O/LLM/req-body/tokens/output-parse) | IMPLEMENTED | Deps are `serde`,`serde_json`,`minijinja`(no default features),`sha2` only (Cargo.toml). No `std::fs`/`std::net`/`reqwest`/`tokio` in any kernel src module. (Test harness `common/mod.rs` reads fixtures via `std::fs`, but that is `tests/`, not the library.) |
| FR-006 (get_source) | IMPLEMENTED | `engine::get_source` (engine.rs:103). Test: hashing.rs V1.8 (source bytes hash == template_hash). |
| FR-007 (no-variants → implicit `default`=root body) | IMPLEMENTED | `resolve_variant` None→`default`,`def.body` (engine.rs:78). Test: render.rs V1.1. |
| FR-008 (caller-owned selection, no assignment logic) | IMPLEMENTED | `resolve_variant` only matches name; no weighting/selector. Test: render.rs V1.3. |
| FR-009 (unknown variant error names it) | IMPLEMENTED | `UnknownVariant{requested}` (engine.rs:87). Test: render_errors.rs V1.4. |
| FR-010 (root body always default; no missing-default path) | IMPLEMENTED | `resolve_variant` has only `UnknownVariant` error; no missing-default branch (engine.rs:73-92); `KernelError` has no missing-default variant (error.rs:19-56). Test: render.rs V1.5 (multi-variant None→root body, not a named arm). |
| FR-011 (reserved `default`→root body, enforced in logic) | IMPLEMENTED | `Some("default")` matched same as None (engine.rs:78); `DEFAULT_VARIANT` const enforced in kernel, not the generated type (which lacks it). |
| FR-012 (template_hash=SHA256(source)) | IMPLEMENTED | engine.rs:172 `sha256_hex(resolved.source)`. Test: hashing.rs V1.8, per_variant_distinct_template_hash. |
| FR-013 (render_hash=SHA256(text)) | IMPLEMENTED | engine.rs:173. Test: hashing.rs V1.8. |
| FR-014 (no vars_hash) | IMPLEMENTED | `RenderResult` has no such field (engine.rs:114-129); hashing.rs `fr_014_render_result_has_no_vars_hash` exhaustively destructures (compile-level guard). |
| FR-015 (provenance as plain data, no telemetry) | IMPLEMENTED | `RenderResult` is a plain struct, no sink/tracing. Fields: text,name,variant,template_hash,render_hash,guard. |
| FR-016 (per-variant required roots via undeclared_variables(false)) | IMPLEMENTED | `required_roots` calls `template.undeclared_variables(false)` (agreement.rs:109). Tests: agreement.rs V2.1/V2.2/V2.3. |
| FR-016a (parse-first; never empty set on parse fail) | IMPLEMENTED | `add_template_owned` + `get_template` BEFORE `undeclared_variables` (agreement.rs:99-104); error short-circuits. Tests: agreement.rs V4.3; excluded_features.rs `agreement_rejects_every_excluded_feature`. |
| FR-017 (exclude locals + globals/filters allowlist) | IMPLEMENTED | Locals excluded by engine analysis; globals subtracted via `env_globals` (agreement.rs:113,136). Tests: V2.2/V2.3 (locals), V2.4 range+namespace (globals). |
| FR-018 (pure, no mutation, no render) | IMPLEMENTED | `&PromptDefinition` shared borrow, no `values`, no render. Test: agreement_purity.rs V2.5 (serialize before/after compare). |
| FR-019 (does NOT do ⊆ check) | IMPLEMENTED | `required_roots` returns the set only; no comparison to declared `variables`. Documented agreement.rs:9-10. |
| FR-020 (allowlist env-derived, not hardcoded) | IMPLEMENTED | `env_globals` reads `env.globals()` dynamically (agreement.rs:136-140). Test: agreement::tests::env_globals_include_builtin_globals (asserts non-empty + range/dict/namespace present). |
| FR-021 (carry tags; expose untrusted/external) | IMPLEMENTED | `provenance_view` buckets by `VariableDeclProvenance` (provenance.rs:92). Test: provenance.rs V3.1. |
| FR-022 (opt-in guard, separate field, not concatenated) | IMPLEMENTED | `RenderResult.guard: Option<String>`, set via `build_guard_text` (engine.rs:178); never appended to `text`. Tests: provenance.rs V3.2 (off→None, body==plain), V3.3 (on→Some, body byte-identical). |
| FR-023 (additive, non-mutating) | IMPLEMENTED | `build_guard_text` reads names only; `text` computed independent of guard. Test: V3.3 body equality. |
| FR-024 (configurable, `{fields}` plain replacement) | IMPLEMENTED | `DEFAULT_GUARD_TEMPLATE` + `GuardConfig.template` override; `str::replace` (NOT minijinja) (provenance.rs:155). Test: V3.4 override → `"ATTN fields: ctx, q"`. |
| FR-025 (no sanitization of values) | IMPLEMENTED | Guard never accesses values; rendered `text` unchanged. Test: V3.5 `<script>` payload passes through verbatim. |
| FR-026 (in core crate, no pyo3/napi) | IMPLEMENTED | Cargo.toml has no FFI dep; `ci:check-ffi` covers `prompting-press-core`. |
| FR-027 (consume relocated 001 shape, not redefine) | IMPLEMENTED | `pub mod generated` is the moved codegen output (generated/prompt_definition.rs); consumer re-exports from kernel (prompting-press/src/lib.rs:20). `schemas/moon.yml` codegen-check repointed to kernel path. |
| FR-028 (5 structured error variants; no missing-default) | IMPLEMENTED | `KernelError` = UnknownVariant, ExcludedFeature, Parse, UndefinedVariable, Render (error.rs:19-56). No missing-default variant. |
| FR-029 (render-fixture regression set exists as a guard) | PARTIAL | Fixtures exist (`tests/fixtures/render/interpolation.json`, `conditional-loop.json`) + loader `load_regression_case`, BUT no test consumes them (loader is dead code). Equivalent template→output regression is covered via `load_def_fixture` in render.rs. See F-01. |

## Per-SC coverage

| SC | Status | Evidence |
|----|--------|----------|
| SC-001 (determinism, byte-identical + equal hashes) | COVERED | hashing.rs V1.2. |
| SC-002 (excludes 100% loop/set/block locals + globals) | COVERED | agreement.rs V2.2/V2.3/V2.4. |
| SC-003 (undeclared var detectable in required-roots) | COVERED | agreement.rs V2.6. |
| SC-004 (variant resolution: default/named/unknown, no silent named arm) | COVERED | render.rs V1.5 (None→root body, asserts `!= concise`); render_errors.rs V1.4 (unknown). |
| SC-005 (guard off==plain; on→separate field, body byte-identical) | COVERED | provenance.rs V3.2/V3.3. |
| SC-006 (analysis + provenance mutate nothing) | COVERED | agreement_purity.rs V2.5; provenance is `&def`-only. |
| SC-007 (zero pyo3/napi/FFI; no I/O/LLM/etc.) | COVERED | `ci:check-ffi` gate + Cargo.toml dep set; FR-005 negative review. |
| SC-008 (every excluded feature rejected, none renders/passes analysis) | COVERED | excluded_features.rs render + agreement paths over all 6 constructs. |
| SC-009 (undefined var → loud error 100%, never silent empty) | COVERED | render_errors.rs V1.7. |

## Constitution Alignment

| Principle | Status | Evidence |
|-----------|--------|----------|
| I — Shared core, structural parity | PASS | Single render/analysis/hash site; `BTreeSet` deterministic ordering; `sha2`; no time/random. Render/agreement performed once in Rust. |
| II — FFI isolation | PASS | Kernel Cargo.toml has no `pyo3`/`napi`; new deps `minijinja`(no default features)+`sha2` are pure-Rust; `ci:check-ffi` covers core + consumer. |
| III — Minimal boundary | PASS | No I/O/LLM/request-body/token-count/output-parse in any kernel module. `output_model` carried as opaque reference (lib.rs normative "does NOT do" section). |
| IV — Typed input / sound agreement + provenance | PASS | `undeclared_variables(false)` + env-derived allowlist; FR-016a parse-first short-circuit; provenance is metadata + opt-in additive guard, never silent mutation (FR-025 enforced, value pass-through tested). |
| V — Repo canonical; git owns versioning | PASS | No managed version axis; variants caller-owned; `template_hash`/`render_hash` each over a string; no `vars_hash` (compile-guarded). |
| VI — Per-language idiom | PASS (boundary respected) | Kernel returns native `KernelError`; normalization to `[{field,code,message}]` deliberately deferred to consumer (error.rs doc). |
| VII — JSON Schema single source | PASS | Kernel consumes relocated generated shape; codegen task + `schemas/moon.yml` freshness gate repointed to kernel path; generated file marked do-not-edit. |

No constitution conflict (no CRITICAL).

## Metrics

- FR coverage: 30/30 addressed; 29 IMPLEMENTED, 1 PARTIAL (FR-029) = **96.7% full, 100% addressed**.
- SC coverage: 9/9 COVERED = **100%**.
- Findings: 0 CRITICAL, 0 HIGH, 1 MEDIUM (F-01), 3 LOW.

## Verification Commands

- `mise exec -- cargo build -p prompting-press-core`: **not run** (no shell access in this agent session; build correctness inferred from static review — all symbols/types/imports resolve consistently).
- `mise exec -- cargo test -p prompting-press-core 2>&1 | grep "test result"`: **not run** (same). 41 tests reported green at the prior main-thread check; the 7 integration suites + 4 inline unit modules read here are internally consistent with passing.
- `mise exec -- moon run ci:check-ffi`: **not run**; gate script + Cargo.toml reviewed, expected pass.
