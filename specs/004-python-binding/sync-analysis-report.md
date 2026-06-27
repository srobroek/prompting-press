# Sync Analysis (drift) — Spec 004 (Python binding)

**Date**: 2026-06-27 · **Step**: 15 (`/speckit.sync.analyze`)
**Scope**: spec.md / plan.md / tasks.md vs. the implementation (`git diff main..HEAD`).

> **Provenance**: the `speckit-sync` subagent hit the systemic tool-channel glitch (`tool_uses: 0` —
> it echoed bash commands without executing them and wrote no report; this is the exact failure the
> handover warned of, where the analyze agent previously hallucinated a fictional codebase). Its
> output was discarded. This analysis was performed main-thread against real `rg`/file evidence.

## Verdict: one low-severity stale task reference; no code↔spec drift.

### Stale reference (task-tracking only — code is correct)

- **D-A (LOW)** — `specs/004-python-binding/tasks.md:60` (T006) instructs building the exception base
  as "`create_exception!` + an `#[pyclass(extends=PyException)]` base carrying `.errors`". The
  implementation at `crates/prompting-press-py/src/error.rs:13-26` & `:103-144` deliberately uses
  **only** `create_exception!` and documents why `#[pyclass(extends=PyException)]` was rejected (it
  requires Python ≥ 3.12 under the `abi3-py310` floor). This is the **implementation correctly
  diverging from a stale task hint** — the code, spec, and constitution all agree; only the T006 task
  prose is stale. Already noted by the comments reviewer (review-report S1). No code change needed;
  the task text could be corrected for tidiness but is a closed task.

### No drift in the load-bearing directions

- **Spec→code (unbuilt scope):** none. All FR/SC are implemented and verified (verify-report PASS;
  qa-report 10/10; verify-tasks 28/28).
- **Code→spec (unspecced behavior):** none. The 18 exported symbols all map to specced surface
  (render/get_source/check/Composition/Message/RenderResult/CheckReport/Finding/FieldError/GuardConfig
  + the 5-member exception hierarchy + `PromptDefinition`). `core_version` is a minor diagnostic
  carried from the spec-001 stub — harmless, not new behavior.
- **Phantom completions:** none (verify-tasks 28/28, re-confirmed main-thread).
- **The 9 review fixes (0e537c8 / ee28a82) + TD001:** consistent with the spec; the new `ci:test-python`
  gate strengthens SC coverage, contradicts nothing.

## Evidence (quoted, main-thread)

- T006 drift: `tasks.md:60` "create_exception! + an #[pyclass(extends=PyException)] base" vs.
  `error.rs:13` "## Why `create_exception!`, not `#[pyclass(extends=PyException)]`".
- Delegation intact (no engine-logic drift): `render.rs:237` → `prompting_press_core::render`;
  `check.rs:179` → `prompting_press::check` (confirmed earlier this session).
