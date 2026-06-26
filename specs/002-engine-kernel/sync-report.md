# Sync / Drift Report — Spec 002 (Engine kernel)

**Date**: 2026-06-26 · Step 15 (spec↔code drift). Run on the main thread (the `speckit-sync` subagent
hit the recurring tool-channel glitch, `tool_uses: 0`).

## Verdict: ✅ No material drift — spec and code are in sync.

| Type | Count |
|------|-------|
| STALE_SPEC | 0 |
| MISSING_CODE | 0 |
| UNSPECCED_SCOPE | 0 |
| ARTIFACT_STALE | 0 (1 cosmetic note) |

## Checks

- **Contract signatures (`contracts/kernel-api.md`) vs as-built (`src/*.rs`):** match. `render`,
  `get_source`, `required_roots`, `provenance_view`, `RenderResult`, `GuardConfig` all present as
  documented. The contract carries an explicit caveat (lines 5–6): *"Signatures are illustrative … not
  final names … exact identifiers are settled in implementation."* So any minor naming drift is
  caveat-permitted, not a contradiction.
- **Review-cycle type evolution (Default on GuardConfig; PartialEq/Eq on RenderResult/KernelError):**
  no drift — neither `data-model.md` nor the contract asserts derive-level details (they describe
  fields + behavior), so the added derives create no contradiction. (Cosmetic note: data-model could
  optionally mention the derives, but their absence is not a false statement.)
- **MISSING_CODE:** none. All 30 FRs implemented (verify step 11 = 100%); the one prior gap (FR-029,
  finding F-01) was fixed — `tests/render_regression.rs` now renders + asserts the fixtures.
- **UNSPECCED_SCOPE:** none. The public API surface (4 capabilities + their types + `KernelError`)
  matches the contract; no kernel capability exists beyond the spec.
- **Tasks:** all 36 `[X]`; each task's named files/artifacts landed (confirmed in verify-tasks step 10).
- **Generated-shape relocation:** a planned 002 action (research D6, FR-027), fully documented in
  plan/research; freshness gate repointed and green. Not drift.

## Conclusion

No reconciliation needed. The implementation faithfully tracks the (refined) spec.
