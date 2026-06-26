# Verify Report — Spec 003 (Rust consumer `prompting-press`)

**Gate**: post-implementation FR/SC adherence (SpecKit step 11). **Mode**: READ-ONLY source analysis.
**Date**: 2026-06-26 | **Branch**: `003-rust-consumer` | **Spec status assessed**: AS-REFINED
(analyze F1/F3/F4/F5/F7; FR-021/022 + SC-009 struck).

## Verdict

**0 CRITICAL, 0 HIGH.** The implementation matches the refined spec intent. All 24 live FRs are
IMPLEMENTED; all 8 live SCs are IMPLEMENTED with passing backing tests. The three orchestrator-flagged
judgment calls (eager validate-at-`append`, the `AnalysisError` finding kind, the `meta.guard`
convention) are sound, in-scope, and spec-consistent. The crate stays FFI-free, duplicates no kernel
logic, and the SEC-004 scrub is real (bound `detail: _`, fixed message, sentinel-secret tests).

> Build/test were **NOT RUN** in this read-only pass (no shell channel available to this agent). The
> assessment is by source inspection of the consumer + the kernel surface it wraps. The orchestrator's
> last-known state (86 workspace / 36 consumer tests green) is consistent with the code read. See
> Verification Commands.

## Metrics

| Metric | Value |
|---|---|
| FRs checked (live) | 24 (FR-001..024; 021/022 struck) |
| FR Implemented | 24 (100%) |
| FR Partial / Missing / Diverged | 0 / 0 / 0 |
| SCs checked (live) | 8 (SC-001..008; 009 struck) |
| SC Implemented | 8 (100%) |
| SC Inconclusive | 0 |
| CRITICAL findings | 0 |
| HIGH findings | 0 |
| MEDIUM / LOW / NOTE | 0 / 0 / 4 (notes only) |

## Findings

| ID | Category | Severity | Location | Summary | Recommendation |
|----|----------|----------|----------|---------|----------------|
| N-1 | Judgment call | NOTE | `check.rs:105-127, 219-234` | `FindingKind::AnalysisError` (3rd kind) records a `required_roots` `Err` instead of swallowing it; keeps `check()` infallible (`-> CheckReport`, F7). | Accept. In-scope: it strengthens the gate (a broken/excluded-feature template fails the lint loudly) without adding a public seam. Reason is scrubbed (variant class only), not the raw kernel `detail`. |
| N-2 | Judgment call | NOTE | `compose.rs:116-159, 26-40 (module docs)` | Composition validates + serializes **eagerly at `append`** (option a), not at `resolve`. | Accept. Honors "validate once, no-render-on-invalid, no-partial-as-success": an invalid entry never enters the `Vec`, so `resolve` only ever sees fully-validated entries. The shift from "validate at render" to "validate at append" is documented and the FR-002 *intent* (no render without validation) holds. Test `invalid_entry_vars_error_no_partial_success` pins it. |
| N-3 | Judgment call | NOTE | `check.rs:36-58 (module docs), 80-82, 289-293`; `lib.rs:87-95` | The `meta.guard` convention: a guard is "configured" iff a top-level `"guard"` key is present in `meta` OR `metadata`. | Accept. Documented in both `lib.rs` and `check.rs` module docs with explicit rationale (the kernel has no in-template guard-position concept — confirmed in `core/src/provenance.rs`). Presence-only, read-only, contents opaque. Sensible and the only implementable reading of C-09 against the v1 kernel surface. |
| N-4 | Doc drift (spec/quickstart, not code) | NOTE | `quickstart.md:52` | `quickstart.md` V3.3 still names the **pre-refinement** `Finding::UntrustedOutsideGuard{...}`; the implemented (and spec-FR-018) kind is `UntrustedWithoutGuard`. | Cosmetic stale label in an aux doc only; code + `spec.md` FR-018 + `data-model.md` + `contracts/` all use `UntrustedWithoutGuard`. Optionally fix in a `sync`/cleanup pass. Not an implementation defect. |

## Per-FR coverage

