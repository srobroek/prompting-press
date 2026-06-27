# Review Report — Spec 004 (Python binding `prompting-press-py`)

**Date**: 2026-06-27 · **Cycle**: `/speckit.review.run` (code, errors, tests, types, comments; simplify pending)
**Scope**: `git diff main..HEAD` code files (binding crate + Python facade + CI). `detect-changed-files.sh`
missing → merge-base diff workaround.

> **Provenance**: 5 parallel review subagents. All 5 passed an integrity check (correct file line-counts,
> real file:line citations) — credible, unlike the verify-tasks subagent which fabricated and was discarded.
> Every finding below was re-verified main-thread against `rg`/file reads before inclusion.

## Headline

**No CRITICAL code defect. No IMPORTANT code defect.** verify (FR/SC) PASSED, verify-tasks 28/28. The
binding upholds every governing constraint: zero engine logic (delegates to core/consumer), FFI isolation,
SEC-004 scrub, panic-free FFI, exhaustive no-wildcard error/finding matches, error normalization. What
remains is **test-coverage gaps + small polish**.

## Findings (triaged)

### Fix now (scout-rule — cheap, safe, clear)
| ID | Source | Finding | Evidence |
|----|--------|---------|----------|
| F-A | comments-S3 | `#[allow(dead_code)]` on `Registry::inner()` is stale; 5 live non-test callers; comment describes a past state | `registry.rs:170-175`; callers `render.rs:210/233/260`, `check.rs:179`, `compose.rs:228` |
| F-B | comments-I1 | README check example: `is_empty()` trailing comment invites misreading (it's an alias for `passed()`; `bool()`/`len()` are the inverse) | `README.md:150-151`; `check.rs:67-79` |
| F-C | tests-I3 | No pytest asserts the no-token-surface guarantee (F4/SC-010); suite already uses `not hasattr` idiom for `.chain` | none in `tests/`; cf. `test_compose.py:305` |
| F-D | code-SUGG1 | `render` resolves the prompt twice (`is_none()` then `.expect()`); resolve-once removes the only production `.expect()` and matches `compose.rs:228` | `render.rs:210` + `render.rs:232-235` |

### Fix now (test coverage — behavioral gaps on security/public surface)
| ID | Source | Finding | Evidence |
|----|--------|---------|----------|
| F-E | tests-C1 | SEC-004-PY untested for a **real kernel render error** carrying a secret (only the Pydantic-validator path is tested Python-side; kernel path only synthetic Rust). Craftable via a value that forces a kernel render error. | `test_render.py:203-227` |
| F-F | tests-I4 | `get_source` has no Python behavioral test — only `callable()` smoke; no assertion it returns *unrendered* source or raises on a bad name | `test_render.py:318` |
| F-G | tests-I1 | SC-002 "names **every** offending field" tested single-field only in Python; multi-row only via hand-built Rust `ConsumerError` (bypasses real `collect_validation_rows`) | `test_render.py:165`; `error.rs:366-389` |

### Defer (out of v1 scope — track, don't implement now per Scope Discipline)
| ID | Source | Finding | Disposition |
|----|--------|---------|-------------|
| D-1 | types-I2 | `RenderResult`/`Finding`/`FieldError` have no `__eq__`/`__hash__` → identity comparison despite content-addressed hashes. A value-equality contract is a real DX enhancement but new behavior beyond any FR/SC. | Roadmap follow-up (debrief). Generality earned by a real consumer need. |
| D-2 | code-SUGG2 | No `.pyi` type stubs ship → no static types downstream. | Roadmap follow-up (published-wheel DX). |
| D-3 | tests-I2 | `join_loc` nested/list-loc path (`render.rs:376-397`) unexercised. | Low-value; fold into F-G if cheap, else accept. |

### Noise / accepted (no action)
- error-S1 (validation fallback surfaces raw pydantic on a near-impossible degenerate path — documented, defensible); error-S2/S3, types-S1/S2/S3, types-I1 (Vec-getter clone — correct for immutable result), types-I3 (lazy-import degradation — near-impossible), comments-S1/S2 (stale tasks.md ref — not code). One-line doc notes optional.

## Recommended action
Apply **F-A…F-G** via `fix-findings` (4 small polish + 3 test gaps; F-E closes a genuine security-test gap).
Defer **D-1/D-2** to roadmap-debrief follow-ups. Re-run pytest + gates, then proceed to `qa.run`.
