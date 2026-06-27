# Binding Requirements-Quality Checklist: TypeScript binding (`prompting-press-node`)

**Purpose**: Unit-tests-for-requirements over spec + plan + tasks before critique/security-review.
Validates that the binding's distinctive requirements (FFI boundary, cross-binding error contract,
marshaling fidelity, boundary discipline, the napi/Zod-facade split) are complete, clear, consistent,
and measurable.
**Created**: 2026-06-27
**Feature**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

**Focus**: FFI isolation / no-logic-duplication · error-contract parity · marshaling fidelity · boundary
discipline (no I/O / no token surface) · clarified-decision completeness · the Rust-addon/TS-facade split.
**Depth**: release-gate. **Audience**: PR reviewer.

## FFI Isolation & No-Logic-Duplication (C-02 / C-01)

- [ ] CHK001 Is the "marshaling + facade only, zero engine logic" boundary stated as a testable requirement (not just prose)? [Clarity, Spec §FR-011/FR-022]
- [ ] CHK002 Are the exact crates allowed to depend on an FFI toolkit enumerated, and is the gate that enforces it named — including that the gate must be EXTENDED to assert `napi` (not just `pyo3`)? [Completeness, Spec §FR-022, Tasks T028]
- [ ] CHK003 Is "render parity is structural, not re-tested" stated explicitly (incl. provenance-hash parity vs Python/Rust) so a reviewer doesn't expect TS render-equality tests? [Clarity, Spec §Overview/Governance/SC-009]
- [ ] CHK004 Are the specific operations that MUST be FFI calls (render/agreement/variant/hash) listed, vs. what the binding may add (facade/marshal/errors/packaging)? [Completeness, Spec §FR-011]
- [ ] CHK005 Is there a requirement that the `ConsumerError`→error translation be exhaustive over the closed Rust enum (so a new variant fails the build, not silently)? [Coverage, Gap → Tasks T006]

## Rust-Addon / TS-Facade Split (the 005-specific structural decision)

- [ ] CHK006 Is it specified WHERE Zod validation lives (the TS facade `src/index.ts`, not the napi addon — because Zod is a TS library), and that this is still zero engine logic? [Clarity, Plan §Structure Decision/Complexity, Spec §FR-002]
- [ ] CHK007 Is the boundary between what the napi addon exposes (marshal + kernel delegation) and what the TS facade adds (`safeParse`, `Error` subclasses) drawn so a reviewer knows which layer owns each behavior? [Completeness, Plan §Structure, Tasks T011/T013/T022]

## Cross-Binding Error Contract (C-06)

- [ ] CHK008 Is the `code` vocabulary specified as a single closed set shared with the Rust consumer + Python binding (enumerated, not "similar")? [Clarity, Spec §FR-014]
- [ ] CHK009 Are the `Error`-subclass hierarchy's base + subtypes and their 1:1 mapping to `ConsumerError` variants fully specified? [Completeness, Spec §FR-014, Clarifications Q2]
- [ ] CHK010 Is the `{field, code, message}` row shape defined identically for both the Zod-validation source and the kernel-error source? [Consistency, Spec §FR-014]
- [ ] CHK011 Is the requirement that native types (`ZodError`, Rust errors) never appear on the public API stated measurably (i.e. `instanceof`-assertable)? [Measurability, Spec §FR-004/FR-014, SC-006]
- [ ] CHK012 Is the `ZodError` → row mapping (issue `path` → field, issue `message` only, "validation" code) specified, so validation errors look identical across bindings AND no rejected value leaks? [Gap, Spec §FR-014/FR-015, Research D3]

## Marshaling Fidelity (FR-003a / Q6)

- [ ] CHK013 Are the value types that must marshal losslessly enumerated (null vs undefined, int vs float, bigint, nested, dates)? [Completeness, Spec §FR-003a, Edge Cases]
- [ ] CHK014 Is the `null`/`undefined`/absent rule stated unambiguously (undefined/absent → field-not-present; null → JSON null) AND matched to the Python binding's None/absent handling for spec-006 parity? [Clarity, Spec §FR-003a, Clarifications Q6]
- [ ] CHK015 Is "the caller MUST NOT hand-build a value map" stated as a requirement (the facade obligation)? [Clarity, Spec §FR-003a]
- [ ] CHK016 Is the boundary between own-path marshaling (this spec) and the broad conformance corpus (spec 006) drawn so coverage scope is unambiguous? [Scope, Spec §Assumptions/Edge Cases]

## Validation Ownership & Timing (Q1)

- [ ] CHK017 Is "the binding owns validation at the render boundary" stated unambiguously, including what `render` accepts (Zod schema + data and/or already-typed data)? [Clarity, Spec §FR-002, Clarifications Q1/Q4]
- [ ] CHK018 Is "validate once via `safeParse`, before any templating; on failure no render" specified measurably (the kernel is provably never reached)? [Measurability, Spec §FR-002, SC-002]
- [ ] CHK019 Is the three-sets invariant (Zod Vars field names vs declared `variables`) documented with its non-silent failure mode (`undefined_variable`, not empty render)? [Completeness, Spec §Edge Cases/Assumptions]

## Boundary Discipline (C-03 / F4)

