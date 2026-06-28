# Scope-Boundary & Parity-Claim Requirements Checklist: Conformance corpus + cross-language hardening

**Purpose**: "Unit tests for the requirements." Validate that the spec/plan/tasks specify (1) the
scope-boundary exclusions (no render-parity re-test; no engine logic / new public API in any binding) and
(2) the parity claims (marshaling parity, schema round-trip parity) precisely enough to be unambiguously,
measurably verifiable — BEFORE implementation.
**Created**: 2026-06-28
**Feature**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

## Scope-Boundary Discipline (render-parity exclusion, C-01 / Principle I)

- [ ] CHK001 Is the exclusion of comprehensive render-parity fixtures stated as a requirement (not just prose), and is it testable? [Clarity, Spec §FR-016]
- [ ] CHK002 Is "render parity is structural, not re-tested" defined with a concrete, checkable artifact boundary (the spec-002 render-fixture set stays byte-unchanged)? [Measurability, Spec §FR-016 / §SC-006]
- [ ] CHK003 Are the two things the corpus DOES test (marshaling, schema round-trip) and the one it does NOT (render parity) stated consistently across spec, plan, and tasks without drift? [Consistency, Spec Overview / Plan Summary / Tasks notes]
- [ ] CHK004 Is the boundary between "golden tripwire" and "comprehensive render-parity set" specified so the golden set cannot silently grow into the excluded category? [Ambiguity, Spec §FR-005 / §Assumptions hash-pinning]
- [ ] CHK005 Is there a requirement that makes the exclusion *observable in CI* (e.g. a scope-guard verification task), rather than relying on reviewer vigilance alone? [Coverage, Tasks T020]

## Scope-Boundary Discipline (no engine logic / no new public API, C-02 / Principle II,III)

- [ ] CHK006 Is "no engine logic in any binding/runner" defined precisely enough to adjudicate the one piece of glue (the Rust passthrough-Vars newtype)? [Clarity, Spec §FR-017 / Plan §D4]
- [ ] CHK007 Is the no-op `Validate` + Serialize-delegation newtype explicitly justified as zero-engine-logic, with the reasoning recorded (not left implicit)? [Completeness, Plan Complexity Tracking / research §D4]
- [ ] CHK008 Is "no new public API on any binding" stated as a requirement, distinguishing the runners (test harnesses, may read files) from the library (no I/O)? [Clarity, Spec §FR-018]
- [ ] CHK009 Is the `ci:check-ffi` gate's continued green status specified as a success criterion (so an accidental FFI/engine dependency fails)? [Measurability, Spec §SC-006]
- [ ] CHK010 Are the runners' permitted operations bounded by requirement (construct native values + call public API + compare), so "what a runner may do" is unambiguous? [Coverage, Contract §3 "Forbidden in a runner"]

## Parity-Claim Testability — Marshaling

- [ ] CHK011 Is "marshaling parity" defined as a verifiable equality (identical rendered text AND identical template_hash/render_hash), not a vague "behaves the same"? [Measurability, Spec §FR-005 / §SC-001]
- [ ] CHK012 Are all five hard cases (dates, Decimal, nested models, null/undefined/None, int-vs-float) enumerated as required coverage with at least one fixture each? [Completeness, Spec §SC-002 / Tasks T002]
- [ ] CHK013 For types lacking a universal native equivalent (date, Decimal), is the "same logical input" pinned via a canonical serialized form so the expectation is unambiguous across languages? [Ambiguity, Spec §FR-006 / research §D1]
- [ ] CHK014 Is the per-field `type`-tag mechanism specified well enough that each runner deterministically knows which native value to construct (date vs string both being JSON strings)? [Clarity, data-model §"Typed value descriptor" / research §D2]
- [ ] CHK015 Is the null/undefined/None/absent contract stated as a fixed cross-binding equality (null/None→JSON null; undefined/absent→field-not-present), and marked as pinned-not-redesigned? [Consistency, Spec §FR-008 / §Edge Cases]
- [ ] CHK016 Is the cross-check assertion model (three bindings each equal the one committed golden ⇒ transitive parity) specified, including how a golden is produced and why runners never regenerate at test time? [Completeness, research §D3 / Contract §4]
- [ ] CHK017 Is the requirement that hashes be stable across OS/architecture stated (no locale/line-ending/float-format dependence in the expected values)? [Coverage, Spec §FR-004 / §Edge Cases]
- [ ] CHK018 Is the requirement that runners drive each binding's REAL render path (not a hand-built kernel value bypass) stated, so the test exercises the marshaling code under test? [Clarity, Spec §FR-007]

