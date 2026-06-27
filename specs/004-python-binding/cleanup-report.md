# Cleanup Report — Spec 004 (Python binding `prompting-press-py`)

**Generated**: 2026-06-27 · **Step**: 14 (`/speckit.cleanup`, main thread)
**Scope**: `git diff main..HEAD` — binding crate (7 modules + build.rs), Python facade + 4 test suites,
CI wiring, advisory + test scripts.

## Summary

| Category | Found | Fixed | Tasks created | In report |
|----------|-------|-------|---------------|-----------|
| Critical | 0 | — | — | — |
| Small | 0 | 0 | — | — |
| Medium | 1 | — | 1 (TD001) | — |
| Large | 0 | — | — | — |

The diff was already clean going into cleanup (it passed verify + a 5-aspect review.run + a
code-review/security-review pair, with 9 findings already fixed in `0e537c8`/`ee28a82`). The hygiene
scan surfaced **nothing auto-fixable**.

## Hygiene scan results

- **Debug artifacts** (`dbg!`/`println!`/`eprintln!` outside tests; Python `breakpoint`/`pdb`/`print`): none.
- **Dead code / stale `#[allow]`**: none (the one stale `#[allow(dead_code)]` was already removed in `0e537c8`).
- **TODO/FIXME/HACK/XXX**: none. (Two grep hits were false positives — a `mktemp XXXXXX` template in
  `test-python.sh` and the `#[cfg(test)]` SEC-004 `sk-…` fixture in `error.rs`.)
- **Hardcoded secrets / credentials**: none outside `#[cfg(test)]` fixtures.
- **Boundary (Principle III)**: no I/O, network, eval, subprocess, or token surface — confirmed in prior gates.

## Medium issue → task created

- **TD001** (security review L-1): unbounded `depythonize` recursion on the `insert(dict)` / marshal
  path could stack-overflow on pathologically deep input (process abort/DoS). Not a release blocker —
  it is the trusted-caller `insert(dict)` path, and render/compose are already depth-bounded by
  pydantic-core. Appended to `tasks.md` under "Tech Debt Tasks". Judgment-call (depth cap vs. document) —
  not auto-fixed.

## Accepted (no action — documented design)

- **L-2**: `Message.__repr__` includes rendered `text` by design (the caller's own output; logging note).
- **S3**: `generated/__pycache__` verified untracked + gitignored.

## Roadmap follow-ups (not tech debt — for roadmap-debrief)

- **D-1**: `RenderResult`/`Finding` value-equality (`__eq__`/`__hash__`) — content-addressed but
  identity-compared. New behavior beyond any FR/SC.
- **D-2**: `.pyi` type stubs for downstream static typing.

## Validation status

- Linter: `cargo clippy -p prompting-press-py -- -D warnings` clean; `cargo fmt --check` clean.
- Tests: `cargo test -p prompting-press-py` 30; `pytest packages/python/tests` 56 (via `ci:test-python`).
- Constitution: no violations (delegation-only confirmed; FFI isolation gated; no token surface).

## Next steps

1. Proceed to `/speckit.sync.analyze` ∥ `/speckit.sync.conflicts` (steps 15/16).
2. Address TD001 in a future iteration (or fold into spec 005/007).
3. Carry D-1/D-2 into the roadmap-debrief as deferred follow-ups.