| FR | Status | Evidence |
|----|--------|----------|
| FR-001 typed Vars via garde + custom validators | IMPLEMENTED | `render.rs:71` bound `V: Serialize + Validate`; tests define `#[garde(custom(at_most_100))]` + `#[garde(length/range)]` (`tests/render.rs:23-42`). No bespoke framework. |
| FR-002 validate once, before render; fail → no render | IMPLEMENTED | `render.rs:83` `vars.validate().map_err(ConsumerError::from)?` *before* the kernel call at :92. Test `single_validation_failure_blocks_render` asserts the `Validation` variant (not `Kernel`) — proof the kernel was never reached (`tests/render_validation.rs:62-83`). |
| FR-003 validation in consumer; kernel validation-blind | IMPLEMENTED | Validation lives in `render.rs`/`compose.rs`; only `minijinja::Value` reaches `prompting_press_core::render`. Kernel is validation-blind by design (`core/src/lib.rs:48-51`). |
| FR-003a serialize validated struct → kernel value | IMPLEMENTED | `render.rs:87` `minijinja::Value::from_serialize(vars)`; `compose.rs:151` same. No hand-built value map. |
| FR-004 garde Report never on public API | IMPLEMENTED | `From<garde::Report>` in `error.rs:140`; public `render` returns `Result<RenderResult, ConsumerError>`. Compile-level pin `tests/render_validation.rs:116-128`. |
| FR-005 YAML/JSON/object → one PromptDefinition | IMPLEMENTED | `registry.rs:89 load_json`, `:113 load_yaml`, `:42 insert`; all produce the kernel's `PromptDefinition`. |
| FR-006 YAML ≡ JSON identical representation | IMPLEMENTED | `tests/loader.rs:71,106` compare re-serialized `serde_json::Value`s of YAML- vs JSON-loaded defs (structural, not smoke) over single-body AND multi-variant fixtures. |
| FR-007 malformed → structured error, nothing partially loaded | IMPLEMENTED | Loaders deserialize *then* insert (`registry.rs:90-92, 114-116`); on `Err` they return before `insert_and_get`. Tests assert `ConsumerError::Load` + registry untouched (`tests/loader.rs:194-249`). `#[serde(deny_unknown_fields)]` rejects shape violations. |
| FR-008 consume kernel shape, no parallel shape | IMPLEMENTED | `lib.rs:182-183` re-exports `prompting_press_core::...::PromptDefinition`; no parallel struct defined anywhere in the crate. |
| FR-008a registry (name→def); absent → structured error | IMPLEMENTED | `Registry { BTreeMap<String, PromptDefinition> }` (`registry.rs:26-29`); `render`/`get_source`/`resolve` map absent → `ConsumerError::UnknownPrompt` (`render.rs:75-77,110-112`; `compose.rs:190-192`). Test `unknown_prompt_is_structured_error`. |
| FR-009 render by registry name + vars; provenance + guard surfaced | IMPLEMENTED | `render.rs:63-93`; resolves by name (no "prompt handle" — F3); guard plumbed straight to kernel, `RenderResult.guard` surfaced. F5 plumb-only test `guard_config_is_plumbed_through` (`tests/render.rs:119-153`) asserts some/none, not kernel guard wording. |
| FR-010 get_source delegates to kernel | IMPLEMENTED | `render.rs:105-115` → `prompting_press_core::get_source`. No validation (no vars). |
| FR-011 no render/agreement/variant/hash logic duplicated | IMPLEMENTED | All such ops are kernel calls (`render.rs:92`, `compose.rs:198`, `check.rs:198 required_roots`, `:241 provenance_view`). `check()` is `BTreeSet` set-difference only. No hashing/MiniJinja-render code in the crate. |
| FR-012 ordered composition → ordered {role,text} | IMPLEMENTED | `Composition` is an ordered `Vec<Entry>` (`compose.rs:91-95`); `resolve` walks in append order producing `Message{role,text}` (`:185-214`). Test `ordered_composition_resolves_n_to_n` (N→N, append order, role from def). |
| FR-013 no fluent .chain() API | IMPLEMENTED | Only `new()` + `append(&mut self) -> Result<(), _>` (not `Self`), explicitly non-chainable (`compose.rs:135`). The only `.chain` in the crate is `Iterator::chain` (`check.rs:248`, `core build_guard_text`) — unrelated. |
| FR-014 normalize garde Report + KernelError → {field,code,message}; no leak | IMPLEMENTED | `error.rs:140 From<garde::Report>`, `:167 From<KernelError>` (exhaustive over the closed 5-variant enum). `ConsumerError`/`FieldError` are the only public error types. |
| FR-015 no raw bound-value detail in messages (SEC-004) | IMPLEMENTED | `error.rs:181,187,193` bind `detail: _` and emit fixed strings for `Parse`/`Render`/`ExcludedFeature`. Sentinel-secret tests `render_detail_secret_is_scrubbed` / `parse_detail_secret_is_scrubbed` assert the secret reaches neither the row nor `Display`. `check.rs:298 analysis_error_reason` likewise surfaces only the variant class. |
| FR-016 single check over registry; referenced ⊆ declared | IMPLEMENTED | `check.rs:171 check(&Registry) -> CheckReport`; `check_agreement` subtracts `def.variables.keys()` from kernel `required_roots` per variant (`:186-235`). |
| FR-017 roots from kernel; consumer owns subtraction; authority = def.variables | IMPLEMENTED | `required_roots` is the kernel call (`check.rs:198`); declared set is `def.variables.keys()` (`:188`) — the spec-001 shape, not the garde struct (clarify Q1). |
| FR-018 provenance lint: declared untrusted/external + no guard → finding | IMPLEMENTED | `check_provenance` (`check.rs:240-275`): `provenance_view().untrusted ∪ .external`; if non-empty and no `"guard"` key in `meta`/`metadata`, emit `UntrustedWithoutGuard` per field. Reframe is faithful to F1. Tests V3.3 + `external_field_obligation_and_metadata_guard_satisfaction`. |
| FR-019 check pure (no mutation/render/side-effects) | IMPLEMENTED | `check(&Registry)` (shared borrow — mutation impossible by type); only output is `CheckReport`. Behavioral backstop `tests/check_purity.rs` snapshots before/after over a failing registry and asserts byte-identical. |
| FR-020 findings actionable (prompt + variant + field) | IMPLEMENTED | `Finding { prompt, variant: Option<String>, kind, detail }` (`check.rs:86-97`); agreement findings name variant (Some), provenance prompt-level (None); `detail` names the offending var/field. Tests assert prompt/variant/name and that `detail` contains the field. |
| FR-023 FFI-free | IMPLEMENTED | `Cargo.toml` deps: `prompting-press-core`, `serde`, `serde_json`, `minijinja`, `garde 0.23 (derive,serde)`, `serde_yaml_ng 0.10` — no `pyo3`/`napi`. (Transitive absence is the `ci:check-ffi` gate — see Verification Commands.) |
| FR-024 no I/O; output_model metadata-only; no token counting | IMPLEMENTED | Loaders take `&str` (`registry.rs:89,113`), not paths; no file/net/db access anywhere; `output_model` carried on the re-exported shape, never parsed; no token-count seam (hook dropped — F4). |

