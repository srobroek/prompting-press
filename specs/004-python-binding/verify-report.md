# Verify Spec Summary — 004-python-binding

- Spec: 004-python-binding (Python binding `prompting-press-py` → `packages/python`)
- Branch: `004-python-binding` (base `main`)
- Verdict: **PASS**
- Requirements checked: 25 FR + 11 SC = 36
- Implemented: 36
- Partial: 0
- Missing: 0
- Diverged: 0
- Inconclusive: 0 (SC-009/SC-011 runtime execution relies on the main thread's reported
  test run — see Notes; all wiring/artifacts verified statically)

## Integrity check

- `specs/004-python-binding/spec.md` = 447 lines (expected 447) — match.
- `specs/004-python-binding/tasks.md` = 203 lines (expected 203) — match.
- Git diff stat could not be run directly (no shell tool in this agent); the 40-file/6699-line
  claim is unverified by me, but every load-bearing source file named below was read and is real
  and substantial. Tool channel is not corrupted — files are intact.

## Functional Requirement Details

| ID | Status | Evidence | Gap |
|----|--------|----------|-----|
| FR-001 | IMPLEMENTED | Pydantic Vars w/ `@field_validator` accepted by `render`; tests `packages/python/tests/test_render.py:47-58,165-177`. | — |
| FR-002 | IMPLEMENTED | `render` validates via `model_validate` *before* templating; `crates/prompting-press-py/src/render.rs:217-220,277-308`. Test asserts no render on invalid input (`test_render.py:165-177`). | — |
| FR-003 | IMPLEMENTED | Validation owned in Python; only marshaled values cross FFI; kernel reached via `prompting_press_core::render` (`render.rs:237`). | — |
| FR-003a | IMPLEMENTED | Lossless bridge `marshal::to_kernel_value` (`marshal.rs:55-62`); int/float/None/nested round-trip tests `marshal.rs:78-184`. | — |
| FR-004 | IMPLEMENTED | Pydantic `ValidationError` normalized via `validation_error_to_pyerr` (`render.rs:334-374`), never surfaced; test `test_render.py:185-195`. | — |
| FR-005 | IMPLEMENTED | `load_yaml`/`load_json`/`insert` all route through the consumer loader (`registry.rs:60-162`); 4-surface parity test `test_loader.py:102-147`. | — |
| FR-006 | IMPLEMENTED | YAML≡JSON structural parity test `registry.rs:312-335` + Python `test_loader.py:154-168`. | — |
| FR-007 | IMPLEMENTED | Malformed → `LoadError`, nothing inserted; `registry.rs:338-354`, `test_loader.py:176-243`. | — |
| FR-008 | IMPLEMENTED | Codegen'd `PromptDefinition` from JSON Schema (`generated/prompt_definition.py` header + `codegen.sh`); single representation. | — |
| FR-008a | IMPLEMENTED | `Registry` pyclass; absent name → `UnknownPromptError` (`render.rs:208-215`, `test_loader.py:296-308`). | — |
| FR-009 | IMPLEMENTED | `render(reg, name, vars, data, variant, guard)` returns `RenderResult{text,name,variant,template_hash,render_hash,guard}`; guard plumbed (`render.rs:197-240,541-593`). No per-prompt Vars pre-registration. | — |
| FR-010 | IMPLEMENTED | `get_source` delegates to `prompting_press::get_source` (`render.rs:252-263`). | — |
| FR-011 | IMPLEMENTED | No render/agreement/variant/hash logic in binding — verified across all 7 src files; each delegates to core/consumer. | — |
| FR-012 | IMPLEMENTED | `Composition.from_messages`/`append`/`resolve` → ordered `list[Message]` (`compose.rs:124-256`); order tests `test_compose.py:145-203,361-379`. | — |
| FR-013 | IMPLEMENTED | No `.chain()`; `append` returns None; test `test_compose.py:302-308`. | — |
| FR-014 | IMPLEMENTED | Exception hierarchy under `PromptingPressError` (`error.rs:108-144`); EXHAUSTIVE `ConsumerError` match `error.rs:179-216`; closed code vocab via `prompting_press::error::code`. | — |
| FR-015 | IMPLEMENTED | SEC-004 scrub: kernel errors routed through consumer scrubber first (`error.rs:224-227`); secret-scrub test `error.rs:271-323`; no `__str__`/`__repr__` override re-introducing rows. | — |
| FR-016 | IMPLEMENTED | `check(reg)` delegates to `prompting_press::check` (`check.rs:177-180`); undeclared-var test `test_check.py:107-130`. | — |
| FR-017 | IMPLEMENTED | Analysis obtained from core; no re-derivation; declared set is the def's `variables` block, not Pydantic. | — |
| FR-018 | IMPLEMENTED | `untrusted_without_guard` finding surfaced (`check.rs:160-167`); test `test_check.py:149-193` (incl. guard under meta/metadata clears). | — |
| FR-019 | IMPLEMENTED | `check(&Registry)` is `&`-borrow → mutation impossible; purity test `test_check.py:254-287`. | — |
| FR-020 | IMPLEMENTED | Findings name prompt/variant/field; all 4 kinds reachable incl. reserved-default + analysis-error (`test_check.py:201-242`). | — |
| FR-021 | IMPLEMENTED | maturin abi3 wheel; `abi3-py310` (`Cargo.toml:61`), `requires-python >=3.10`, module `prompting_press`, dist `prompting-press` (`pyproject.toml:23,30,57`). | — |
| FR-022 | IMPLEMENTED | `pyo3`/`pythonize` only in `-py` (`Cargo.toml:59-67`); `ci:check-ffi` gate covers core + consumer (`scripts/ci/check-ffi-isolation.sh:22-25`). | — |
| FR-023 | IMPLEMENTED | No I/O / model calls / token counter; `output_model` metadata-only (`generated/prompt_definition.py:120-125`); README boundary §`README.md:35-42`. | — |
| FR-024 | IMPLEMENTED | Codegen freshness gate `schemas:codegen-check` regenerates + asserts clean git diff (`schemas/moon.yml:33-47`). | — |
| FR-025 | IMPLEMENTED | `check-advisories-py` task + `scripts/ci/check-advisories-py.sh` (pip-audit over uv.lock); registered `ci/moon.yml:41-46`. | — |