## Parity-Claim Testability — Schema Round-Trip

- [ ] CHK019 Is "schema round-trip parity" defined as identical accept/reject verdict across all three bindings, driven through each binding's OWN loader (not one standalone validator)? [Clarity, Spec §FR-009]
- [ ] CHK020 Is the expected outcome for an invalid document specified as a structured rejection (no partial load, no crash), verifiable per binding? [Measurability, Spec §FR-010]
- [ ] CHK021 Is the requirement to exercise BOTH JSON and YAML loader paths stated, with how the fixtures provide a YAML case (the twin) documented? [Coverage, Spec §FR-011 / research §D6]
- [ ] CHK022 Is the reuse of the existing `schemas/jsonschema/fixtures/{valid,invalid}/` set (rather than forking a parallel set) stated as a requirement, with the accept/reject mapping defined? [Consistency, Spec §FR-003 / data-model §"Schema round-trip fixture"]

## CI-Gate Enforcement Measurability

- [ ] CHK023 Is "enforced as a CI gate" specified as a measurable outcome (runs on PRs; a divergence FAILS the build), not just "there is a gate"? [Measurability, Spec §FR-012 / §SC-007]
- [ ] CHK024 Is local reproducibility specified concretely (a documented `moon run` command that reproduces the CI result)? [Clarity, Spec §FR-013 / quickstart]
- [ ] CHK025 Is the requirement that the gate's failure output identify binding + fixture + divergence-kind stated, and bounded against leaking bound-value content beyond the fixture? [Completeness, Spec §FR-014]
- [ ] CHK026 Is the Rust consumer's first-class participation specified as a requirement (so parity is not asserted between only two of three bindings)? [Coverage, Spec §FR-015 / Plan §D5]
- [ ] CHK027 Is the gate's "the gate actually detects drift" property specified as a measurable success criterion (a seeded divergence MUST fail), so the gate cannot pass vacuously? [Measurability, Spec §SC-004]

## Dependencies & Assumptions

- [ ] CHK028 Are the dependencies on specs 004/005 (both implemented) stated, and is the empirical TS==Python hash parity recorded as the pre-existing fact this gate makes permanent? [Assumption, Spec §Dependencies / §Assumptions]
- [ ] CHK029 Is the assumption "no new shipped-library dependency" (esp. no JS decimal lib, no second YAML parser) stated, with the pin-exact discipline for any test-harness helper? [Dependency, Spec §Assumptions / Plan Constraints]
- [ ] CHK030 Are the two intentional plan-time deferrals (exact serialized representations; CI job topology) recorded as deferrals that change no acceptance criterion — i.e. not open ambiguities? [Ambiguity, Spec §Assumptions / checklists/requirements.md notes]

## Evaluation Result (2026-06-28)

**All 30 items PASS** — each references a requirement that is present, clear, consistent, and measurable
across spec/plan/tasks. No gaps, ambiguities, or conflicts found in the two focus areas. Evidence
spot-checked against the spec: the render-parity exclusion is a hard requirement (FR-016) with a checkable
artifact boundary (SC-006, spec-002 fixtures byte-unchanged) and a stated golden-set bound (FR-005); the
parity claims are equality assertions with enumerated hard cases (SC-002), canonical serialized forms
(FR-006/D1), the `type`-tag construction mechanism (D2), and the transitive cross-check-via-golden model
(D3); CI enforcement is measurable (SC-007 fail-on-divergence, SC-004 seeded-divergence-must-fail,
FR-013/quickstart local repro); the two plan-time deferrals are explicitly marked as changing no
acceptance criterion (not open ambiguities).

This is expected — the spec was authored directly against these risk areas — but the gate confirms it
rather than assuming it. No spec changes required before critique/security-review.

## Notes

- Check items off as evaluated: `[x]`. An item passes if the referenced requirement is present, clear,
  consistent, and measurable; record a finding inline if not.
- This checklist tests the REQUIREMENTS, not the implementation — it is run now (pre-implementation) and
  is distinct from the runners (which test the code later).
