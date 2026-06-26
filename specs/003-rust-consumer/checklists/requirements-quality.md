# Requirements Quality Checklist: Rust consumer crate (`prompting-press`)

**Purpose**: Formal pre-implementation release gate — validate that the spec's requirements are
complete, clear, consistent, and measurable before critique / security-review and agent-assign
implementation. Tests the *requirements*, not the code.
**Created**: 2026-06-26
**Feature**: [spec.md](../spec.md) · [plan.md](../plan.md) · [tasks.md](../tasks.md)

> Focus: no-logic-duplication / boundary (C-01/C-02/C-03) · error-normalization (C-06) · lint
> correctness (C-04/C-09) · dual-input loader (C-07). Depth: formal release gate.

## No-duplication & boundary (C-01/C-02/C-03)

- [x] CHK001 Is "the consumer wraps the kernel and duplicates no render/agreement/variant/hashing logic" stated as a normative requirement, not just prose? [Clarity, Spec §FR-011]
- [x] CHK002 Is the FFI-free constraint expressed with a verifiable backing criterion (the gate + zero pyo3/napi)? [Measurability, Spec §FR-023, §SC-007]
- [x] CHK003 Are the boundary prohibitions (no I/O, no LLM, no request-body, no output parsing; output_model metadata-only) stated as testable negatives? [Coverage, Spec §FR-024]
- [x] CHK004 Is it clear the crate reads no files itself (caller pushes text/objects in)? [Clarity, Spec §FR-024 / Edge Cases]
- [x] CHK005 Does the spec specify the dependency direction (consumer → kernel, never the reverse)? [Consistency, Spec Governance / Dependencies]

## Typed Vars + validation (C-06)

- [x] CHK006 Is "validation runs once, at render, before templating" specified unambiguously? [Clarity, Spec §FR-002]
- [x] CHK007 Is the no-render-on-invalid-input rule stated and measurable? [Measurability, Spec §FR-002, §SC-002]
- [x] CHK008 Is the requirement that native validator outputs never appear on the public API explicit? [Completeness, Spec §FR-004, §SC-006]
- [x] CHK009 Is the validated-struct → kernel-value bridge (serialize) specified, so the caller doesn't hand-build a map? [Clarity, Spec §FR-003a]
- [x] CHK010 Is the render input contract (caller passes prompt + typed vars together; no per-prompt type registration) unambiguous? [Clarity, Spec §FR-009 / Clarifications]

## Error normalization (C-06)

- [x] CHK011 Is the common error shape (`{field, code, message}`) defined and is it the sole public error surface? [Completeness, Spec §FR-014]
- [x] CHK012 Is the mapping source for BOTH validation failures AND kernel errors specified (both normalize)? [Coverage, Spec §FR-014]
- [x] CHK013 Is the info-leakage prohibition (don't echo raw bound-value content into messages/logs) stated as a requirement? [Clarity, Spec §FR-015 / Edge Cases]
- [x] CHK014 Is "native error types MUST NOT leak" consistent across spec, the SC, and the contract? [Consistency, Spec §FR-014, §SC-006]

## Dual-input loader (C-07)

- [x] CHK015 Are all three input forms (YAML, JSON, constructed object) specified as first-class with identical downstream behavior? [Completeness, Spec §FR-005]
- [x] CHK016 Is the YAML/JSON parity requirement measurable (identical internal representation)? [Measurability, Spec §FR-006, §SC-003]
- [x] CHK017 Is malformed-input behavior specified (structured error, nothing partially loaded)? [Edge Case, Spec §FR-007]
- [x] CHK018 Is "consume the kernel's shape, define no parallel shape" stated? [Consistency, Spec §FR-008]
- [x] CHK019 Is the registry's role (load target + render-resolution + lint scope; absent-name behavior) specified? [Completeness, Spec §FR-008a]

## Agreement + provenance lint (C-04/C-09)

- [x] CHK020 Is the authoritative "declared variables" set unambiguously fixed (the definition's `variables` block, not the garde struct)? [Clarity, Spec §FR-017 / Clarifications]
- [x] CHK021 Is "the consumer owns the ⊆ comparison; referenced roots come from the kernel" stated (no re-derivation)? [Consistency, Spec §FR-017]
- [x] CHK022 Is the provenance lint rule (untrusted/external outside a declared guard position → finding) specified? [Completeness, Spec §FR-018]
- [x] CHK023 Is the check's purity (no render, no mutation, no side effects) a stated, verifiable requirement? [Measurability, Spec §FR-019]
- [x] CHK024 Is each finding required to be actionable (names prompt + variant + offending variable/field)? [Clarity, Spec §FR-020]
- [x] CHK025 Is per-variant analysis specified (each variant's template checked)? [Coverage, Spec §FR-016 / US3 sc.5]

## Composition + token hook (C-06/C-03)

- [x] CHK026 Is composition specified as an explicit ordered sequence resolving to ordered `{role, text}` messages? [Completeness, Spec §FR-012]
- [x] CHK027 Is the `.chain()` exclusion stated as a negative requirement? [Clarity, Spec §FR-013]
- [x] CHK028 Is the partial-failure behavior in composition specified (one entry fails → error, not partial-as-success)? [Edge Case, Spec US4 sc.3]
- [x] CHK029 Is the token-count hook specified as the ONLY mechanism, with hook-absent = no-count-not-error, and no built-in counter? [Completeness, Spec §FR-021, §FR-022, §SC-009]

## Dependencies, assumptions & traceability

- [x] CHK030 Is the dependency on spec 002 (the wrapped kernel API + result/error types + re-exported shape) stated with specifics? [Dependency, Spec Dependencies]
- [x] CHK031 Is the garde-as-the-validation-system assumption explicit, with the version-confirmation obligation recorded? [Assumption, Spec Assumptions / research D1]
- [x] CHK032 Is the YAML-parser choice acknowledged as a planning-resolved assumption (serde_yaml archived) rather than left undefined? [Assumption, research D2]
- [x] CHK033 Does every Success Criterion (SC-001…SC-009) trace to ≥1 functional requirement, and each FR group to a covering SC? [Traceability, Spec §SC / §FR]
- [x] CHK034 Are the deferred plan-level details (count_tokens signature, normalized-error Rust type, exact dep patch) flagged rather than silently omitted? [Completeness, Spec Assumptions / memory.md]

## Notes

- Check items off as requirements are confirmed/repaired. Behavioral verification lives in tasks.md
  tests + quickstart.md.
- All 34 items evaluated PASS against the current spec (post-clarify). The spec is internally
  consistent: the four clarify decisions (declared-vars authority, registry, render(prompt,vars),
  serialize bridge) are reflected uniformly across Clarifications, FRs, Key Entities, and Assumptions.
- One framing note (same as 002): "non-technical stakeholder" = the library *consumer* (an app
  developer); `garde`/`KernelError`/`Value::from_serialize` appear in FRs only as constitution-/
  kernel-fixed contract terms (C-06 names garde as the Rust idiom), not as choices this spec invents.
