# Specification Quality Checklist: Guard delimiting redesign

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-06-29
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [ ] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous (for resolved FRs)
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [ ] All functional requirements have clear acceptance criteria — blocked by [NEEDS CLARIFICATION] items
- [x] User scenarios cover primary flows
- [ ] Feature meets measurable outcomes defined in Success Criteria — blocked by open Q1/Q2/Q3
- [ ] No implementation details leak into specification — FR-D14/FR-D15 name the pre-pass layer; acceptable per the "load-bearing constitution identity" precedent (these are governance-level constraints, not free design choices)

## Notes

- **SPEC IS BLOCKED — DO NOT PROCEED TO PLAN** until all three open questions are resolved in a clarify session AND the required constitution amendment is ratified.

### Open questions requiring clarify (all blocking)

- **[Q1] Delimiter scheme** — fixed XML-ish tags + entity-escaping vs. random-nonce tags. Determines FR-D02, FR-D07, and whether SC-001 (determinism) is preserved. Lean: fixed tags with entity-escaping (Anthropic/OpenAI recommended pattern; preserves determinism).
- **[Q2] Activation mode** — always-on when guard enabled, vs. new `advisory | delimited` sub-mode. Determines FR-D10, the `GuardConfig` shape change, and backward-compatibility scope.
- **[Q3] Implementation layer** — kernel template pre-pass (recommended) vs. other. Must confirm: (a) which interpolation forms are in v1 scope (`{{ var }}` only vs. also `{{ var | filter }}` and `{{ user.name }}`); (b) that the pre-pass can be implemented without `unstable_machinery` (FR-D15); (c) whether the kernel's value-blindness is maintained.

### Required constitution amendment (non-negotiable gate)

- **REQUIRED BEFORE PLAN**: a constitution MAJOR amendment must be authored and ratified amending:
  - **FR-022** (spec 002) — remove the "body MUST be identical to plain render" clause for the guard-on case
  - **FR-023** (spec 002) — narrow "MUST NOT modify rendered body content" to "value semantic content"; permit structural marker insertion
  - **SC-005** (spec 002) — split the invariant: preserve the guard-off half; replace the guard-on half with the new SC-D01/SC-D04 invariants
  - **Constitution Principle III** — refine the "additive and non-mutating" body doctrine for the guard-on case
  - **Constitution Principle V** — note that `render_hash` now depends on guard mode (guard-on hashes the delimited body)
  - **Roadmap decision C-09** — amend "additive guard expansion … body never mutated" to reflect delimiter insertion, while preserving the "value content never silently mutated" constraint
- **Amendment classification**: MAJOR (Principle III body-invariant redefined in a backward-incompatible way)
- **DECISIONS.md entry required**: rationale + migration note for callers who relied on body byte-identity with guard on

### Non-blocking notes

- FR-D14/FR-D15 (pre-pass in kernel, no `unstable_machinery`) are governance-level constraints from Principle I and Principle IV — they appear in the FRs because they are constitutionally load-bearing, following the same precedent as `SHA256`/`template_hash`/`render_hash` in spec 002.
- FR-025 (no value mutation) from spec 002 is explicitly preserved — the spec correctly distinguishes structural marker insertion from semantic value mutation.
- SC-001 (determinism) is implicitly at risk if Q1 resolves to random-nonce tags; this dependency is called out in the Assumptions and the Q1 clarify question.
