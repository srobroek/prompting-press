# Specification Analysis Report — 006 Conformance corpus + cross-language hardening

**Date**: 2026-06-28 · **Gate**: `/speckit.analyze` (cross-artifact consistency + coverage + constitution)
**Artifacts**: spec.md (19 FR, 7 SC, 3 user stories), plan.md, tasks.md (21 tasks), constitution v1.1.0,
data-model.md, contracts/corpus-format.md, research.md
**Method note**: the analyze pass ran as a forked subagent; per the project's subagent-fabrication guard,
the load-bearing MEDIUM finding (F1) was **independently re-verified on the main thread** before action
(confirmed: contract §5 named a single `conformance` test target; tasks define three distinct targets —
`conformance_marshaling`/`conformance_schema`/`conformance_goldens` — with no `conformance.rs`). The repo
facts the subagent cited (existing schema fixtures, render/loader entry points, `RenderResult` hash
fields, the `cargo test -p prompting-press` CI gap) were already confirmed by direct source reads earlier
this session.

## Findings

| ID | Category | Severity | Location | Summary | Status |
|----|----------|----------|----------|---------|--------|
| F1 | Inconsistency | MEDIUM | contracts/corpus-format.md §5 vs tasks.md T015/T005/T007/T011 | Rust-leg test-target naming drift: contract said `--test conformance`; tasks split into `conformance_marshaling` + `conformance_schema` (+ `#[ignore]`d `conformance_goldens`). No `conformance.rs` exists. Tasks were internally consistent (T015 already used the split names). | **RESOLVED** — contract §5 updated to `--test conformance_marshaling --test conformance_schema`, goldens target noted as not gated. |
| F2 | Coverage gap | LOW | spec FR-019; tasks | FR-019 (corpus measures binding fidelity; expected values derived from the shared core; never proposes an alternative rendering) has no dedicated task — satisfied **structurally** by the golden-provenance design (goldens generated from the Rust reference binding, T005/T006). | Accepted — design-enforced, no task needed. |
| F3 | Coverage gap | LOW | spec FR-004; tasks T020/T021 | FR-004 (OS/arch hash stability) is not exercised by a per-OS conformance run (runners run one Linux leg each). | Accepted — discharged by the canonical-serialized-form design (SHA-256 over canonical strings removes locale/line-ending/float-format variance); no per-OS run planned by design. |
| F4 | Underspecification | LOW | tasks T008/T009 | Python/TS marshaling runners are told to "consult the signature" for the static-data render path rather than naming the exact call. | Accepted — implementer reads the pinned entry point; plan already names it. |
| F5 | Inconsistency | LOW | CLAUDE.md (embedded constitution v1.0.0) vs `.specify/memory/constitution.md` v1.1.0 | The constitution mirror embedded in CLAUDE.md lacks the v1.1.0 Principle VI options-object bullet. | Out of scope for 006 (006 doesn't touch that rule); flagged for a separate CLAUDE.md/AGENTS.md refresh. |

**No CRITICAL or HIGH findings. No constitution MUST violations. No duplications. No ambiguous/
unmeasurable success criteria. No unresolved placeholders.**

## Coverage Summary (Requirements → Tasks)

100% of the 26 requirements (19 FR + 7 SC) are covered: 24 explicitly mapped to tasks, 2 (FR-004, FR-019)
satisfied implicitly by design (see F2/F3). Mapping highlights:

- FR-001 → T001–T003 · FR-002 → T002 · FR-003 → T003 · FR-005 → T005–T010 · FR-006 → T002/T004 ·
  FR-007 → T007–T009 · FR-008 → T002/T007–T009 · FR-009/010 → T011–T014 · FR-011 → T003/T014 ·
  FR-012 → T015–T017 · FR-013 → T015/T016/T021 · FR-014 → T007/T010/T011 · FR-015 → T007/T011/T017 ·
  FR-016/017/018 → T020 · FR-004/019 → implicit (T021/T005).
- SC-001 → T007–T010 · SC-002 → T002/T010 · SC-003 → T011–T014 · SC-004 → T010/T021 · SC-005 → T021 ·
  SC-006 → T020 · SC-007 → T016/T017.

## Constitution Alignment

None violated. Render parity NOT re-tested (I/C-01); runners call only existing public APIs, zero engine
logic (II/C-02); no library boundary expansion (III); schema round-trip is the second guarantee (VII/C-07);
the one test-only `RawVars` newtype is correctly scoped (Serialize-delegation + no-op garde Validate, in
the test file). The plan's Constitution Check table maps all principles to PASS, consistent with the design.

## Unmapped Tasks

None. T001–T021 each map to setup, the fixture/golden foundation, a user story (US1/US2/US3), or a
polish/scope-guard concern. Phase grouping, `[P]` markers, and dependency ordering (T004→T005→T006
sequential; runners parallel; US3 after US1+US2) are coherent.

## Metrics

- Requirements: 26 (19 FR + 7 SC) · Tasks: 21 · Coverage: 100% (24 explicit + 2 design-implicit)
- Ambiguity: 0 · Duplication: 0 · Inconsistency: 2 (F1 resolved; F5 out-of-scope artifact) · Critical: 0

## Resolution & Next Actions

- **F1 (MEDIUM): RESOLVED** this pass — `contracts/corpus-format.md` §5 reconciled to the tasks' two-target
  Rust split (`conformance_marshaling`, `conformance_schema`), with the `#[ignore]`d goldens target noted
  as ungated. Tasks were already the consistent source of truth; this was a one-line doc-sync.
- **F2/F3/F4: accepted** as design-acceptable (structurally enforced / discharged by canonical-form design /
  implementer reads pinned entry point). No changes.
- **F5: out of scope** — CLAUDE.md/AGENTS.md constitution-mirror staleness, unrelated to 006. Noted for a
  separate refresh.
- **No CRITICAL/HIGH → cleared to proceed** to Phase 1 step 7 (`/speckit.taskstoissues`) then step 8
  (checkpoint), then Phase 2 implementation.

## Extension Hooks

No `before_analyze`/`after_analyze` hook is registered in `.specify/extensions.yml`. None dispatched.
