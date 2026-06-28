# Verify-Tasks Report — 006 Conformance corpus + cross-language hardening

**Date**: 2026-06-28 · **Scope**: `all` (base `origin/main` → HEAD `8919130` + uncommitted) · **Tasks**: 21 (all `[X]`)

> ⚠️ **FRESH SESSION ADVISORY**: This check ideally runs in a session separate from the implementing
> agent. It was delegated to a fresh `speckit-verify-tasks` subagent TWICE; both times the subagent
> emitted its commands as text and returned `tool_uses: 0` (the known subagent tool-channel glitch —
> `memory/speckit-workflow-gotchas`). It was therefore run on the MAIN thread, with the asymmetric error
> model applied strictly (when in doubt, flag) and every verdict grounded in the actual committed diff
> (`git show 8919130`) and live execution — not the implementer's self-report.

## Integrity Check

- ✓ `tasks.md` exists; all 21 tasks T001–T021 marked `[X]`.
- ✓ All key artifacts present on disk (corpus, 4 Rust test files, 2 Python + 2 TS runners, CI script + moon/ci.yml wiring).
- ✓ HEAD = `8919130` on branch `006-conformance-corpus`; all 24 implementation files appear in that commit's diff (Layer 2 — no phantom).

## Summary Scorecard

| Verdict | Count | Tasks |
|---|---|---|
| ✅ VERIFIED | 18 | T001–T009, T011–T013, T015–T019 (artifacts present + in-diff + symbols found + behavior runs) |
| ✅ VERIFIED (behavioral) | 3 | T010, T014, T021 — verification tasks; evidence = the gate runs green + seeded-divergence fails all three bindings (re-executed this pass) |
| ⏭️ SKIPPED | 0 | — |
| 🔍 PARTIAL | 0 | — |
| ⚠️ WEAK | 0 | — |
| ❌ NOT_FOUND | 0 | — |

**Flagged items: 0.** No phantom completions detected.

## Per-Layer Evidence (highlights)

- **Layer 1 (file existence):** every referenced artifact present (12 file-bearing tasks all ✓).
- **Layer 2 (git-diff cross-ref):** all 24 impl files appear in commit `8919130` — nothing marked done without a corresponding change.
- **Layer 3 (content symbols):** `RawVars`/`build_value`/`impl garde::Validate for RawVars` (T004); `regenerate_marshaling_goldens` + `#[ignore]` (T005); `marshaling_fixtures_match_golden` (T007); `schema_round_trip_matches_verdict` (T011); `template_hash`/`render_hash` assertions in the Python runner (T008, 9 hits); `templateHash`/`renderHash` in the TS runner (T009, 7 hits); `conformance`/`regen` moon tasks (T016/T018); ci.yml `conformance` job (T017, real `runs-on` + steps).
- **Layer 4 (dead-code):** `not_applicable` — the deliverable IS test code (runners invoked by the test runner, not imported); `RawVars` is test-only; the `#[ignore]`d generator is intentional. None of these are dead code in the shipped sense.
- **Layer 5 (semantic):** the committed gate was re-executed this pass — `ci:conformance` green (Rust 2 + Python 17 + TS 16); `ci:check-ffi` PASSED; spec-002 render fixtures byte-unchanged; goldens are real 64-hex values, not placeholders. No stubs, no `TODO`/`unimplemented!`, no hardcoded-pass.

## Verdict

**All 21 tasks VERIFIED. Zero phantom completions, zero flagged items.** The implementation is backed by
real, executing code and the committed diff. No interactive walkthrough required (no flagged items). No
`/speckit.converge` needed (nothing unbuilt).
