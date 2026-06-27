# Binding Requirements-Quality Checklist: Python binding (`prompting-press-py`)

**Purpose**: Unit-tests-for-requirements over spec + plan + tasks before critique/security-review.
Validates that the binding's distinctive requirements (FFI boundary, cross-binding error contract,
marshaling fidelity, boundary discipline) are complete, clear, consistent, and measurable.
**Created**: 2026-06-27
**Feature**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

**Focus**: FFI isolation / no-logic-duplication · error-contract parity · marshaling fidelity · boundary
discipline (no I/O / no token surface) · clarified-decision completeness. **Depth**: release-gate.
**Audience**: PR reviewer.

## FFI Isolation & No-Logic-Duplication (C-02 / C-01)

- [ ] CHK001 Is the "marshaling + facade only, zero engine logic" boundary stated as a testable requirement (not just prose)? [Clarity, Spec §FR-011/FR-022]
- [ ] CHK002 Are the exact crates allowed to depend on an FFI toolkit enumerated, and is the gate that enforces it named? [Completeness, Spec §FR-022, Plan §Constitution]
- [ ] CHK003 Is "render parity is structural, not re-tested" stated explicitly so a reviewer doesn't expect Python render-equality tests? [Clarity, Spec §Overview/Governance]
- [ ] CHK004 Are the specific operations that MUST be FFI calls (render/agreement/variant/hash) listed, vs. what the binding may add (facade/marshal/exceptions/packaging)? [Completeness, Spec §FR-011]
- [ ] CHK005 Is there a requirement that the `ConsumerError`→exception translation be exhaustive over the closed Rust enum (so a new variant fails the build, not silently)? [Coverage, Gap → Tasks T006]

## Cross-Binding Error Contract (C-06)

- [ ] CHK006 Is the `code` vocabulary specified as a single closed set shared with the Rust consumer (enumerated, not "similar")? [Clarity, Spec §FR-014]
- [ ] CHK007 Are the exception hierarchy's base + subtypes and their 1:1 mapping to `ConsumerError` variants fully specified? [Completeness, Spec §FR-014, Clarifications Q2]
- [ ] CHK008 Is the `{field, code, message}` row shape defined identically for both the Pydantic-validation source and the kernel-error source? [Consistency, Spec §FR-014]
- [ ] CHK009 Is the requirement that native types (`pydantic.ValidationError`, Rust errors) never appear on the public API stated measurably (i.e. assertable)? [Measurability, Spec §FR-004/FR-014, SC-006]
- [ ] CHK010 Is the Pydantic `ValidationError` → row mapping (loc → field, "validation" code) specified, so validation errors look identical across bindings? [Gap, Spec §FR-002/FR-014]

## Marshaling Fidelity (FR-003a)

- [ ] CHK011 Are the value types that must marshal losslessly enumerated (None/null, int vs float, bool, nested, date/Decimal)? [Completeness, Spec §FR-003a, Edge Cases]
- [ ] CHK012 Is the `model_dump` mode (json vs python) decision criterion specified, or explicitly deferred to an impl-pinned test? [Clarity, Plan/Research §D2]
- [ ] CHK013 Is "the caller MUST NOT hand-build a value map" stated as a requirement (the facade obligation)? [Clarity, Spec §FR-003a]
- [ ] CHK014 Is the boundary between own-path marshaling (this spec) and the broad conformance corpus (spec 006) drawn so coverage scope is unambiguous? [Scope, Spec §Assumptions/Edge Cases]

## Validation Ownership & Timing (Q1)

- [ ] CHK015 Is "the binding owns validation at the render boundary" stated unambiguously, including what `render` accepts (model+data and/or instance)? [Clarity, Spec §FR-002, Clarifications Q1]
- [ ] CHK016 Is "validate once, before any templating; on failure no render" specified measurably (the kernel is provably never reached)? [Measurability, Spec §FR-002, SC-002]
- [ ] CHK017 Is the three-sets invariant (Vars field names vs declared `variables`) documented with its non-silent failure mode (`undefined_variable`, not empty render)? [Completeness, Spec §Edge Cases/Assumptions]

## Boundary Discipline (C-03 / F4)

