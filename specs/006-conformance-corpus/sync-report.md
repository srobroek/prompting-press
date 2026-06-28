# Sync Report — 006 Conformance corpus + cross-language hardening

**Date**: 2026-06-28 · **Scope**: spec↔code drift (step 15) + inter-spec conflicts (step 16)
**Method note**: run on the main thread (the sync-subagent path has been hitting the `tool_uses:0` channel
glitch this session). Read-only analysis against the committed diff `8919130`, the spec artifacts, the
roadmap ledger, and the shared schema/CI contracts.

## Part 1 — sync.analyze (spec ↔ code drift)

**2 drifts found and resolved** (both documentation catch-up — the as-built code is correct and verified;
the design-time `data-model.md` predated two empirical findings made during implementation). Neither is a
scope change (so no `iterate`) nor a code defect (so no `bugfix`) — the spec doc was updated to match the
verified as-built truth.

| # | Location | Drift | Resolution |
|---|----------|-------|------------|
| DR-1 | `data-model.md` type-mapping table (datetime/date/decimal rows) | Said "construct a NATIVE object" (`datetime.fromisoformat`, `new Date`, `Decimal`); as-built feeds the **canonical serialized string** because native objects recanonicalize (Pydantic `Z`/`1E-17`, JS `Date` `.000Z`) | Table updated to "the string verbatim" + an As-built note linking memory [[D1]]. |
| DR-2 | `data-model.md` schema-fixture coverage list | Listed `variant-named-default.json` as a loader `reject`; the loader ACCEPTS it (serde shape-validation ≠ JSON-Schema; the rule is enforced by `check()` downstream) | Removed from the reject list + an As-built note linking memory [[A1]]. The implemented manifest already excluded it. |

Spec/plan/tasks otherwise consistent with the code (19/19 FR + 7/7 SC mapped — verify-report.md).
`plan.md` and `spec.md` were already correct (the int-vs-float `2.5` fix landed during the critique pass).

## Part 2 — sync.conflicts (inter-spec contradictions)

**Zero conflicts.**

1. **Roadmap §278 alignment is exact.** What shipped matches scope-in verbatim — FFI-marshaling fixtures
   over the five named cases (datetime/Date/chrono, Decimal, nested, null/undefined/None, int-vs-float),
   schema round-trip fixtures, CI gate across the three packages. Scope-out (comprehensive render-parity
   fixtures) correctly excluded; the spec-002 engine-regression render set is untouched. Depends-on
   (004, 005) satisfied; governed-by (C-01, C-07) upheld.
2. **No shared contract touched.** Spec 006 only ADDED test files, the `conformance/` corpus, and CI
   wiring — it modified no kernel/consumer/binding source (`crates/*/src`, `packages/*/{python,src}`) and
   not the JSON Schema. It therefore cannot contradict the interfaces specs 002–005 own. The fixtures
   *reuse* spec-001's schema fixtures (don't fork them) and *consume* the specs 003/004/005 public APIs
   as-is — no redefinition.
3. **Constitution consistency.** Principles I/II/III/VII upheld (verify-report.md + both security reviews);
   the corpus is the literal realization of Principle VII's "conformance corpus … guards the FFI boundary
   + schema round-trip, NOT render parity."

## Verdict

Drift resolved (2 doc updates); no inter-spec conflicts. The spec artifacts and the implementation are now
in sync, and 006 is consistent with the roadmap and every other spec. Proceed to retro (step 17).
