# Verify-Tasks Report — Spec 004 (Python binding `prompting-press-py`)

**Date**: 2026-06-27
**Scope**: `all` (`git diff main..HEAD` + uncommitted; working tree clean)
**Tasks audited**: 28 completed (`[X]`) tasks, T001–T028
**Base ref**: `main` (branch `004-python-binding`, 10 commits ahead)

> ⚠️ **FRESH SESSION ADVISORY**: This audit was run after a session crash, in a recovery
> session separate from the implementing session. Phantom-completion detection applies.

## Provenance note (READ THIS)

A fresh-context `speckit-verify-tasks` subagent was spawned first (per the workflow), but it hit
the **known-systemic tool-channel fabrication glitch**: it reported `tasks.md` and `qa-report.md` as
`0 bytes` and listed files that do not exist (`.icicles`, `contracts.md`, `checklist-requirements.md`).
Direct main-thread inspection proved those claims false (`tasks.md` = 203 lines, `qa-report.md` = 37
lines; the fabricated files do not exist; `git status` clean). **The subagent verdict was discarded
in full.** This report is written from main-thread evidence only (`rg`/`git`/`cargo test`/`pytest`),
per the standing rule that every audit-subagent finding must be re-verified against objective evidence.

## Summary scorecard

| Verdict | Count |
|---------|-------|
| ✅ VERIFIED | 28 |
| 🔍 PARTIAL | 0 |
| ⚠️ WEAK | 0 |
| ❌ NOT_FOUND | 0 |
| ⏭️ SKIPPED | 0 |

**No phantom completions. No flagged items.**

## Load-bearing evidence (main-thread, reproducible)

**Layer 1–2 (existence + diff):** all referenced files exist and appear in `git diff main..HEAD`
(7 binding sources, 4 pytest suites, facade, CI wiring, advisory script, roadmap reconcile — 40 files,
+6699 lines).

**Layer 3 (symbols present):** `render`/`get_source` (render.rs), `check` + `CheckReport`/`Finding`
(check.rs), `Composition.from_messages`/`append`/`resolve` + `Message` (compose.rs), exception
hierarchy via `create_exception!` (error.rs), `Registry.insert`/`load_yaml`/`load_json` (registry.rs),
`to_kernel_value` (marshal.rs), `#[pymodule]` (lib.rs) — all confirmed.

**Layer 4 equivalent (FFI wiring):** Rust symbols are exported across the `#[pymodule]` boundary; the
Python facade `__init__.py` re-exports all 13 public names + `PromptDefinition` from `.generated`, and
`__all__` lists them. Confirmed at runtime: `import prompting_press` exposes
`{Registry, RenderResult, render, get_source, check, Composition, Message, CheckReport, Finding,
FieldError, GuardConfig, PromptDefinition, PromptingPressError, PromptValidationError, PromptRenderError,
UnknownPromptError, LoadError, core_version}`. A Rust-side "never called" is expected for a binding and
is NOT dead code.

**Layer 5 (semantic — the constitutional check, roadmap C-02 / Principle I):** the binding contains
**zero engine logic**. Verified by delegation evidence + absence of reimplementation:
- Delegates: `render.rs:237` → `prompting_press_core::render`; `render.rs:260` →
  `prompting_press::get_source`; `check.rs:179` → `prompting_press::check`; `compose.rs:239` →
  `prompting_press_core::render`; `registry.rs` wraps `prompting_press::Registry` + routes loaders to
  the consumer.
- Absent (all `rg` searches empty): `minijinja::Environment`/template parsing, `Sha256`/hashing,
  agreement/`undeclared_variables` analysis. `marshal.rs` only bridges Python → `minijinja::Value`
  via pythonize (the value bridge, not a rendering engine).

**Cross-cutting guarantees:**
- **F4 (no token surface):** `rg "count_tokens|token_count|TokenCount|count-tokens"` over
  `packages/python/python` + `crates/prompting-press-py/src` → nothing. ✅
