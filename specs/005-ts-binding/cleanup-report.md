# Cleanup Report — Spec 005 (TypeScript binding `prompting-press-node`)

**Generated**: 2026-06-28 · **Step**: 14 (`/speckit.cleanup`, main thread)
**Scope**: `git diff main..HEAD` — the napi crate (7 modules) + TS facade + 4 test suites + 2 CI
scripts/gates + the Python C-11 conformance change.

## Summary

| Category | Found | Fixed | Tasks created | In report |
|----------|-------|-------|---------------|-----------|
| Critical | 0 | — | — | — |
| Small (auto-fix) | 0 | 0 | — | — |
| Medium (tech-debt) | 5 | — | tracked → roadmap-debrief | — |
| Large | 0 | — | — | — |

The diff was already clean going into cleanup — it passed verify + a 5-aspect review.run + 3 deep idiom
audits + the code/security-review pair, with all in-scope findings fixed across commits `329cd20`,
`895a592`, `df60ade`, `5c54e2c`, `d61ad9a`. The hygiene scan surfaced **nothing auto-fixable.**

## Hygiene scan results

- **Debug artifacts** (`dbg!`/`println!`/`eprintln!` outside `#[cfg(test)]`; JS `console.log`/`debugger`): none.
- **Dead code / stray `#[allow]`**: none in the node crate.
- **TODO/FIXME/HACK/XXX**: none in the 005 sources or the CI scripts.
- **Hardcoded secrets / credentials**: none outside `#[cfg(test)]` / test fixtures.
- **Build-artifact leakage** (`*.node`, `*.tgz`, napi `index.{js,d.ts}`): none tracked/staged (gitignored).
- **Boundary (Principle III)**: no I/O, network, eval, subprocess, env, or token surface — confirmed by
  the security review.

## Tech-debt follow-ups (MEDIUM — tracked, not auto-fixed; carry to roadmap-debrief)

These are the deferred items from the review cycle + the three idiom audits. None blocks the spec; all
are recorded in `review-report.md` / `code-security-review-report.md`.

1. **Python binding: ship `py.typed` + a `.pyi` stub** (Python audit I-1) — the compiled PyO3 extension
   presents an *untyped* surface to mypy/pyright; the one real capability gap for a typed-prompt library.
2. **`decodeAddonError` row-shape validation** (TS audit S1 / security M1) — **DONE this cycle** (`d61ad9a`);
   listed for completeness.
3. **Rust `summarize` → idiomatic `write!`** (`prompting-press-py/src/error.rs:233`, Rust audit py-1) —
   the one real micro-nit; align with the consumer's `Display` form. Trivial.
4. **Python tests `conftest.py`** to de-dup the `_registry` helper (Python audit S-1).
5. **`isSchema` duck-typing in TS `render`** (TS audit I2 / code-review CR-S2) — accepted as-is (render
   keeps schema+data positional, below the C-11 optional-tail rule); a doc note is the only candidate.

## Accepted (no action)
- `--audit-level high` lets `moderate` through (deliberate; matches Rust/Python gates).
- Rust kernel + consumer stay positional (single `Option<&str>` is below the C-11 Rust threshold — by decision).

## Note on working-tree state
`.claude/agents/*.md` (6 files) + `apm.lock.yaml` show as modified but are an **APM recompile** (agent
renames + `apm_version 0.21.0 → 0.22.0`) — unrelated tooling/environment drift, NOT spec-005 work.
Deliberately left untouched (the user's APM-management concern, not this branch's).

## Validation status
- `cargo test -p prompting-press-node` 36; `node:test` 59; `ci:test-python` (cross-check) green.
- `cargo clippy -D warnings` + `cargo fmt --check` clean; tsc strict clean.
- `ci:check-ffi`, `ci:check-floating-versions`, `schemas:codegen-check`, `ci:check-advisories(-node)` green.
- Constitution v1.1.0: no violations (C-11 conformant; verified by code-review).

## Next steps
1. `/speckit.sync.analyze` ∥ `/speckit.sync.conflicts` (15/16).
2. Carry the 5 tech-debt follow-ups (esp. #1 py.typed/.pyi) to roadmap-debrief.
