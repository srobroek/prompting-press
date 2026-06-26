# Clarifications — Spec 003 (Rust consumer)

## Session 2026-06-26

4 questions asked and answered; all integrated into `spec.md` (`## Clarifications` + FRs + Key
Entities + Assumptions) and `memory.md`. The four cohere into a deliberately *simpler* design (no
generic type-registration, no Rust-type introspection in the lint).

### Q1 — Authoritative "declared variables" for the agreement check
**Answer**: The prompt **definition's `variables` block** (the spec-001 shape carried by the kernel).
`check()` compares template-referenced roots against `definition.variables` — pure data, CI-portable,
no need to introspect the caller's garde struct. The garde struct is the runtime validator, not the
lint's authority.
**Impact**: FR-017 sharpened; Key Entities (prompt definition) + Assumptions updated.

### Q2 — Registry shape
**Answer**: A library-owned **map of prompt name → `PromptDefinition`**. The app loads prompts into it;
`render(name, …)` resolves against it; `check(registry)` lints over it. Absent name → structured error.
**Impact**: New FR-008a; Key Entities (Registry) + Assumptions updated.

### Q3 — How typed Vars connect to a prompt at render
**Answer**: **The caller passes both at render** — `render(prompt, vars)`. No per-prompt Vars-type
registration; Vars↔prompt correctness is the caller's responsibility (and a `check()`-time concern).
**Impact**: FR-009 sharpened; Assumptions updated.

### Q4 — Vars → kernel value bridge
**Answer**: **Serialize the validated struct.** The Vars struct is serializable; after garde
validation passes, the crate serializes it into the kernel's value type. One typed struct in →
validated → serialized (standard serde+garde pairing). Caller does not hand-build a value map.
**Impact**: New FR-003a; Key Entities (Typed Vars model) + Assumptions updated.

## Deferred to plan (not blocking)

- Exact garde current version + `Validate`/`#[garde(custom(...))]`/`Report` API (verify-at-spec-time).
- The YAML-parser crate for the dual-input loader (maintained + pure-Rust; e.g. serde_yaml_ng /
  serde_norway — verify at plan time).
- The `count_tokens` hook's exact Rust signature (trait vs boxed closure).
- The normalized-error Rust type (`Vec<{field, code, message}>`) and the garde-`Report`→field mapping.
