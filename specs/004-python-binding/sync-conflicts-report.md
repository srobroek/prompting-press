# Sync Conflicts (inter-spec) — Spec 004 (Python binding)

**Date**: 2026-06-27 · **Step**: 16 (`/speckit.sync.conflicts`)
**Scope**: spec 004 vs. specs 001/002/003/005, the JSON Schema SSoT, the kernel/consumer API, and the
roadmap ledger (decisions C-01..C-10).

> **Provenance**: the `speckit-sync-conflicts` subagent hit the systemic tool-channel glitch
> (`tool_uses: 0` — it narrated "report written: 68 lines" but executed no tools and wrote no file;
> every citation it produced was therefore unbacked). Its output was discarded. This analysis was
> performed main-thread against real `rg`/file evidence.

## Verdict: no conflicts.

### The flagged item — token hook — RESOLVED (T027 reconciliation succeeded)

The historically-stale "token hook" in the roadmap 004/005 `Scope (in)` lines is **gone** (struck by
T027), so there is no spec↔decision contradiction:
- `.specify/memory/roadmap.md:236` (004 entry): "`~~token hook~~` (struck — the token surface was
  dropped in spec 003, refinement F4 … never a binding concern)".
- `.specify/memory/roadmap.md:254-255` (005 entry): "`~~token hook~~` (struck — same F4 reason as 004)".
- Remaining "token" mentions are all legitimate: the Deferred "Token budgeting / truncation" entry
  (`:313-316`), the Principle III boundary description (`:89`, `:104`, `:361`), and the 003-history
  note (`:6-21`). None is a live 004/005 scope claim.
- Spec 004 itself (`spec.md` Scope-out) and spec 003 (F4) agree the surface was dropped. Consistent.

### Shared-contract checks — all consistent (both sides exist as assumed)

- **Kernel API (002) that 004 binds exists as specified:** `prompting_press_core::render`
  (`engine.rs:154`), `get_source` (`engine.rs:103`), `required_roots` (`agreement.rs:88`),
  `provenance_view` (`provenance.rs:92`). No phantom-API assumption.
- **Consumer API (003) that 004 binds exists:** `prompting_press::check` (`check.rs:204`), `get_source`
  (`render.rs:117`), `Registry` (`registry.rs:27`), `ConsumerError` (`error.rs:100`), `FindingKind`
  (`check.rs:127`). The hallucinated "consumer reimplements render+hash" from the prior corrupted run
  is FALSE — 003 delegates to the kernel; no such claim exists.
- **004 vs 005 (TS binding):** mutually out-of-scope, separate FFI crates (`pyo3`/`prompting-press-py`
  vs `napi`/`prompting-press-node`), same shared contracts — no contradictory ownership.
- **JSON Schema SSoT** (`schemas/jsonschema/prompt-definition.schema.json`): no `version`/`versions`/
  `vars_hash`; consistent with Principle V/VII and 004's codegen'd shape.
- **Constitution / roadmap C-NN:** 004's decisions (C-02 FFI isolation, C-06 idiom, C-07 codegen)
  are upheld by the implementation; no decision is contradicted.

## Conclusion
No blocking conflicts, no warnings. Spec 004's assumed APIs and shared contracts all exist and agree
across specs.
