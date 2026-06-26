# Clarifications — Spec 002 (Engine kernel)

## Session 2026-06-26

3 questions asked and answered. All integrated into `spec.md` (`## Clarifications` + dependent
sections) and `memory.md`.

### Q1 — Undefined-variable render behavior
**Question**: At render time, how should the kernel treat a variable referenced in the template but
absent from the supplied values?
**Answer**: **Strict — render errors.** Undefined variable use causes a loud render error (a
defense-in-depth backstop to the static agreement check); intentionally-optional references must use
an explicit defined-check (`is defined`).
**Impact**: New FR-001a; US1 scenario 8; new edge case; SC-009; Assumptions updated.

### Q2 — Agreement-analysis granularity
**Question**: What granularity should the agreement analysis expose for a prompt's required root
variables?
**Answer**: **Per resolved variant** — the kernel reports the required-root set for one
template/variant source at a time; aggregating across variants is the consumer's concern.
**Impact**: FR-016, FR-019 sharpened; Key Entities (required-roots set) updated; Assumptions updated.

### Q3 — Guard-instruction placement
**Question**: Where should the opt-in guard instruction be placed relative to the rendered body?
**Answer**: **A separate field on the render result, caller-configurable with a provided default.**
The guard text is returned as a distinct field, never concatenated into the rendered body; placement
into a final prompt is the caller's decision.
**Impact**: FR-022..FR-024 reworked; FR-015 (result fields) updated; US3 scenarios 2–4; new "Guard
field" Key Entity; SC-005 updated; Assumptions updated.

## Deferred to plan (not blocking)

- Exact pinned MiniJinja version + globals/filters allowlist contents (roadmap Q3; allowlist is a
  fixed kernel constant per C-08, not a new seam).
- The strict-undefined mechanism (MiniJinja `UndefinedBehavior::Strict`) and the excluded-feature
  rejection mechanism — confirm against the pinned version.
- The "values" wire type at the kernel boundary.
- The default guard-template wording.