## Success Criterion Details

| ID | Status | Evidence |
|----|--------|----------|
| SC-001 | PASS | Validate-then-render single call path; no kernel/Pydantic type on surface. `test_render.py:145-157,185-195`. |
| SC-002 | PASS | Reject-before-render, every offending field named, no render performed. `render.rs:217-220`; `test_render.py:165-177`. |
| SC-003 | PASS | YAML/JSON/dict/instance → identical text + both hashes. `test_loader.py:102-147,154-168,246-262`. |
| SC-004 | PASS | Undeclared-variable detection naming prompt/variant/var; mutates/renders nothing. `test_check.py:107-130,254-287`. |
| SC-005 | PASS | Untrusted-without-guard flagged naming prompt+field. `test_check.py:149-171`. |
| SC-006 | PASS | No native error type on public API; common `{field,code,message}` shape, closed code vocab. `test_render.py:185-195`; `error.rs:179-216`. |
| SC-007 | PASS | `pyo3` only in `-py` (FFI gate); zero engine logic (all 7 src files delegate). `Cargo.toml`; `check-ffi-isolation.sh`. |
| SC-008 | PASS | N entries → N ordered `{role,text}`, each with own validated vars. `test_compose.py:145-203,361-379`. |
| SC-009 | PASS* | abi3 wheel buildable + fresh-env import + render/check/compose execute. Wiring + abi3 target verified statically; main thread reports built cp310-abi3 wheel, fresh-env import, 50 pytest pass. |
| SC-010 | PASS | Codegen byte-identical on regeneration (`schemas:codegen-check`); no token surface — generated shape has no token field, README states "no `count_tokens` surface at all". |
| SC-011 | PASS* | Advisory gate `check-advisories-py.sh` scans `packages/python/uv.lock` for CVEs and fails on a vulnerable dep; task registered. Static wiring verified; execution per main thread / CI. |

