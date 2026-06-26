# Requirements Quality Checklist: Engine kernel (`prompting-press-core`)

**Purpose**: Formal pre-implementation release gate — validate that the spec's requirements are
complete, clear, consistent, and measurable before the critique / security-review passes and
agent-assign implementation. Tests the *requirements*, not the code.
**Created**: 2026-06-26
**Feature**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

> Focus areas (user-selected): Soundness & boundary rigor · Determinism & provenance · Variant & error
> semantics · Security/guard semantics. Depth: formal release gate.

## Soundness & boundary rigor (agreement check + C-02/C-03)

- [x] CHK001 Are the v1 template features the kernel supports stated as an exhaustive, closed set (interpolation, conditionals, loops) rather than an open "such as" list? [Clarity, Spec §FR-001]
- [x] CHK002 Is the set of *excluded* template features enumerated completely and identically wherever it appears (FR-002, Edge Cases, Assumptions, SC-008)? [Consistency, Spec §FR-002]
- [x] CHK003 Does the requirement state *what observable outcome* an excluded feature produces (a clear error) rather than only that it is "excluded"? [Measurability, Spec §FR-002, §SC-008]
- [x] CHK004 Is the agreement analysis's exclusion set (loop locals, `{% set %}` targets, block locals) specified precisely enough to be objectively checkable? [Clarity, Spec §FR-017]
- [x] CHK005 Is the globals/filters allowlist requirement defined by *origin* (derived from the pinned engine version) rather than by a hard-coded list that could drift? [Clarity, Spec §FR-020]
- [x] CHK006 Is "the analysis MUST NOT mutate" stated as a verifiable property (inputs/output unchanged) rather than an aspiration? [Measurability, Spec §FR-018, §SC-006]
- [x] CHK007 Is the boundary prohibition (no I/O, LLM, request-body, token-count, output-parse) expressed as a testable negative requirement with a backing success criterion? [Coverage, Spec §FR-005, §SC-007]
- [x] CHK008 Is "validation-blind" defined unambiguously (kernel receives already-validated values; does no type/constraint checking)? [Clarity, Spec §FR-004]
- [x] CHK009 Is the requirement that the kernel *consumes* (not redefines) the 001 generated shape stated, and is the dependency-direction constraint (kernel ⊀ consumer) captured as a requirement rather than only in the plan? [Completeness, Spec §FR-027]
- [x] CHK010 Is the soundness rationale (why excluding includes/macros keeps the check sound) recorded so a future editor cannot reintroduce them without seeing the consequence? [Traceability, Spec §FR-002 / research D1–D2]

## Determinism & provenance (C-01 / C-05)

- [x] CHK011 Is "deterministic / byte-identical output" quantified as a verifiable property over repeated renders rather than described loosely? [Measurability, Spec §FR-003, §SC-001]
- [x] CHK012 Are `template_hash` and `render_hash` each defined unambiguously over a specific string input (variant source vs rendered output)? [Clarity, Spec §FR-012, §FR-013]
- [x] CHK013 Is the absence of `vars_hash` stated as an explicit negative requirement (not merely omitted)? [Completeness, Spec §FR-014]
- [x] CHK014 Are the fields of the render result fully enumerated, including the conditional guard field, so the output contract is complete? [Completeness, Spec §FR-015]
- [x] CHK015 Is the per-variant scope of `template_hash` consistent between the render requirement and the variant/entity descriptions? [Consistency, Spec §FR-012 / Key Entities] — **FIXED 2026-06-26**: "Resolved variant" entity used pre-refine "explicit default" wording; corrected.
- [x] CHK016 Is "provenance is data on the return value" stated as a requirement that forbids telemetry/tracing coupling, with measurable intent? [Clarity, Spec §FR-015]
- [x] CHK017 Does the spec distinguish structural cross-language parity (out of scope here, assumed by C-01) from the determinism the kernel itself must guarantee, without conflating them? [Consistency, Spec Assumptions / §SC-001]

## Variant & error semantics (post-FR-010 refine / FR-028)

