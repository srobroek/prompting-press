# US4 CI Gates — Implementation Report (T028–T030a)

## Summary

Five CI gates implemented as moon tasks (locally runnable, CI calls via `mise exec -- moon run`). A GitHub Actions workflow wires them with OS build matrix.

---

## Gate scripts and moon tasks

### T028 — FFI-isolation gate

**Script:** `scripts/ci/check-ffi-isolation.sh`

- Iterates an explicit, reviewable `COVERED_CRATES` list (`prompting-press-core`, `prompting-press`).
- For each crate × FFI toolkit (`pyo3`, `napi`): runs `cargo tree -p <crate> -i <ffi>` and checks for "did not match any packages" in the output. Uses `|| true` to prevent `set -e` from aborting on the expected non-zero exit when the package is absent.
- Exits 1 with a message citing Principle II / C-02 if any FFI toolkit is found.
- Maintainer note in script: add new FFI-free crates to `COVERED_CRATES` explicitly.

**Moon task:** `ci:check-ffi` (project `ci`, `ci/moon.yml`)
```
mise exec -- moon run ci:check-ffi
```

### T029 — Codegen-freshness gate

**Script:** `schemas/scripts/codegen-check.sh` (pre-existing, T027)

The existing gate already:
- Is called after all three `:codegen` tasks run (moon `deps`).
- Uses `git add -N` + `git diff --exit-code` over all three generated paths, catching both modified and newly untracked files.
- Names drifted files in its error output.

No changes needed. The Python generated file (`packages/python/python/prompting_press/generated/prompt_definition.py`) is covered by the `${PY}` variable in the script.

**Moon task:** `schemas:codegen-check` (pre-existing)
```
mise exec -- moon run schemas:codegen-check
```

### T030 — Schema + fixtures in CI

Both gates pre-exist from US2/US3. The workflow simply calls them:

**Moon tasks:** `schemas:check-schema`, `schemas:validate-fixtures`
```
mise exec -- moon run schemas:check-schema
mise exec -- moon run schemas:validate-fixtures
```

### T030a — Floating-version lint

**Script:** `scripts/ci/check-floating-versions.sh`

Scans explicit manifests only (lockfiles excluded):
- `mise.toml`
- `Cargo.toml` (workspace)
- `packages/*/package.json` (maxdepth 2, excludes `node_modules/`)
- `packages/*/pyproject.toml`
- `crates/*/Cargo.toml`

Flags literal `^`/`~` in JSON strings, `"latest"`, `"*"` as version values, and their TOML equivalents. Does NOT flag bounded ranges (`>=1.14,<2.0` in `pyproject.toml` — acceptable per SEC-003). Confirmed `mise.toml`'s `jq = "1.8.1"` passes.

**Moon task:** `ci:check-floating-versions` (project `ci`, `ci/moon.yml`)
```
mise exec -- moon run ci:check-floating-versions
```

---

## ci project

Added `ci/moon.yml` (language: unknown, no global task inheritance) and registered `ci: "ci"` in `.moon/workspace.yml`. `moon query projects` now returns 8 projects:
```
ci
prompting-press
prompting-press-core
prompting-press-node
prompting-press-py
prompting-press-python
prompting-press-typescript
schemas
```
`packages/go` remains absent (intentional, FR-005).

---

## GitHub Actions workflow — .github/workflows/ci.yml

Triggers: `push`, `pull_request`.

### `gates` job — ubuntu-latest (single runner)

OS-independent checks run once:
1. `ci:check-floating-versions` (T030a)
2. `ci:check-ffi` (T028)
3. `schemas:check-schema` (T030)
4. `schemas:validate-fixtures` (T030)
5. `schemas:codegen-check` (T029)

Codegen-freshness is single-runner to avoid rustfmt/EOL drift across OSes producing false positives.

### `build` job — matrix: ubuntu-latest, macos-latest, windows-latest

Steps per leg:
1. `actions/checkout@<sha>`
2. `actions/setup-python@<sha>` — Python 3.12
3. `jdx/mise-action@<sha>` — installs pinned toolchain from `mise.toml`
4. `cargo build --workspace`

**Action versions pinned to commit SHA (no floating `@main`/`@latest`):**
- `actions/checkout` → `34e114876b0b11c390a56381ad16ebd13914f8d5` (v4)
- `jdx/mise-action` → `c37c93293d6b742fc901e1406b8f764f6fb19dac` (v2)
- `actions/setup-python` → `a26af69be951a213d495a4c3e4e4022e16d87065` (v5)

---

## Windows pyo3 link handling

`crates/prompting-press-py` uses `pyo3` with `extension-module` + `abi3-py39`. The standard CI recipe for abi3 on Windows is `actions/setup-python` before `cargo build` so pyo3's build script locates a CPython installation and resolves the import library for the cdylib link.

The workflow adds `actions/setup-python` (Python 3.12) on ALL matrix legs before the mise step. This is harmless on Linux/macOS (where `extension-module` + the macOS `build.rs` `-undefined dynamic_lookup` already handle the link without a live interpreter) and required on Windows.

**Residual Windows risk:** The macOS `build.rs` branch (`cfg!(target_os = "macos")`) has no Windows branch. If the abi3 + setup-python approach is insufficient on Windows runners (e.g. pyo3 can't locate the import lib from the PATH Python), `cargo build` may fail with a link error on the Windows leg. This cannot be validated locally without a Windows runner. The `setup-python` approach is the documented upstream recipe for abi3 Windows CI; if it fails, a Windows-specific `build.rs` branch (`cfg!(target_os = "windows")`) to link against `python3.lib` may be needed. This is flagged as a post-push validation item.

---

## Local pass output (clean tree)

```
$ mise exec -- moon run ci:check-floating-versions ci:check-ffi schemas:check-schema schemas:validate-fixtures schemas:codegen-check

schemas:check-schema       | OK: prompt-definition.schema.json is a valid JSON Schema Draft 2020-12 document.
schemas:validate-fixtures  | Summary: 10/10 expectations met ALL PASS
schemas:codegen-check      | codegen-check PASSED — all three generated files are up-to-date.
ci:check-ffi               | FFI-isolation gate PASSED.
                           |   prompting-press-core: no pyo3, no napi in dependency tree
                           |   prompting-press: no pyo3, no napi in dependency tree
ci:check-floating-versions | Floating-version lint PASSED — all manifests use pinned versions.
                           |   OK: mise.toml, Cargo.toml, packages/typescript/package.json,
                           |       packages/python/pyproject.toml, crates/*/Cargo.toml

Tasks: 8 completed (5 cached)   Time: 12s 472ms
```

---

## YAML validity

```
$ mise exec -- uv run --with pyyaml python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/ci.yml')); print('yaml ok')"
yaml ok
```

---

## What can only be validated post-push

- GitHub Actions matrix execution (all three OS legs of the `build` job).
- Windows pyo3 abi3 link resolution — the `setup-python` approach is the standard recipe; see residual Windows risk above.
- Actual `mise-action` runner behaviour (tool caching, PATH setup) across runner OSes.
- Any GitHub-side YAML schema validation beyond well-formedness (e.g. step name limits, unknown context vars).
