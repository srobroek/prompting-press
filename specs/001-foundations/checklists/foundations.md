# Foundations Requirements-Quality Checklist: Spec 001

**Purpose**: Unit-test the *requirements* of spec 001 (layout / schema / codegen / CI guardrails) for
completeness, clarity, consistency, measurability, and coverage — before implementation. Tests how
the requirements are *written*, not whether code works.
**Created**: 2026-06-25
**Feature**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

> Domain: foundations/infrastructure · Audience: implementer + reviewer · Depth: formal pre-impl gate.
> Findings recorded inline (✓ pass, ⚠ gap/ambiguity) with resolution where addressed this pass.

## Requirement Completeness

- [x] CHK001 Are the required workspace members enumerated explicitly (each crate + package)? [Completeness, Spec §FR-001..005] — ✓ FR-001..005 name all 7.
- [x] CHK002 Is the dependency direction between members specified (who may depend on whom)? [Completeness, Spec §FR-001/002/003] — ✓ kernel←consumer←bindings stated; plan §Structure encodes it.
- [x] CHK003 Are the JSON-Schema fields the schema must express fully listed? [Completeness, Spec §FR-010/010a/011a] — ✓ role, body, variables(type+provenance+constraints), variants, output_model, metadata, meta.
- [x] CHK004 Are the codegen outputs (one per language) and their determinism requirement specified? [Completeness, Spec §FR-014/015/016] — ✓.
- [x] CHK005 Are BOTH CI guardrails (FFI-isolation, codegen-freshness) defined with trigger + failure behavior? [Completeness, Spec §FR-018/019/020] — ✓.
- [ ] CHK006 Is the **scope of the FFI-isolation check** (which crates it covers) specified as a maintainable, explicit list? [Completeness, Spec Edge Cases] — ⚠ Spec edge-case notes the concern; FR-018 names only core + consumer. **Gap**: a future FFI-free crate wouldn't be covered. Acceptable for v1 (only these two crates exist) but the check's crate-list should be a reviewable artifact (plan D3 implies `cargo tree` per named crate). Flag for the implementer to make the list explicit.
- [x] CHK007 Are the validation fixtures (accept + reject cases) enumerated? [Completeness, Spec §FR-013, data-model.md matrix] — ✓ 8-case matrix.

## Requirement Clarity / Measurability

- [x] CHK008 Is "deterministic codegen" quantified objectively? [Measurability, Spec §FR-015, SC-003] — ✓ "re-run produces zero diff" — byte-level, testable.
- [x] CHK009 Is "buildable workspace" given an objective pass condition? [Measurability, SC-001] — ✓ single orchestrated command builds all active members.
- [x] CHK010 Is "marked as generated" given a concrete, checkable meaning? [Clarity, Spec §FR-016] — ✓ generated + predictable segregated location; reasonable (a header marker is the conventional realization).
- [ ] CHK011 Is "no diff" for the freshness gate precise about NEW/untracked files (partial regeneration)? [Clarity, Spec §FR-019] — ✓ resolved: FR-019 says "including partial regeneration"; plan D2 specifies `git add -N` to catch untracked. Pass.
- [x] CHK012 Is the reserved Go placeholder's exclusion objectively defined (not in build, no toolchain)? [Clarity, Spec §FR-005, FR-006] — ✓.
- [x] CHK013 Can each guardrail-failure message requirement be verified? [Measurability, Spec §FR-020, SC-004/005] — ✓ "names the invariant and location" + scratch-branch test.

## Requirement Consistency

- [x] CHK014 Do the spec, plan, and tasks agree on the crate names and layout? [Consistency] — ✓ identical across spec §entities, plan §Structure, tasks T005–T011.
- [x] CHK015 Are the codegen tools consistent between research, plan, and tasks? [Consistency] — ✓ datamodel-code-generator 0.65.1 / json-schema-to-typescript 15.0.4 / cargo-typify 0.7.0 in all three.
- [x] CHK016 Do the schema's variant rules align with the constitution/clarifications? [Consistency, Spec §Clarifications, contracts/] — ✓ root-body-default, reserved `default`, per-variant body-only — schema matches clarifications.
- [ ] CHK017 Is the schema's **physical location** consistent across artifacts? [Consistency] — ⚠ minor: the contract lives at `specs/001-foundations/contracts/`, the deliverable at `schemas/jsonschema/`. Spec §FR-008 and tasks T015 both say `schemas/jsonschema/` is the home and T015 promotes the contract there — consistent and intentional, but a reviewer could misread the contracts/ copy as the deliverable. Resolved by T015's explicit "promote" wording; no spec change needed.