- **SEC-004 scrub:** error.rs routes `KernelError` through the consumer's tested
  `From<KernelError> for ConsumerError` scrubber first (discards `parse`/`render`/`excluded_feature`
  detail), surfaces only fixed messages; documented + unit-tested (Rust `error.rs` test +
  `test_render.py::test_rejected_sensitive_input_is_not_leaked`). ✅

**Behavioral (tests actually pass — not self-report):**
- `cargo test -p prompting-press-py` → **30 passed** (main-thread, this session).
- `pytest packages/python/tests` → **50 passed** (ephemeral uv venv, against the built
  `cp310-abi3` wheel, this session). Test bodies are substantial (44 Python test fns / 146 asserts;
  no stubs/`pass`/`TODO`/`NotImplementedError`).

## Verified items

| Task | Verdict | Summary |
|------|---------|---------|
| T001 | ✅ VERIFIED | `abi3-py310` + pythonize 0.29 pinned in Cargo.toml (diff confirmed) |
| T002 | ✅ VERIFIED | pydantic runtime dep + uv.lock refreshed (pyproject + uv.lock in diff) |
| T003 | ✅ VERIFIED | baseline build/FFI/codegen gates (ci:check-ffi/codegen-check pass, T025) |
| T004 | ✅ VERIFIED | `#[pymodule]` wiring in lib.rs registers all classes/fns/exceptions |
| T005 | ✅ VERIFIED | marshal.rs `to_kernel_value` via pythonize → `minijinja::Value` |
| T006 | ✅ VERIFIED | exception hierarchy (create_exception!) + SEC-004 scrub in error.rs |
| T007 | ✅ VERIFIED | `Registry` pyclass wrapping `prompting_press::Registry` |
| T008 | ✅ VERIFIED | Rust scrub/marshal unit tests — 30 cargo tests pass |
| T009 | ✅ VERIFIED | Python render tests (test_render.py, 9 fns) pass |
| T010 | ✅ VERIFIED | `render` delegates to `prompting_press_core::render` (no engine logic) |
| T011 | ✅ VERIFIED | `get_source` delegates to consumer; `RenderResult` pyclass |
| T012 | ✅ VERIFIED | build+test gate (cargo+pytest pass) |
| T013 | ✅ VERIFIED | loader tests (test_loader.py, 10 fns) incl. parity + Norway-safe |
| T014 | ✅ VERIFIED | `load_yaml`/`load_json`/`insert` route to consumer loader |
| T015 | ✅ VERIFIED | build+test gate; SC-003 parity holds (same Rust loader) |
| T016 | ✅ VERIFIED | check tests (test_check.py, 11 fns) all finding kinds + purity |
| T017 | ✅ VERIFIED | `check` delegates to `prompting_press::check`; CheckReport/Finding |
| T018 | ✅ VERIFIED | build+test gate |
| T019 | ✅ VERIFIED | compose tests (test_compose.py, 14 fns) order/partial/empty/no-.chain() |
| T020 | ✅ VERIFIED | `Composition` resolve loop calls kernel render directly; no .chain() |
| T021 | ✅ VERIFIED | build+test gate |
| T022 | ✅ VERIFIED | facade `__init__.py` re-exports 13 symbols + PromptDefinition; `__all__` |
| T023 | ✅ VERIFIED | README quickstart + guard doctrine (281 lines in diff) |
| T024 | ✅ VERIFIED | abi3 wheel built (`target/wheels/...cp310-abi3...whl`); fresh-venv import + render confirmed |
| T025 | ✅ VERIFIED | full gate suite green (qa-report SC table; re-confirmed cargo+pytest) |
| T026 | ✅ VERIFIED | SC coverage walk (qa-report.md, SC-001…SC-011 mapped) |
| T027 | ✅ VERIFIED | roadmap "token hook" line reconciled (roadmap.md +43/-? in diff) |
| T028 | ✅ VERIFIED | `scripts/ci/check-advisories-py.sh` + ci/moon.yml + ci.yml wiring in diff |

## Unassessable items (SKIPPED)

None.

## Flagged items

None — verification complete with no flagged items.