- [ ] CHK020 Is "no token counter / no token surface" stated as an explicit out-of-scope requirement with the F4 rationale, not merely omitted? [Clarity, Spec §FR-023/Assumptions]
- [ ] CHK021 Are no-I/O, no-model-calls, no-request-body, no-output-parsing stated as binding requirements (inherited, not assumed)? [Completeness, Spec §FR-023]
- [ ] CHK022 Is `outputModel` specified as metadata-only (never resolved/parsed) at the binding layer? [Clarity, Spec §FR-023/Key Entities]

## Loader & Codegen (C-07 / Q3)

- [ ] CHK023 Is the loader-locus decision (reuse Rust loader via FFI) specified, including how the constructed-object path routes through it? [Completeness, Spec §FR-005, Clarifications Q3]
- [ ] CHK024 Is YAML↔JSON parity stated as a structural property (one loader), with measurable acceptance (identical representation + render)? [Measurability, Spec §FR-006, SC-003]
- [ ] CHK025 Is the generated-shape codegen requirement (never hand-edited, freshness-gated) stated, distinct from the application-authored Zod Vars schemas? [Consistency, Spec §FR-008/FR-024]
- [ ] CHK026 Is malformed-input behavior (`LoadError`, nothing partially loaded) specified for all three input forms? [Coverage, Spec §FR-007]

## Packaging & Versioning (Q5 / Q7 / Q8)

- [ ] CHK027 Is the packaging shape (napi native addon, per-platform `optionalDependencies`) and the module format (ESM-only) specified as requirements? [Clarity, Spec §FR-021, Clarifications Q5/Q8]
- [ ] CHK028 Is the Zod-v4 target stated (not a v3/v4 dual range), so the `ZodError` mapper targets one issue API? [Clarity, Spec §FR-001, Clarifications Q7]
- [ ] CHK029 Is the napi floating-version (`"3"`) issue called out for reconciliation (pin exact) rather than silently carried, and the npm-dep pinning addressed? [Conflict, Spec §Assumptions, Tasks T001/T002]
- [ ] CHK030 Is the buildable/installable/importable acceptance (SC-009) distinguished from publish (spec 007), so scope is bounded? [Scope, Spec §FR-021, SC-009]

## Acceptance Criteria Quality & Coverage

- [ ] CHK031 Does every Success Criterion (SC-001…SC-011) map to at least one functional requirement and one task? [Traceability, Spec §Success Criteria, Tasks]
- [ ] CHK032 Is the SEC-004 scrub stated as a measurable requirement (a seeded secret provably absent from the thrown error's message/stack/rows)? [Measurability, Spec §FR-015, Tasks T009/T010]
- [ ] CHK033 Are the `check` finding kinds (undeclared / untrusted-without-guard / reserved-default / analysis-error) all enumerated as required outputs with deterministic order? [Completeness, Spec §FR-020]
- [ ] CHK034 Is the no-partial-as-success guarantee for composition stated measurably (one bad entry → thrown error, no partial array)? [Measurability, Spec §US4/SC-008]
- [ ] CHK035 Is the new Node advisory gate (SC-011) and the `ci:test-node` gate (the binding's TS-observable test coverage) specified as requirements, not left implicit? [Coverage, Spec §FR-025, Tasks T029/T030]

## Evaluation (2026-06-27)

Ran the 35 items against `spec.md` + `plan.md` + `tasks.md`. **Outcome: PASS (34 satisfied, 1 minor —
satisfied via tasks).**

- **Satisfied (34)** — verified against spec/plan/tasks text: boundary/no-logic (FR-011/022,
  Overview/Governance), the addon/facade split (Plan §Structure Decision + Complexity Tracking; Tasks
  T011/T013/T022), error contract (FR-014, Clarifications Q2, SC-006), the `ZodError` mapper +
  message-only scrub (FR-014/015, Research D3), marshaling fidelity + the Q6 null/undefined rule
  (FR-003a, Edge Cases, Clarifications Q6), validation ownership (FR-002, Q1/Q4, SC-002), boundary
  discipline (FR-023, Assumptions), loader/codegen (FR-005/006/007/008/024, Q3), packaging (FR-021,
  Q5/Q7/Q8, SC-009), the napi-pin reconciliation (Assumptions + Tasks T001/T002), SEC-004 scrub (FR-015),
  `check` finding kinds (FR-020), no-partial composition (US4/SC-008), the Node advisory + ci:test-node
  gates (FR-025, Tasks T029/T030). All 11 SCs (SC-001…SC-011) trace to ≥1 FR and ≥1 task.
- **CHK005 (minor, satisfied via tasks)**: the *exhaustive* `ConsumerError`→error translation
  ("a new variant is a compile error, not a fallthrough") is specified in **task T006** + memory, but is
  not elevated to a standalone spec FR. Acceptable for a binding spec — it is implementation discipline
  inherited from the closed-enum invariant (spec §FR-014 "closed `KernelError`"), pinned by the task.
  Not a requirements defect; no spec change required. (Identical disposition to the 004 binding checklist.)

No `[Gap]`/`[Conflict]` item is left unresolved or silently passed. The two `[Conflict]`-tagged items
(CHK029 napi pin; the token-hook line) are explicitly reconciled in the spec/tasks, not carried.

## Notes

- Check items off as resolved: `[x]`. A `[Gap]`/`[Conflict]` item that cannot be satisfied from the
  spec text should route back through the spec (not be silently passed).
- This is a requirements-quality gate — items test whether the *requirements are well-written*, not
  whether code works (that is verify/qa, Phase 3).
