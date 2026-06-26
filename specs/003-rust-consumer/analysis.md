# Specification Analysis Report — Spec 003 (Rust consumer)

**Date**: 2026-06-26 · **Type**: cross-artifact consistency + risk analysis (read-only gate, step 6)
· **Artifacts**: spec.md, plan.md, tasks.md (+ data-model.md, contracts/consumer-api.md, quickstart.md)
vs constitution v1.0.0 and the merged spec-002 kernel surface.

## Verdict

**0 CRITICAL, 0 constitution violations. 100% requirement→task coverage (26/26 FR, 9/9 SC mapped).**
But **2 HIGH findings (F1/F2)** make the provenance lint (FR-018 / SC-005 — half the headline
differentiator) untestable as written; flagged to the user before implementation. Token-hook wiring
(F4) and three minor items also recorded.

## Findings

| ID | Category | Severity | Location | Summary | Recommendation |
|----|----------|----------|----------|---------|----------------|
| F1 | Underspecification | **HIGH** | FR-018, T019, contracts §3 | The provenance lint's "untrusted/external field used **outside a declared guard position**" has no operational definition. Kernel `provenance_view` returns only field-name sets; `GuardConfig` is a render-time opt-in *instruction*, not a template region. The kernel has **no "guard position" concept** (verified in `provenance.rs`). | Redefine the provenance lint against the kernel's real surface, OR defer the "guard position" enforcement to a later spec. **User decision (below).** |
| F2 | Ambiguity | **HIGH** | T019, V3.3, FR-018 | Follows F1: V3.3's seeded `UntrustedOutsideGuard` test case can't be authored deterministically without a "guard position" predicate. | Resolve F1, then write the V3.3 fixture + predicate. |
| F3 | Inconsistency | MEDIUM | FR-009 ("prompt handle") vs contract/plan/tasks (registry-name only) | FR-009 offers a "prompt handle" alternative to registry-name resolution; no artifact defines a handle. Orphaned concept. | **Resolve with default**: drop the "handle" alternative — registry-name is the sole resolution path. |
| F4 | Coverage gap | MEDIUM | FR-022 / SC-009, T024 | "counts come from the hook" is ambiguous: does the crate INVOKE `count_tokens` and return a count (no result field exists; `RenderResult` is kernel-owned), or merely EXPOSE the hook for the caller? T024 only builds/tests the seam in isolation. | **User decision (below).** |
| F5 | Underspecification | MEDIUM | FR-009 "optional guard field" | No consumer test exercises `guard.enabled == true`; consumer always passes `GuardConfig::default()`. | **Resolve with default**: guard-EXPANSION behavior is owned/tested by spec-002 (kernel); 003 surfaces the field. State out-of-scope in spec; consumer test asserts the field is plumbed through (passed in → present in result) without re-testing kernel guard logic. |
| F6 | Inconsistency | LOW | data-model "BTreeMap→deterministic" vs `def.variables: HashMap` | Registry iteration is BTreeMap-deterministic; the per-prompt declared set is `HashMap` keys. Determinism still holds (driven by the kernel's `BTreeSet` `required_roots`). | No change; optional clarifying note. |
| F7 | Ambiguity | LOW | Edge Cases "benign empty result or a clear error" | Empty registry/composition behavior left as either/or. | **Resolve with default**: empty composition → `Ok(vec![])`; `check()` over empty registry → empty `CheckReport` (pass). Pin in tasks. |
| F8 | Coverage gap | LOW | SC-007 negative cargo-tree | Lives in CI gate steps, not a unit test — matches constitution intent (CI gate). | None; `ci:check-ffi` satisfies SC-007. |

## Coverage

All 26 FRs and 9 SCs map to ≥1 task. Weak/partial: **FR-018/SC-005** (F1/F2 — untestable until the
provenance-lint semantics are defined), **FR-022/SC-009** (F4 — hook wiring/return path unclear). No
unmapped tasks, no duplication, no constitution conflict (the token hook is the only seam — C-03/C-08).

## Resolution — APPLIED 2026-06-26 (via refine.update)

- **F1** (user decision): REFRAMED — provenance lint = "prompt declares untrusted/external + no guard
  configured" (`UntrustedWithoutGuard`). FR-018/SC-005/V3.3/T015/T019 + data-model/contract updated.
- **F4** (user decision): the token-count hook is **DROPPED entirely** from spec 003 (deferred to a
  later spec). FR-021/022, SC-009, the `tokens.rs` module, task T024, the Key Entity, and quickstart
  V5.1/V5.2 removed. Net: 8 SCs (was 9), 26 tasks (was 27). Roadmap "deferred" note to be added.
- **F3**: render resolves by registry name only (FR-009 "prompt handle" dropped).
- **F5**: guard-expansion is kernel-owned; the consumer only plumbs `GuardConfig` through + surfaces
  the `guard` field (FR-009 note + a T007 plumb-through assertion).
- **F7**: empty composition → `Ok(vec![])`; empty-registry `check()` → empty `CheckReport` (Edge Cases
  + T017/T021 pinned).
- **F6, F8**: no action (non-issues).

All applied across spec/plan/tasks/data-model/contract/quickstart. Historical records (this report,
the critique, the original requirements-quality checklist, security report, research deferred-notes)
are left intact as the audit trail. Ready for taskstoissues → checkpoint → agent-assign.
