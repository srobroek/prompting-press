# T026 — SC Coverage Walk (spec 004 Python binding)

Every Success Criterion maps to a passing backing test or CI gate. Verified 2026-06-27 (impl phase,
T024–T026). Toolchain via `mise exec --`.

| SC | Criterion | Backing evidence | Status |
|----|-----------|------------------|--------|
| SC-001 | Idiomatic validate-then-render path, no native types on the API | `tests/test_render.py::test_valid_render_produces_text_and_hex_hashes` | ✅ |
| SC-002 | Invalid input rejected before render, every offending field named | `test_render.py::test_validation_failure_raises_before_render` | ✅ |
| SC-003 | YAML/JSON/object parity (identical render + provenance) | `tests/test_loader.py` parity (yaml, json, insert-dict, insert-model-instance) | ✅ |
| SC-004 | Agreement check detects undeclared-variable reference | `tests/test_check.py::test_undeclared_variable_is_flagged_naming_the_variable` | ✅ |
| SC-005 | Provenance lint flags untrusted/external-without-guard | `test_check.py` (untrusted-without-guard + guard-presence-clears, parametrized meta/metadata) | ✅ |
| SC-006 | No native error type on the public API | `test_render.py::test_validation_error_is_not_a_pydantic_error` (`not isinstance(pydantic.ValidationError)`) | ✅ |
| SC-007 | `pyo3` only in `-py`; no engine logic in the binding | `ci:check-ffi` gate (PASSED, `--force`) | ✅ |
| SC-008 | N composition entries → N ordered `{role,text}` messages | `tests/test_compose.py` (order+roles via both `from_messages` and `append`) | ✅ |
| SC-009 | maturin abi3 wheel builds; fresh-env `import` + render works | T024: `maturin build --release` → `prompting_press-0.0.0-cp310-abi3-*.whl`; installed in a clean `/tmp` venv (only pydantic pulled), `import prompting_press` + render → `'Hi Ada'`, 64-hex hashes | ✅ |
| SC-010 | Generated shape byte-identical to fresh regen; no token surface | `schemas:codegen-check` PASSED (all 3 generated files up-to-date, working tree clean); narrowed token grep finds nothing | ✅ |
| SC-011 | CI advisory gate scans Python deps (`uv.lock`) for CVEs | `ci:check-advisories-py` PASSED (24 packages audited, no known vulnerabilities) | ✅ |

## Full gate suite (T025) — all green

- `cargo test -p prompting-press-py` → 30 passed.
- `pytest packages/python/tests/` → 50 passed (9 render + 15 loader + 12 check + 14 compose, parametrized).
- `cargo clippy -p prompting-press-py --all-targets -- -D warnings` → clean.
- `cargo fmt -p prompting-press-py -- --check` → clean.
- `moon run :build` → 5 tasks completed.
- `ci:check-ffi`, `ci:check-floating-versions`, `ci:check-advisories` (Rust), `ci:check-advisories-py`,
  `schemas:codegen-check` → all PASSED (`--force`, not cache-masked).

## Notes

- SEC-004 secret-scrub is pinned both Rust-side (`error.rs` test) and Python-side
  (`test_render.py::test_rejected_sensitive_input_is_not_leaked`): a seeded secret never appears in
  `str(exc)`, `repr(exc)`, or any `.errors[*]` row.
- Guard plumbing (FR-009) verified: `RenderResult.guard` populated when `GuardConfig(enabled=True)` on
  an untrusted-declaring prompt, `None` otherwise, `.text` byte-identical either way (guard never in
  text — the system-prompt-addendum doctrine, documented in the README).
