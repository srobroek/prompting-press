# Sync Conflicts (inter-spec) — Spec 005 (TypeScript binding)

**Date**: 2026-06-28 · **Step**: 16 (`/speckit.sync.conflicts`)
**Scope**: spec 005 vs. specs 002/003/**004** + the constitution + the JSON Schema SSoT, with focus on
the cross-spec ripple of the **C-11 amendment** (constitution v1.1.0) applied this cycle.

> **Provenance**: run MAIN-THREAD (the `speckit-sync-conflicts` subagent fabricated — tool_uses:0 — in
> the 004 cycle). Every conflict cites the two contradicting locations.

## Verdict: ONE real cross-spec conflict (the C-11 change made 004's code keyword-only but left 004's quickstart positional) → fix the stale 004 doc. No other conflicts.

### Conflict (cross-spec doc drift from the C-11 amendment)

- **X-1** — The C-11 amendment was applied to the **merged spec-004 Python binding's code** this cycle
  (`895a592`: `render`/`get_source`/`Composition.append`/`GuardConfig` made keyword-only via PyO3
  `signature` `*,`). But **`specs/004-python-binding/quickstart.md:45,52,67`** still call
  `render(reg, "greet", Greeting, {"name":"Ada","count":3})` — passing `data` **positionally**. Against
  the shipped (post-`895a592`) code that now raises `TypeError`. The 004 spec body (`render(name, …)`)
  and contract (`vars_model, data` shape) are high-level and NOT contradicted; only the quickstart
  examples are concretely stale. **Fix: update the 004 quickstart `render(...)` calls to `data=` kwarg
  (and add a one-line C-11 note).** Done this cycle (the change that broke them was made this cycle).

### Cross-binding consistency — VERIFIED conformant (no conflict)
The three bindings now agree on the C-11 call shape, each in its native idiom:
- **Rust consumer** (`render.rs:72-76`): `render<V>(reg, name, vars, variant: Option<&str>, guard:
  &GuardConfig)` — single `Option` positional, below the Rust C-11 threshold (conformant by decision).
- **Python** (`render.rs:198`): `(reg, name, vars, *, data=None, variant=None, guard=None)` — keyword-only.
- **TS** (`index.ts:423`): `render(reg, name, schema, data, opts?: RenderOptions)` — options object.

### No contradiction with prior decisions / shared contracts
- **C-11 vs Principle VI's existing rules**: C-11 *extends* Principle VI (the `.chain()` ban + "explicit
  ordered array" composition still hold — the 005 `Composition` is still an explicit ordered array, now
  of objects). No contradiction; C-01..C-10 untouched.
- **Kernel (002) + consumer (003) API** that 005 binds: unchanged by this cycle; all signatures 005
  assumes still exist. The C-11 change touched only the *binding facades*, never the kernel/consumer.
- **JSON Schema SSoT**: unchanged; 005's codegen'd shape is consistent.
- **004 ↔ 005**: separate packages/crates, shared T0NN namespace handled by the `[004]`/`[005]` issue
  prefixes; no contradictory scope. Both now C-11-conformant.

## Disposition
X-1 is a stale-doc conflict caused by this cycle's cross-spec C-11 application — the 004 quickstart is
updated to match its now-keyword-only code (recorded here; the breaking change and its fix are both this
cycle). No code conflict, no contradiction between live decisions.
