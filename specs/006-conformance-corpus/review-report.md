# PR Review Summary — 006 Conformance corpus + cross-language hardening

**Date**: 2026-06-28 · **Scope**: the 22 code/config files committed at `8919130` (corpus + 4 Rust test
files + 4 Python/TS runners + CI glue). `specs/` docs excluded.
**Method note**: the `.specify/scripts/bash/detect-changed-files.sh` the skill references does not exist in
this repo; scope taken from `git show 8919130`. All six review lenses (code, comments, tests, errors,
types, simplify) were applied on the main thread — the review surface is small and homogeneous (test code
+ fixtures + CI glue), and the review-subagent tool path has been hitting the `tool_uses:0` channel glitch
this session, so a consolidated main-thread review is the reliable equivalent. Config: all six agents
enabled.

## Critical Issues (0)

None.

## Important Issues (0)

None.

## Suggestions (2 — LOW, optional)

- **[comments] S1 — minor docstring drift in `test_conformance_marshaling.py`.** The module docstring's
  quick-reference table (line ~20) still says `decimal → decimal.Decimal(value)`, but the code (line ~95)
  and the `CONSTRUCTION` constant correctly use the **raw string** for decimal (and the DECISION block
  lower in the same docstring explains exactly why). So the authoritative rationale IS present and
  correct — only the one-line summary table lags it. Cosmetic; the `CONSTRUCTION`-vs-docstring drift is
  already guarded by `test_canonical_type_construction_choice_is_recorded`. Fix: align that one table line
  to "raw string (see DECISION)". Not worth blocking.
- **[simplify] S2 — `date` is passed as a raw string in all three runners even though a native
  `date`/`Date` value reproduces the golden for the date-only case.** This is a deliberate consistency
  choice (the fixture contract is "the value IS the canonical string"), documented in each runner. No
  change recommended — uniform handling across the three canonical-serialized types is clearer than a
  per-type special case, and it is the more conservative choice. Noted only for completeness.

## Strengths

- **[code] Zero engine logic in any runner** — all three call the binding's real public render/loader
  (Rust `prompting_press::render`, Python `prompting_press.render`, TS facade `render`); the one Rust
  `RawVars` newtype is test-only Serialize-delegation + no-op garde Validate. C-02 held cleanly.
- **[tests] The runners ARE the tests and they are genuinely adversarial** — they assert byte-identity
  against a committed golden, and the seeded-divergence check (verified) proves they fail loudly. The TS
  `assertDateDiverges()` runtime probe is a standout: it proves the string-vehicle workaround is still
  necessary and will flag if a future Node makes `Date` converge — self-documenting and self-checking.
- **[errors] Reject paths assert on error TYPE, never message text** (SEC-002), across all three schema
  runners — so the tests cannot couple to (or pressure a reviewer to weaken) the SEC-004 scrub. Failure
  messages name case + divergence-kind (FR-014) without leaking beyond fixture content.
- **[errors/security] SEC-001 path confinement is thorough** — the Python `_safe_resolve` rejects
  absolute paths and `..` segments BEFORE any filesystem read, then double-checks resolved-path
  containment under the repo root. Rust + TS mirror it.
- **[types] The fixture/type-tag model is sound** — recursive `{type,value}` descriptors with a small
  fixed tag vocabulary; `absent` correctly omits the key (distinct from explicit `null`→`None`/JSON null).
  Consistent across all three runners.
- **[comments] Comment density and accuracy are high** — each file's header explains what it guards
  (Principle VII), why render parity is NOT re-tested (Principle I), and the canonical-serialized-form
  decision with the empirical evidence. A future maintainer cannot mistake the intent.
- **[code] Robust repo-root discovery** in all three (walk up to the `conformance/` ancestor) — no
  hardcoded `..` counts; survives package relocation.

## Recommended Action

**No critical or important issues — ready to proceed.** The two LOW suggestions are optional polish; S1
(one stale docstring summary line) is the only one worth a touch, and it is non-blocking and already
guarded by a test. I will fold S1 into the `cleanup` step (14) rather than routing to `fix-findings`
(which is for material findings). Re-running review after that is not warranted for a one-line comment.

## Lens Coverage

| Lens | Verdict |
|---|---|
| code (guidelines/bugs/quality) | ✓ clean — C-01/C-02 upheld, no bugs |
| comments | 1 LOW (S1 docstring summary-line drift) |
| tests | ✓ strong — adversarial, seeded-divergence-proven |
| errors | ✓ clean — assert-on-type, SEC-001/002, named failures |
| types | ✓ clean — sound fixture/tag model |
| simplify | 1 noted (S2 — no change recommended) |