- [x] CHK018 After the refine, is the variant-resolution rule internally consistent across FR-007, FR-008, FR-010, FR-011, US1 scenarios, and SC-004 — with no residual "missing-default error" language? [Consistency, Spec §FR-010, §SC-004] — **FIXED 2026-06-26**: FR-028 still listed "missing-default ambiguity" (deleted by the FR-010 refine); removed.
- [x] CHK019 Is the reserved name `default` requirement stated unambiguously (always maps to root body; enforced in kernel logic, not the generated type)? [Clarity, Spec §FR-011, Assumptions]
- [x] CHK020 Is "MUST NOT silently choose an arm" still meaningful after the refine (i.e., the default is the deterministic root body, never a silent pick among named arms)? [Clarity, Spec §FR-010]
- [x] CHK021 Is the error taxonomy (FR-028) complete relative to every failure the FRs imply — unknown-variant, excluded-feature, parse, strict-undefined, render? [Completeness, Spec §FR-028] — **FIXED 2026-06-26**: FR-028 omitted the strict-undefined (FR-001a) error class; added.
- [x] CHK022 Is each error condition traceable to the requirement that triggers it (e.g., strict-undefined ↔ FR-001a, unknown-variant ↔ FR-009)? [Traceability, Spec §FR-028]
- [x] CHK023 Is the strict-undefined requirement specified with its escape hatch (optional refs via an explicit defined-check), so "loud error" is not ambiguous about intentional optionality? [Clarity, Spec §FR-001a]
- [x] CHK024 Is the `ExcludedFeature` vs `Parse` distinction defined well enough to be testable, including the documented fallback when the engine's error kind is not distinguishable? [Measurability, Spec §FR-028 / research D4]
- [x] CHK025 Are the conservative-analysis limits (dynamic subscripts, use-before-set) documented as accepted false-negatives so they are not later mistaken for defects? [Assumption, Spec Edge Cases]

## Security / guard semantics (C-09)

- [x] CHK026 Is the guard expansion specified as opt-in *and* per-render, with the opt-out behavior (no guard field, body unchanged) stated explicitly? [Completeness, Spec §FR-022, §SC-005]
- [x] CHK027 Is "additive and non-mutating" defined concretely (separate result field; template/values/body unchanged; no value stripping)? [Clarity, Spec §FR-023, §FR-025]
- [x] CHK028 Is the guard field's relationship to the body unambiguous (a separate field, never concatenated), consistent across the Clarifications, FRs, Key Entities, and SC-005? [Consistency, Spec §FR-022 / Clarifications]
- [x] CHK029 Is the configurability requirement complete (a provided default plus full caller override) and is the default's existence required (even if its wording is deferred)? [Completeness, Spec §FR-024]
- [x] CHK030 Does the spec clearly state the security boundary — provenance tags are metadata + lint + opt-in guard, with NO runtime enforcement/sanitization — so callers do not over-trust it? [Clarity, Spec §FR-025 / C-09]
- [x] CHK031 Are the provenance-view contents (which fields are untrusted/external) specified as a derived, deterministic output? [Measurability, Spec §FR-021]

## Dependencies, assumptions & traceability

- [x] CHK032 Is the dependency on spec 001 (generated shape, crate layout, FFI/codegen gates) stated with what specifically is consumed? [Dependency, Spec Dependencies]
- [x] CHK033 Is the pinned-engine assumption explicit, with the version-confirmation obligation (roadmap Q3) recorded rather than assumed? [Assumption, Spec Assumptions / research D1]
- [x] CHK034 Is the kernel-boundary "values" shape acknowledged as a planning-resolved assumption rather than left undefined in the requirements? [Assumption, Spec Assumptions / research D5]
- [x] CHK035 Does every Success Criterion (SC-001…SC-009) trace to at least one functional requirement, and does every FR group have a covering SC? [Traceability, Spec §SC / §FR]

## Notes

- Check items off as the requirements are confirmed/repaired: `[x]`.
- This list tests requirement *quality*; behavioral verification lives in tasks.md tests + quickstart.md.
- Items flagging a real gap/ambiguity/conflict should route back through `/speckit.refine.update` (pre-checkpoint) before implementation.
