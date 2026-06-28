# Specification Analysis Report — Spec 005 (TypeScript binding)

**Date**: 2026-06-27 · **Step**: 6 (`/speckit.analyze`) — cross-artifact consistency
**Artifacts analyzed**: spec.md, plan.md, tasks.md, research.md, data-model.md, contracts/ts-api.md,
quickstart.md, checklists/binding.md, constitution.md, both roadmaps, live repo state.

> **Provenance**: the analyze subagent's 6 findings were ALL re-verified main-thread against source
> before acting (standing rule — active subagent-fabrication glitch). This cycle the subagent was
> **accurate**: F1/F2/F3/F4 confirmed true against `scripts/ci/check-ffi-isolation.sh`,
> `packages/typescript/package.json`, and `docs/research/roadmap.md`. All actionable findings were fixed
> in this cycle (not deferred).

## Findings & resolutions

| ID | Severity | Finding (verified) | Resolution |
|----|----------|--------------------|------------|
| F1 | HIGH | `ci:check-ffi` ALREADY asserts `napi` — `check-ffi-isolation.sh:28` ships `FFI_CRATES=("pyo3" "napi")` (spec 001). The spec/plan/tasks wrongly said "MUST be extended". | **Fixed**: reworded spec §Overview + FR-022, plan §approach/Constraints/Scope/Constitution-table, research D7, and **T028 reframed "extend" → "verify"** (only touch the script if a gap is found). |
| F2 | MEDIUM | `packages/typescript/package.json` pins `engines.node: ">=20"`, but spec/plan/quickstart said "Node 16+". | **Fixed**: all "Node 16+" → "Node 20+" (spec ×2, plan ×2, quickstart, tasks T026), matched to the scaffold. |
| F3 | MEDIUM | `docs/research/roadmap.md:76,85` (the design doc) still listed "token hook" for both bindings; the spec's assumption claimed it was already struck. (The `.specify/memory/roadmap.md` ledger WAS struck by T027 — two different files.) | **Fixed**: struck "token hook" from `docs/research/roadmap.md` lines 76 + 85 with the F4 rationale; corrected the spec Assumption to name both files accurately. |
| F4 | LOW | Spec hedged "pin Zod IF the floating-version gate covers package.json" — the gate scans the whole file, so the condition is "yes". | **Fixed**: dropped the hedge; spec now states Zod + npm deps MUST be pinned exact (gate scans whole `package.json`). |
| F5 | LOW | Two impl-time decisions open (napi structured-error mechanism D4; test-runner D6) — acknowledged in research as "open at impl time". | **Accepted** (no change): prerequisites for T006/T008 + T002/T010 respectively, enforced by task ordering. Flagged in the critique (E1/E6). |
| F6 | LOW | `outputModel`-never-parsed edge case has no dedicated assertion task (prose-only in T025). | **Accepted** (optional): absence-of-behavior; covered by the boundary prose. Can fold a one-line assertion into a TS test if desired — not required for the SC set. |

## Coverage Summary

- **25/25 FRs** have ≥1 task (100%). FR-022's task (T028) is now correctly framed as verify-not-extend.
- **11/11 SCs** mapped; T031 walks each. SC-007 framing corrected (F1).
- **0 duplications**, **0 unmapped tasks** (T001–T031 all map to a requirement/gate/SC).
- **2 acknowledged impl-time ambiguities** (F5), both gated by task ordering.

## Constitution Alignment

No CRITICAL violations. The plan's 8-principle Constitution Check is sound and verified against live code:
- **C-01** (no engine logic): binding marshals to `prompting_press_core::render` directly — kernel
  signature confirmed (`engine.rs:154`); consumer `render<V>` is garde-generic (`render.rs:72`), so the
  kernel-direct path is correct, not a workaround.
- **C-02** (FFI isolation): the gate already enforces `napi` + `pyo3` (which is *why* F1 is a documentation
  issue, not a compliance gap).
- **C-06/C-07/C-04**: all backed by reused, verified consumer surface (`ConsumerError`, `From<KernelError>`,
  `Finding`/`FindingKind`, `check`, `Registry`, `Composition`).

## Verdict

**PROCEED to taskstoissues / implementation.** No CRITICAL issues; the HIGH finding (F1) and both MEDIUMs
(F2/F3) are corrected in this cycle. The spec/plan/tasks are now consistent with each other AND with the
live repo (the gate, the scaffold's Node floor, both roadmap files).

## Metrics
- Requirements: 25 FR + 11 SC · Tasks: 31 · FR coverage: 100% · Ambiguities: 2 (impl-time) · Duplication: 0 · Critical: 0
