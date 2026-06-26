# Sync Report — Spec 001 Foundations (Phase 3, Steps 15+16)

**Date:** 2026-06-26 · Run on main thread (both delegated sync subagents hit a tool-channel
glitch this session — one hallucinated a garbled dir listing claiming spec.md was 0 bytes;
verified FALSE, spec.md is 23,225 bytes intact in git and on disk).

## Step 15 — Drift (spec ↔ code): CONSISTENT (no actionable drift)

- **QA-era pipx→uv change is NOT drift.** Task T020 text explicitly allowed
  "`requirements.txt --hash=` **or** a committed `uv.lock`"; the impl chose uv.lock. Consistent.
- **Amended-task specifics all match code:** T013 explicit moon project map (no glob),
  T028 explicit FFI covered-crate list, T030a floating-version lint + `jq` pinned 1.8.1.
- **No CODE-AHEAD (unspecced scope):** every shipped artifact traces to an FR/task —
  ci/ + scripts/ci/ → T028/T030a; codegen scripts → T023–T026; negative-scope-checklist → T033;
  hermetic-CI install steps → security fix H-1 (within FR-018..020 CI scope).
- **No SPEC-AHEAD beyond by-design:** stub crates / no-runtime-logic are Principle III by design,
  not gaps.
- **One doc-freshness nit (→ fixed in Step 18):** `spec.md` header still says `Status: Draft`
  though it's implemented + CI-green.

## Step 16 — Inter-spec / governance conflicts: NONE

- Only spec 001 is written (002–007 are roadmap-planned, not specced) → no spec-vs-spec conflict.
- **001 vs constitution:** C-02 (FFI isolation) enforced live (gate green); Principle III
  (minimal boundary) intact — negative-scope checklist clean. No principle contradicted.
- **001 vs roadmap:** status `implemented` (v1.1.1); delivery matches the roadmap's stated
  001 outcome (crates + schema + codegen + CI gates). Consistent.
- **3 generated shapes mutually consistent** for the 006 conformance corpus: the 3 root types
  (PromptDefinition, VariableDecl, Variant) + fields + sealing semantics match across languages.
  The raw type COUNT differs (rust 11 / py 7 / ts 3) — a generation-style artifact (typify newtypes
  + untagged enum + Display/FromStr; Python enum classes; TS inlined unions), NOT a field mismatch.

### Forward-compat ADVISORY for 002+ (non-blocking; 002–007 not yet written)

- `variants.propertyNames` reserved-`default` rule (FR-011b) is enforced at the **validation
  layer**, not in the generated Rust type (typify strips `not`/`propertyNames`). Specs 002 (kernel)
  and 003 (consumer) consuming the generated Rust type must rely on schema-validation for that
  invariant — correct by design (no language's generated type can encode a forbidden map key).
  Carry into 002/003 planning so it isn't re-discovered. Already in docs/memory worklog.

**Overall: clean. No iterate/converge/bugfix needed.**