`*` SC-009/SC-011 are runtime-execution criteria. I verified the artifacts, task wiring, abi3
target, and gate scripts statically; the actual `maturin build` + fresh-env import + pip-audit
run were reported green by the main-thread verification this session.

## Findings By Severity

### Must Fix Before Proceeding
- None.

### Should Address
- None. The implementation satisfies every FR and SC with backing evidence.

### Notes
- **Constitution alignment confirmed.** Principle II: `pyo3`/`pythonize` confined to `-py`; the
  FFI gate enumerates `prompting-press-core` + `prompting-press` and uses `cargo tree -i`
  (`check-ffi-isolation.sh:45-60`). Principle III: README explicitly documents no I/O, no LLM
  call, no token counter, `output_model` as metadata only; generated shape carries no token
  field. Principle VI: Pydantic facade, `from_messages` (no `.chain()`), errors normalized to the
  shared `{field,code,message}` shape. Principle VII: Python shape codegen'd, one dual-input
  loader. Principle I: render path calls `prompting_press_core::render` directly (critique E1) —
  still zero engine logic; parity stays structural via the shared `from_serialize` value path.
- **Risk-pattern: interface extensions / exhaustive matches.** Both the `ConsumerError`→PyErr map
  (`error.rs:179-216`) and the `FindingKind`→discriminant map (`check.rs:160-167`) are exhaustive
  with NO wildcard arm — a new core variant is a compile error, not a silent fallthrough. Good.
- **Risk-pattern: SEC-004 secret scrub.** Verified at the Python surface, not just the Rust
  string: the test asserts `str(exc)`, `repr(exc)`, AND every `exc.errors` row are secret-free
  (`error.rs:271-323`); the Pydantic mapper copies only `msg`, never `input`/`ctx`
  (`render.rs:353-374`), with a Python-side proof (`test_render.py:203-227`).
- **Risk-pattern: output completeness.** Both provenance hashes are surfaced 1:1 on `RenderResult`
  and asserted as 64-char lowercase hex in both Rust (`render.rs:452-462`) and Python
  (`test_render.py:154-155`) tests.
- **Three-sets gap is loud, not silent.** A Vars/template field-name mismatch surfaces as a
  `PromptRenderError` with `code=="undefined_variable"` rather than an empty render
  (`render.rs:500-539`; `test_render.py:235-254`) — the spec edge case is pinned by a test.
- **abi3 floor reconciled (clarify Q4).** `abi3-py310` (`Cargo.toml:61`) now agrees with
  `requires-python >=3.10` and the codegen `--target-python-version 3.10`; the generated
  `X | None` syntax requires 3.10, so the prior `abi3-py39` install-then-ImportError trap is
  closed. The hierarchy uses `create_exception!` (not `#[pyclass(extends=PyException)]`) precisely
  because field-carrying native-exception subclassing requires Python ≥3.12 under abi3 — documented
  at `error.rs:13-22`.
- **Token-surface drop (F4).** No token hook/counter anywhere; README states it explicitly; the
  T025/T026 gate includes a narrow `rg` for `count_tokens|token_count|TokenCount|count-tokens`.

## Verification Commands
- `wc -l specs/004-python-binding/spec.md`: not run (no shell tool); confirmed 447 via file read.
- `wc -l specs/004-python-binding/tasks.md`: not run; confirmed 203 via file read.
- `cargo test -p prompting-press-py`: not run by this agent; main thread reports 30 passed.
- `pytest packages/python/tests`: not run by this agent; main thread reports 50 passed.
- `moon run ci:check-ffi`: not run; gate script + COVERED_CRATES verified statically.
- `moon run schemas:codegen-check`: not run; gate task + script verified statically.
- `moon run ci:check-advisories-py`: not run; task + script verified statically.
