# Sync / Drift Analysis — Spec 003 (Rust consumer)

**Date**: 2026-06-27 · **Step**: 15 (`sync.analyze`) · **Verdict**: ✅ CLEAN (no drift)

> **Provenance note.** The `speckit-sync` subagent was corrupted by the recurring bash-channel
> glitch this session and **fabricated** a fictional codebase in its report (it cited `src/vars.rs`,
> `tests/integration.rs`, `Vars::to_value`, `meta.fields`, a "byte-scanner", and a "CRITICAL: consumer
> reimplements render+hash locally" finding — **none of which exist**). That report is **discarded in
> full**. This analysis was performed main-thread against objective evidence (rg/cargo over the real
> tree), which is reliable. Every claim below is backed by a command result.

## Direction 1 — Stale spec (spec describes, code lacks): CLEAN

All four capabilities the spec/contract describe are implemented and delegate to the kernel:

| Spec capability | Code | Evidence |
|-----------------|------|----------|
| Validate-then-render (FR-001..003a, 009) | `render::render` | `render.rs:104` → `prompting_press_core::render` |
| Unrendered source (FR-010) | `render::get_source` | `render.rs:126` → `prompting_press_core::get_source` |
| Agreement + provenance lint (FR-016..020) | `check::check` | `check.rs:250` → `required_roots`; `check.rs:292` → `provenance_view` |
| Dual-input loader (FR-005..008) | `Registry::{load_yaml,load_json,insert}` | `registry.rs:90/114/43` |
| Composition (FR-012/013) | `Composition::{append,resolve}` | `compose.rs:144/197`, kernel render at `compose.rs:210` |

## Direction 2 — Unspecced code (code does, spec lacks): CLEAN

Every public symbol maps to a declared capability (`lib.rs:176-231`): `core` re-export, `PromptDefinition`/`RenderResult` re-exports (Principle VII), `error`/`registry`/`render`/`check`/`compose` modules, and `core_version()` (the load-bearing dependency edge). No orphan public surface. No SHA-256/hashing in the consumer (`rg sha2|Sha256` → none) — hashing stays in the kernel (C-01).

## Direction 3 — Contract mismatch: CLEAN

- `ConsumerError` variants (`error.rs:100-117`) and the closed `code` vocabulary (`error.rs:43-74`) match `contracts/consumer-api.md` §error.
- `FindingKind` is exactly the three specified variants — `UndeclaredVariable`, `UntrustedWithoutGuard`, `AnalysisError` — plus the CR-1 `ReservedVariantName` (`check.rs:127-158`); matches `data-model.md`.
- `render(reg, name, &vars, variant, &guard)` signature matches contract §render (`render.rs:72`).

## Mid-stream refinements — verified landed consistently in BOTH spec and code

- **F4 (token-count hook DROPPED): CLEAN both ways.** No token functionality in code — the only
  `rg token` hits are the SEC-004 secret-fixture literal (`error.rs:223`) and doc comments stating the
  hook was dropped (`lib.rs:55-64`). Spec side: FR-021/022, SC-009, T024 all struck-through with a
  "dropped (F4)" reason (`spec.md:333`, `tasks.md:145/161`). No leak in either direction.
- **F1 (provenance lint reframed): CLEAN.** `FindingKind::UntrustedWithoutGuard` present
  (`check.rs:136`); detection = `provenance_view` untrusted∪external minus a `guard` key in
  meta/metadata (`check.rs:291-326`); tested (`tests/check.rs`).
- **CR-1 (reserved `default` variant): CLEAN.** `FindingKind::ReservedVariantName` + dedup so the root
  body is analyzed once and the dead arm flagged not analyzed (`check.rs:227-240`, `340-350`); tested.
- **F5 (guard plumb-only): CLEAN.** `render` passes `guard` straight through (`render.rs:104`);
  composition passes a default `GuardConfig` (`compose.rs:214`). No guard-expansion logic in the
  consumer — kernel-owned.
- **F7 (empty inputs): CLEAN.** Empty composition → `Ok(vec![])` (`compose.rs:198/225`); empty
  registry `check` → empty `CheckReport` (pass) (`check.rs:204-213`). Both pinned by tests.
- **No `vars_hash`: CLEAN.** Consumer surfaces `RenderResult` 1:1; no `vars_hash` anywhere (Principle V).

## Overall verdict

✅ **CLEAN** — no drift in any direction. Spec, contract, and code agree, including all five analyze-gate
refinements. Proceed.