> FR-021 / FR-022 (token-count hook) — STRUCK (F4). Correctly absent: no `tokens` module, no
> token-count type or seam in `lib.rs`/`Cargo.toml`. Not counted against coverage.

## Per-SC coverage

| SC | Status | Backing test(s) |
|----|--------|-----------------|
| SC-001 typed Vars → render, no kernel/native type on API, single validate-then-render path | IMPLEMENTED | `valid_vars_render_with_provenance`, `render_is_deterministic`, `public_return_type_is_normalized` |
| SC-002 invalid rejected before render, every offending field named | IMPLEMENTED | `single_validation_failure_blocks_render`, `multiple_validation_failures_all_reported` |
| SC-003 YAML/JSON/object identical representation + render | IMPLEMENTED | `yaml_and_json_parse_to_equal_definitions_single_body` + `_multi_variant`, `constructed_object_is_on_equal_footing` (structural `Value` compare) |
| SC-004 undeclared-var detected, names prompt/variant/var, mutates/renders nothing | IMPLEMENTED | `undeclared_variable_is_flagged`, `each_variant_is_analyzed_independently`, `check_is_pure_no_mutation` |
| SC-005 untrusted/external + no guard flagged, names prompt+field | IMPLEMENTED | `untrusted_without_guard_is_flagged`, `external_field_obligation_and_metadata_guard_satisfaction` |
| SC-006 no garde Report / KernelError on public API; all errors {field,code,message} | IMPLEMENTED | `public_return_type_is_normalized` (compile-level), `From` impls + closed `code` vocabulary in `error.rs` |
| SC-007 zero pyo3/napi/FFI deps; no render/agreement/variant/hash logic of its own | IMPLEMENTED | `Cargo.toml` (no FFI deps) + `ci:check-ffi` gate; FR-011 evidence (all kernel calls) |
| SC-008 N entries → exactly N ordered {role,text}, each with own validated vars | IMPLEMENTED | `ordered_composition_resolves_n_to_n`, `fragment_by_value_into_parent`, `empty_composition_resolves_to_empty_vec` |

