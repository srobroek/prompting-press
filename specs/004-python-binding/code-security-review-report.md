# Code Review + Security Review — Spec 004 (Python binding `prompting-press-py`)

**Date**: 2026-06-27 · **Steps**: 12 (code-review) + 13 (security-review), run in parallel.
**Scope**: `git diff main..HEAD` (binding crate + Python facade + tests + CI), through commit `0e537c8`.

> **Provenance**: 2 subagents (security-auditor, code-reviewer). Both passed integrity checks (correct
> line counts, real file:line citations); the verify-tasks fabrication did not recur. Both load-bearing
> findings (M-1, I1) re-verified main-thread against the actual code/CI before inclusion.

## Verdict

**No CRITICAL/HIGH. Code is ship-ready.** Two findings worth fixing before merge: **M-1** (a narrow
PII-leak fallback in the validation-error mapper — directly weakens the spec's named SEC-004-PY control)
and **I1** (the Python + binding test suites are not wired into CI — every Python-observable guarantee
is currently un-gated). Remaining items are hardening/cleanup follow-ups.

## Findings (triaged)

### Fix now
| ID | Sev | Source | Finding | Evidence |
|----|-----|--------|---------|----------|
| M-1 | MEDIUM | security | Validation-error fallback `return err.clone_ref(py)` surfaces the **raw** `pydantic.ValidationError` if `.errors()` introspection fails; its `str()`/`errors()` embed the rejected `input_value` (PII/secret leak) — the exact gap SEC-004-PY exists to close. Also a latent C-06 "native type never crosses FFI" violation. Trigger is near-unreachable (`.errors()` is stable) but the control must hold by construction. | `render.rs:336-344` |
| I1 | IMPORTANT | code | CI runs **no** Python tests, **no** `cargo test -p prompting-press-py`, **no** `maturin`. The `gates` job runs static gates; the `build` job only `cargo build --workspace --locked`. The 56 pytest + 30 binding tests — the only coverage for validate-in-Python, the SEC-004-PY Pydantic scrub, real `@field_validator` behavior, and 4-surface loader parity — can rot green. | `.github/workflows/ci.yml:34-147`; `packages/python/moon.yml:13-24` (only a `codegen` task) |

**M-1 remediation**: on introspection failure, raise a fixed-message `PromptValidationError` with zero
rows (e.g. "validation failed (error detail withheld)"), mirroring the kernel scrubber — never surface
the raw object. Apply the same withhold-detail pattern to L-3.

**I1 remediation**: add a CI job (or extend `build`) that builds the extension (`maturin develop`) into a
venv and runs `pytest packages/python/tests` + `cargo test -p prompting-press-py`. OR record an explicit
decision to defer the Python test gate to spec 007 — but do not leave it an implicit gap.

### Defer (hardening / cleanup follow-ups — track, not blockers)
| ID | Sev | Source | Finding |
|----|-----|--------|---------|
| L-1 | LOW | security | `to_kernel_value` / `depythonize` recursion unbounded on the `insert(dict)` path → deep input could stack-overflow (process abort/DoS). Render/compose paths are pre-bounded by pydantic-core's recursion guard. `marshal.rs:55-62`, `registry.rs:158`. |
| L-3 | LOW | security | `model_dump_json` failure text folded verbatim into `LoadError` message (`registry.rs:136-151`) — could echo a field value. Fix alongside M-1. |
| L-2 | LOW | security | `Message.__repr__` includes full rendered `text` by design (`compose.rs:96-98`) — logging-awareness note, not a defect. |
| S1 | SUGG | code | `test_compose.py:275-282` sentinel assertion is logically vacuous (passes by testing the sentinel, not `resolve`); the `pytest.raises` block above already proves the guarantee. Delete the redundant block. |
| S2 | SUGG | code | README `pip install prompting-press` is aspirational (version `0.0.0`, unpublished). Add a "not yet published" note. |
| S3 | SUGG | code | Verify `generated/__pycache__` is gitignored (a committed `__pycache__` could mask codegen drift). |
| D-QS | INFO | qa | quickstart.md US1 example uses dict-row access `row["field"]`; real API is attribute `row.field`. Doc nit. |

## Confirmed strengths (positive security/compliance evidence)

- SEC-004 kernel scrub verified end-to-end (routes through consumer scrubber first, never reads `.detail`;
  no `__str__`/`__repr__` override; seeded-secret unit test). Pydantic mapper copies `msg`/`loc` only.
- No `unsafe` anywhere; no panic-prone code on the public surface (every `unwrap`/`expect` is `#[cfg(test)]`
  or the one documented-sound borrow); all FFI errors → structured `PromptingPressError`.
- Boundary (Principle III) clean: no I/O, network, env, subprocess, eval, or token counting at runtime.
  (`build.rs` invokes Python at build-time only for `LIBDIR`.)
- Supply chain pinned (uv 0.11.8, pip-audit==2.9.0, dmcg==0.65.1, locked pydantic with hashes); the
  Python advisory gate IS a real failing CI step.
- FFI isolation structurally enforced (`ci:check-ffi`); Principles I/II/VI/VII all confirmed at call sites;
  all 7 review fixes in `0e537c8` verified correct.

## Recommended disposition
Fix **M-1** + **I1** (route via fix-findings / cleanup), then proceed. Defer L-1/L-2/L-3/S1/S2/S3/D-QS to
the cleanup step (14) and roadmap-debrief follow-ups.
