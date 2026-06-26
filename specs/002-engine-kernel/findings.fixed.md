# Findings Fixed Log

> Resolved during Phase-3 QA (after /speckit.verify), 2026-06-26.

## Summary

- **Findings resolved**: 1 (F-01, MEDIUM)
- **Findings deferred**: 0
- **Final status**: CLEAN

## F-01 (MEDIUM, FR-029) — render-regression guard was unbacked

**Source**: `/speckit.verify` (verify-report.md).
**Finding**: The render-regression fixtures (`tests/fixtures/render/{interpolation,conditional-loop}.json`)
and the `common::load_regression_case` loader existed, but no test RENDERED them and asserted output ==
expected — `scaffold.rs` only checked deserialization, and `conditional-loop.json` was consumed by
nothing. FR-029's explicit engine-regression guard was therefore unbacked (behavior was incidentally
covered via render.rs def-fixtures, but the dedicated guard did nothing).

**Fix**: Added `crates/prompting-press-core/tests/render_regression.rs` —
`render_fixtures_match_pinned_output` iterates both render fixtures, builds a default-arm
`PromptDefinition` from each fixture's `template`, renders via `render(&def, None, values, &no_guard())`,
and asserts `result.text == case.expected` byte-for-byte (+ variant == "default"). Now a real
regression guard; genuinely exercises the previously-near-dead loader. No fixture `expected` values
needed correcting (both already matched real render output — assertions are meaningful, not tautological).

**Verification**: 42 tests green (new `render_regression` suite included), clippy -D warnings clean,
fmt clean. Only `crates/prompting-press-core/tests/` touched; no kernel src, no spec-doc change.