> SC-009 (token hook) — STRUCK (F4). Correctly absent. Not counted.

## Constitution Alignment

| Principle | Requirement | Assessment |
|---|---|---|
| **I** Shared core, no duplication (C-01) | render/agreement/variant/hash live once in kernel | PASS. Every such op is a kernel call; `check()` is pure `BTreeSet` set-difference + a metadata-key presence test. No MiniJinja env, no SHA, no variant-resolution table in the crate. |
| **II** FFI isolation (C-02) | consumer free of pyo3/napi (direct + transitive) | PASS (direct: confirmed in `Cargo.toml`). Transitive absence is the `ci:check-ffi` gate's job — NOT RUN here; flagged in Verification Commands as the one externally-gated claim. garde + serde_yaml_ng are pure-Rust per research D1/D2. |
| **III** Minimal boundary (C-03) | no I/O, no LLM, no request-body, no token counting, no output parse | PASS. Loaders take `&str`; `output_model` echoed not parsed; token hook dropped (F4); no network/file/db. |
| **VI** Per-language idiom (C-06) | garde native; Vec+append not .chain(); errors normalized; native types don't leak | PASS. garde IS the validation system; composition is `Vec`+`append`; `ConsumerError` is the sole public error; closed `code` vocabulary; `Display` scrubbed. |
| **VII** JSON Schema single source (C-07) | dual-input loader into the one shape | PASS. YAML/JSON/object all normalize into the kernel's generated `PromptDefinition`; no parallel shape. |
| **IV / Scope Discipline (C-04/C-08/C-09)** | the sound check + provenance lint, pure; no new pluggable seam | PASS. `check()` surfaces both lints, pure. No new public seam introduced (the only candidate, the token hook, was dropped). `AnalysisError` is an internal finding kind, not a pluggable interface. |

No constitution conflict at any severity.

## Notable correctness checks performed (kernel-surface cross-verification)

- The consumer's `From<KernelError>` match (`error.rs:167`) and `analysis_error_reason`
  (`check.rs:298`) are **exhaustive** over the kernel's closed 5-variant `KernelError`
  (`core/src/error.rs:19-56`) with no wildcard — adding a kernel variant is a compile error, not a
  silent miss. Verified the variant set matches exactly.
- `meta`/`metadata` are `serde_json::Map` on the generated shape (`prompt_definition.rs:121,124`), so
  `contains_key(GUARD_KEY)` is valid; `variables`/`variants` are `HashMap` with `keys()`; `role` has a
  `Display` impl (`:250`); `name` is a transparent newtype deref-ing to `String`. Every field the
  consumer touches exists with the assumed type.
- FR-018 independence: the consumer reads `provenance_view` directly for the lint, not the kernel's
  render-time `build_guard_text` (which returns `None` on an empty union) — so the lint correctly owns
  its own decision and is not coupled to guard-text emission.
- The kernel docs themselves (`core/src/provenance.rs:72-77`, `lib.rs:57-75`) confirm provenance tags
  are declarative-only and the guard does not sanitize — which is exactly why FR-018's reframe ("you
  declared untrusted inputs and configured no guard") is the implementable reading, not a weakening.

## Verification Commands

| Command | Status | Note |
|---|---|---|
| `mise exec -- cargo build -p prompting-press` | NOT RUN | No shell channel in this read-only pass. Code compiles by inspection (types/traits line up with the kernel surface). |
| `mise exec -- cargo test -p prompting-press 2>&1 \| grep "test result"` | NOT RUN | 36 consumer tests read; all back live SCs. Orchestrator's last-known: 36 consumer / 86 workspace green. |
| `mise exec -- moon run ci:check-ffi` | NOT RUN | The authoritative SC-007/Principle-II *transitive* FFI gate. Direct deps confirmed FFI-free by reading `Cargo.toml`. Recommend the orchestrator confirm this gate is green before merge — it is the one claim not fully provable by source read alone. |

## Recommendation to orchestrator

Proceed. No CRITICAL/HIGH gaps; the refined spec intent is met. Two non-blocking follow-ups for a
later `sync`/`cleanup` pass: (N-4) fix the stale `UntrustedOutsideGuard` label in `quickstart.md:52`,
and confirm `ci:check-ffi` is green (the one transitive claim this read-only pass could not execute).
