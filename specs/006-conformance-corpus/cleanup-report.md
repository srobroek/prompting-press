# Cleanup Report — 006 Conformance corpus + cross-language hardening

**Generated**: 2026-06-28 · **Source**: post-implementation cleanup (Scout Rule) of spec 006
**Scope**: the 22 code/config files at commit `8919130`.

## Summary

| Category | Found | Fixed | Tasks Created | In Report |
|----------|-------|-------|---------------|-----------|
| Critical | 0 | – | – | – |
| Large | 0 | – | – | – |
| Medium | 0 | – | 0 | – |
| Small | 1 | 1 | – | – |

## Small Issues (fixed)

- **S1 — `packages/python/tests/test_conformance_marshaling.py:20`**: the module docstring's quick-reference
  table said `decimal → decimal.Decimal(value)`, but the code (and the authoritative DECISION block lower
  in the same docstring, and the `CONSTRUCTION` constant) use the **raw canonical string** for decimal.
  Fixed: collapsed the `datetime/date` and `decimal` summary lines into one accurate
  "`datetime / date / decimal → the canonical serialized string verbatim (see DECISION below)`". Cosmetic
  comment-rot; the rationale was already correct elsewhere in the file and is test-guarded by
  `test_canonical_type_construction_choice_is_recorded`. (Originated as `review.run` finding S1.)

## Scan results (no issues)

- **Debug artifacts**: none. (`eprintln!` in the `#[ignore]`d `conformance_goldens.rs` is intentional
  regeneration progress output, not a debug remnant.)
- **Dead code / unused imports**: none. (`RawVars` and the `common/mod.rs` helpers are each used by at
  least one runner; `#![allow(dead_code)]` is justified — each runner uses a subset.)
- **Dev remnants**: none. (The `mktemp -d /tmp/pp-conformance-XXXXXX` `XXXXXX` is mktemp's template
  placeholder, not an `XXX` marker. No TODO/FIXME/HACK, no localhost, no hardcoded secrets.)
- **Constitution**: no violations (no cleanup action conflicts with any MUST).

## Validation

- ✅ Python runner re-run after the edit: 6 passed.
- ✅ `ruff check` on the edited file: All checks passed.
- ✅ Full conformance gate after cleanup: `Conformance gate PASSED.` (no regression).

## Linter deference note

`ruff format` would reformat `test_conformance_schema.py` (and several **pre-existing** committed test
files), but the repo enforces **no** `ruff format` gate and the existing tests are not ruff-formatted — so
applying it would make the new file inconsistent with the surrounding committed code. Deliberately NOT
applied (Linter-Deference + "match the surrounding code"). Not a finding.

## Next steps

No tech-debt tasks created (no Medium/Large issues). Proceed to memory capture (step 17 area) → sync
analysis (steps 15/16) → retro (17) → docs (18) → final checkpoint (19).
