# Verify-Tasks Report — Spec 002 (Engine kernel)

**Date**: 2026-06-26 · **Scope**: `all` (branch `002-engine-kernel`, 7 commits vs `main` + working tree) · **Tasks checked**: 36 (all `[X]`)

> ⚠️ **Fresh-session note**: This phantom-completion sweep was run on the MAIN thread. The intended
> fresh-context subagent (`speckit-verify-tasks`) hit the known environment tool-channel glitch
> (`tool_uses: 0` — its bash/Read never executed; same failure the spec-001 worklog logged) and
> correctly refused to fabricate a verdict from non-functional tools. Per the worklog's established
> workaround, the sweep was run on the main thread instead, verifying strictly against **objective
> evidence** — `git diff main...HEAD`, `grep` over source, and a live `cargo test` run — NOT against
> the task-report files or recollection, to limit confirmation bias.

## Scorecard

| Verdict | Count |
|---------|-------|
| ✅ VERIFIED | 35 |
| ✅ VERIFIED (w/ trivial cleanup applied) | 1 (T003–T009 group) |
| 🔍 PARTIAL | 0 |
| ⚠️ WEAK | 0 |
| ❌ NOT_FOUND | 0 |
| ⏭️ SKIPPED | 0 |

**0 phantom completions.** Every `[X]` task is backed by real, committed implementation evidence.

## Evidence base

- **7 source files** changed in the kernel (`git diff main...HEAD --stat`): `error.rs` (+107),
  `engine.rs` (+307), `hashing.rs` (+60), `agreement.rs` (+168), `provenance.rs` (+156), `lib.rs`
  (+154), plus consumer `lib.rs` (re-export edge, ±17). 84 files total, 6376 insertions.
- **All 10 expected public symbols present** (Layer 3, `grep` over `crates/prompting-press-core/src`):
  `KernelError` (error.rs), `render`/`get_source`/`build_environment`/`resolve_variant`/
  `looks_like_excluded_feature` (engine.rs), `required_roots` (agreement.rs), `provenance_view`/
  `build_guard_text` (provenance.rs), `sha256_hex` (hashing.rs).
- **41 tests pass across 10 suites** (live `cargo test -p prompting-press-core`), backing the
  test-tasks (T013–T015, T021–T022, T027, T032) — these are real, not phantom.
- **Relocation (T003–T009) verified**: generated `prompt_definition.rs` now under
  `crates/prompting-press-core/src/generated/`; consumer re-exports it (2 refs to
  `prompting_press_core::generated` in consumer lib.rs); freshness-gate path repointed in both
  `schemas/moon.yml:42` and `schemas/scripts/codegen-check.sh:17`.
- **T036 polish artifacts present**: `deny.toml`, `scripts/ci/check-advisories.sh`,
  `ci:check-advisories` task in `ci/moon.yml:34` + wired into `.github/workflows/ci.yml:94`.
- **Verification/gate tasks** (T002, T009, T020, T026, T031, T034, T035): each asserts a gate/test
  that objectively exists and passes (build green, 5 CI gates green, 41 tests green) — VERIFIED, not
  taken on faith.

## One finding (resolved during sweep — not a phantom)

**Empty leftover directory** `crates/prompting-press/src/generated/` survived the T003 `git mv` (git
doesn't track empty dirs, so it was never committed and didn't appear in the diff, build, or the green
freshness gate). This is housekeeping leftover, NOT a phantom completion — the relocation genuinely
landed (file moved, consumer re-exports, gate works). Removed via `rmdir` during this sweep; build
re-confirmed green. No task verdict affected.

## Verdict lines

| Task | Verdict | Evidence |
|------|---------|----------|
| T001 | ✅ VERIFIED | minijinja+sha2 in Cargo.toml (committed cc49b4e); build green |
| T002 | ✅ VERIFIED | FFI gate green; pyo3/napi absent from kernel tree |
| T003–T009 | ✅ VERIFIED | relocation landed; empty leftover dir cleaned this sweep |
| T010 | ✅ VERIFIED | `KernelError` enum, 5 variants, error.rs |
| T011 | ✅ VERIFIED | `build_environment` (Strict undefined), engine.rs |
| T012 | ✅ VERIFIED | tests/common harness + fixtures; scaffold tests pass |
| T013–T015 | ✅ VERIFIED | render.rs/hashing.rs/render_errors.rs suites green |
| T016–T019 | ✅ VERIFIED | resolve_variant/get_source/sha256_hex/render in engine+hashing |
| T020 | ✅ VERIFIED | US1 suites pass |
| T021–T022 | ✅ VERIFIED | agreement.rs/agreement_purity.rs suites green |
| T023–T025 | ✅ VERIFIED | required_roots + env-allowlist + FR-016a short-circuit, agreement.rs |
| T026 | ✅ VERIFIED | US2 suites pass |
| T027 | ✅ VERIFIED | provenance.rs suite green |
| T028–T030 | ✅ VERIFIED | provenance_view/build_guard_text + render wiring |
| T031 | ✅ VERIFIED | US3 suite passes |
| T032 | ✅ VERIFIED | excluded_features.rs (6 constructs) green; heuristic fixed |
| T033 | ✅ VERIFIED | crate rustdoc + doctest; X1/SEC-002 invariant documented |
| T034 | ✅ VERIFIED | full gate suite green (build/tests/codegen/ffi/floating/advisories) |
| T035 | ✅ VERIFIED | SC-001..009 each mapped to a passing test (reconciliation report) |
| T036 | ✅ VERIFIED | cargo-deny advisory gate, green, wired into CI |

## Conclusion

No phantom completions. The kernel is genuinely implemented as specified. One trivial empty-dir
leftover was cleaned during the sweep. Safe to proceed to `/speckit.verify`.
