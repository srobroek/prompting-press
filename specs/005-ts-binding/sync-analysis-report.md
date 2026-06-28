# Sync Analysis (drift) — Spec 005 (TypeScript binding)

**Date**: 2026-06-28 · **Step**: 15 (`/speckit.sync.analyze`)
**Scope**: spec.md / plan.md / tasks.md / contracts/ts-api.md vs. the implemented code (`git diff main..HEAD`).

> **Provenance**: run MAIN-THREAD (the `speckit-sync` subagent fabricated — tool_uses:0 — in both the
> 003 and 004 cycles; for a check I have full context on, main-thread is faster and glitch-immune).
> Every drift item below cites the actual file:line.

## Verdict: code↔spec drift found (code is RIGHT, spec text lagged the in-cycle C-11 amendment) — 3 stale-doc items to reconcile.

The drift is the expected kind: the C-11 options-object/variant refactor landed *during* this spec's
review cycle (after spec.md/contracts were authored), so the code gained a capability + a call shape the
spec text doesn't describe. FR-009/FR-012 are high-level enough not to be *contradicted*, but the
contract examples + FR-012's "ordered array of (prompt, vars, variant)" wording are now stale.

### Drift items (spec stale, code correct → update the spec)

- **D-A** — `contracts/ts-api.md:32,39` show `render(reg, "greet", Vars, { name: "Ada", count: 3 })` and
  no variant-selection example; the shipped API is `render(reg, name, schema, data, opts?)` with
  `opts.variant`/`opts.guard` (C-11). The examples still work for the no-opts case but don't show the
  options object or variant selection. **Fix: update the contract examples.**
- **D-B** — `contracts/ts-api.md:65` + **spec FR-012** (`spec.md:289-290`) describe composition entries
  as a positional "ordered array of (prompt, vars, variant)" tuple; the shipped form is an object array
  `{ name, schema?, data, variant? }` (C-11). **Fix: update FR-012 + the contract example.**
- **D-C** — neither spec.md nor the Clarifications record the **C-11 options-object decision** or
  **variant-selectable `render`** (FR-009 implies variant is caller-owned but the spec earlier leaned
  "variant rides through getSource/Composition" — reversed by C-11). **Fix: add a Clarifications note +
  tighten FR-009/FR-012 to the options-object shape, citing C-11 / constitution v1.1.0.**

### No drift in these directions
- **Spec→code (unbuilt scope)**: none. All 25 FR / 11 SC implemented (verify PASS; qa 12/12; tasks 31/31).
- **Code→spec (unspecced behavior)**: the variant-selectable `render` is *new capability*, but it's
  mandated by FR-009 ("variant… caller-owned") + Principle V + C-11 — so it's specced-in-principle,
  just not reflected in the call-shape prose. Captured as D-C.
- **Phantom completions**: none (verify-tasks 31/31).

## Disposition
The 3 items are **doc-staleness from an in-cycle amendment**, not a code defect — the SpecKit-correct
fix is to update the spec/contract to match the shipped (C-11-conformant) API. Since 005 is pre-merge
and the change is small + unambiguously correct, the spec text is updated in this cycle (a lightweight
`iterate`-equivalent: align the stale prose to the decided + implemented reality), recorded here.