## Scenario / Edge-Case Coverage

- [x] CHK018 Are reject-cases specified for each schema constraint (role, provenance, named-default, per-variant extras)? [Coverage, Spec §FR-013] — ✓ matrix covers each.
- [x] CHK019 Is the distinction between "valid JSON but schema-invalid" and "not parseable" addressed? [Edge Case, Spec §Edge Cases, data-model.md] — ✓ both fixtures present.
- [x] CHK020 Is the "schema edited but only some artifacts regenerated" case covered by a requirement? [Coverage, Spec §Edge Cases, FR-019] — ✓ partial-regeneration explicitly gated.
- [ ] CHK021 Is the **codegen-tool determinism risk** (formatter/toolchain version drift) captured as a requirement, not just research prose? [Coverage, research D1] — ⚠ research D1 + memory-synthesis flag it (pin rustfmt/Prettier/black); tasks T020–T022 encode the pins. It lives in plan/tasks, not as a spec FR. Acceptable: it's an implementation constraint, correctly in the plan layer. Pass-with-note.
- [x] CHK022 Are negative-scope requirements (no render/validate/IO) stated and verifiable? [Coverage, Spec §FR-021/022, SC-007] — ✓ explicit negative FRs + a negative-scope success criterion.

## Dependencies & Assumptions

- [x] CHK023 Are the deferred decisions (codegen tooling = Q1) explicitly assumed/resolved rather than silently chosen? [Assumption, Spec §Assumptions, research.md] — ✓ Assumptions defer to planning; research resolves with citations.
- [x] CHK024 Is "published schema" disambiguated (committed-with-$id vs external endpoint)? [Ambiguity, Spec §Assumptions] — ✓ resolved in Assumptions = committed + stable $id.
- [x] CHK025 Are out-of-scope items that could be mistaken as in-scope (registry reservation) explicitly excluded? [Boundary, Spec §Assumptions, tasks T032] — ✓ registry reservation deferred to spec 007.
- [ ] CHK026 Is the assumption that orchestration tooling (moon, Cargo, maturin, napi-rs) pre-exists from the bootstrap validated? [Assumption, Spec §Assumptions] — ⚠ Spec assumes they exist; moon + Cargo are confirmed present, but **maturin and the napi-rs CLI presence is unverified** at spec time. Low risk (both are standard, installable), but the implementer should verify availability in T009/T010 rather than assume. Flag.

## Traceability

- [x] CHK027 Does every functional requirement trace to at least one task? [Traceability] — ✓ FR-001..022 map to T001–T031 (layout→T005-14, schema→T015-19, codegen→T020-27, CI→T028-31).
- [x] CHK028 Does each success criterion (SC-001..007) have a verifying task? [Traceability] — ✓ SC-001→T014, SC-002→T019, SC-003→T027, SC-004/005→T031, SC-006→T019, SC-007→T033.
- [x] CHK029 Do requirements trace to governing constitution decisions? [Traceability] — ✓ FR-018→C-02, FR-008-019→C-07, layout→C-01, negative scope→Principle III.

## Notes

- **Verdict: PASS as a pre-implementation gate.** 24/29 clean; **5 items flagged (CHK006, 017, 021, 026)** are not blockers — they are minor maintainability/verification notes for the implementer, three of which (017, 021) are already resolved by task wording and one (011) confirmed precise. None require a spec edit.
- **Two genuine implementer to-dos surfaced** (carry into implementation, not spec rework):
  1. **CHK006** — make the FFI-isolation check's covered-crate list an explicit, reviewable artifact (so a future crate doesn't silently escape the gate).
  2. **CHK026** — verify maturin + napi-rs CLI availability during T009/T010 rather than assuming the bootstrap provides them.
- No ambiguities, conflicts, or coverage gaps rise to the level of blocking `/speckit.analyze` or implementation.