- [ ] CHK018 Is "no token counter / no token hook" stated as an explicit out-of-scope requirement with the F4 rationale, not merely omitted? [Clarity, Spec §FR-023/Assumptions]
- [ ] CHK019 Is the stale roadmap "token hook" line called out for reconciliation rather than silently carried? [Conflict, Spec §Assumptions]
- [ ] CHK020 Are no-I/O, no-model-calls, no-request-body, no-output-parsing stated as binding requirements (inherited, not assumed)? [Completeness, Spec §FR-023]
- [ ] CHK021 Is `output_model` specified as metadata-only (never resolved/parsed) at the binding layer? [Clarity, Spec §FR-023/Key Entities]

## Loader & Codegen (C-07 / Q3)

- [ ] CHK022 Is the loader-locus decision (reuse Rust loader via FFI) specified, including how the constructed-object path routes through it? [Completeness, Spec §FR-005, Clarifications Q3]
- [ ] CHK023 Is YAML↔JSON parity stated as a structural property (one loader), with measurable acceptance (identical representation + render)? [Measurability, Spec §FR-006, SC-003]
- [ ] CHK024 Is the generated-shape codegen requirement (never hand-edited, freshness-gated) stated, distinct from the application-authored Vars models? [Consistency, Spec §FR-008/FR-024]
- [ ] CHK025 Is malformed-input behavior (`LoadError`, nothing partially loaded) specified for all three input forms? [Coverage, Spec §FR-007]

## Packaging & Versioning (Q4)

- [ ] CHK026 Is the abi3 floor (CPython 3.10) specified as the install floor, with the latest-stable build target distinguished from it? [Clarity, Spec §FR-021, Clarifications Q4]
- [ ] CHK027 Is the scaffold's prior three-way version disagreement (abi3-py39 vs requires-python vs codegen) recorded as resolved, so it isn't re-litigated? [Conflict, Spec §Clarifications]
- [ ] CHK028 Is the buildable/installable/importable acceptance (SC-009) distinguished from publish (spec 007), so scope is bounded? [Scope, Spec §FR-021, SC-009]

## Acceptance Criteria Quality & Coverage

- [ ] CHK029 Does every Success Criterion (SC-001…SC-010) map to at least one functional requirement and one task? [Traceability, Spec §Success Criteria, Tasks]
- [ ] CHK030 Is the SEC-004 scrub stated as a measurable requirement (a seeded secret provably absent from the raised exception's text)? [Measurability, Spec §FR-015, Tasks T008]
- [ ] CHK031 Are the `check` finding kinds (undeclared / untrusted-without-guard / reserved-default / analysis-error) all enumerated as required outputs with deterministic order? [Completeness, Spec §FR-020]
- [ ] CHK032 Is the no-partial-as-success guarantee for composition stated measurably (one bad entry → exception, no partial list)? [Measurability, Spec §US4/SC-008]

## Evaluation (2026-06-27)

Ran the 32 items against `spec.md` + `plan.md` + `tasks.md`. **Outcome: PASS (31 satisfied, 1 minor —
satisfied via tasks).**

- **Satisfied (31)** — verified against spec text: boundary/no-logic (FR-011/022, Overview/Governance),
  error contract (FR-014, Clarifications Q2, SC-006), marshaling fidelity (FR-003a, Edge Cases,
  Research D2), validation ownership (FR-002, Q1, SC-002), boundary discipline + token reconciliation
  (FR-023, Assumptions §Token-surface), loader/codegen (FR-005/006/007/008/024, Q3), packaging (FR-021,
  Q4, SC-009), SEC-004 scrub (FR-015), `check` finding kinds (FR-020), no-partial composition (US4/SC-008).
  All 10 SCs (SC-001…SC-010) trace to ≥1 FR and ≥1 task.
- **CHK005 (minor, satisfied via tasks)**: the *exhaustive* `ConsumerError`→exception translation
  ("a new variant is a compile error, not a fallthrough") is specified in **task T006** + memory, but is
  not elevated to a standalone spec FR. Acceptable for a binding spec — it is implementation discipline
  inherited from the closed-enum invariant (spec §FR-014 "closed `KernelError`"), pinned by the task.
  Not a requirements defect; no spec change required.

No `[Gap]`/`[Conflict]` item is left unresolved or silently passed.

## Notes

- Check items off as resolved: `[x]`. A `[Gap]`/`[Conflict]` item that cannot be satisfied from the
  spec text should route back through the spec (not be silently passed).
- This is a requirements-quality gate — items test whether the *requirements are well-written*, not
  whether code works (that is verify/qa, Phase 3).
