# Task T009 — `packages/python/` published-package skeleton (FR-004)

**Status:** Complete
**Date:** 2026-06-25
**Scope:** Files created only under `packages/python/`. No `crates/`, root `Cargo.toml`,
other packages, or spec artifacts (other than this report) were touched.

## Summary

Created a maturin-backed Python package skeleton that will wrap the PyO3 binding
crate `crates/prompting-press-py/`. No runtime logic ships (spec 001 ships none).
maturin **1.14.1** availability was verified before relying on it (critique E3 / CHK026).

## maturin verification (CHK026 / critique E3)

```
$ mise exec -- maturin --version
maturin 1.14.1
```

`mise exec -- maturin --help` was also run (output discarded) to confirm the CLI
is responsive. Per the task constraints, **no `maturin build` was attempted** —
this task wires the build, it does not exercise it. maturin is pinned to `1.14.1`
in `mise.toml` (`"pipx:maturin" = "1.14.1"`), the repo's single source of truth
for tool versions.

## Files created

### `packages/python/pyproject.toml`

```toml
# Published Python package: a thin wrapper around the PyO3 binding crate
# `crates/prompting-press-py/` (a cdylib). Built with maturin.
#
# SKELETON ONLY (spec 001 / FR-004): this wires the build backend and points it
# at the binding crate. No runtime logic ships here — the importable module is
# the compiled Rust extension, materialized once the crate (US1) and codegen
# (US3) land. Until then there is nothing to publish (version 0.0.0).

[build-system]
# maturin is the PyO3 packaging standard (research D1). The `>=1.14,<2.0`
# bound is the conventional, recommended form for build-system requires and is
# acceptable under the no-floating-versions rule (SEC-003), which targets the
# `^`/`~`/`"latest"`/`"*"` shorthands — not explicit bounded ranges. The exact
# build-time maturin is pinned to 1.14.1 in `mise.toml` (single source of truth).
requires = ["maturin>=1.14,<2.0"]
build-backend = "maturin"

[project]
# `prompting-press` is the PyPI distribution name. The importable module is
# `prompting_press` (see [tool.maturin].module-name).
name = "prompting-press"
version = "0.0.0" # pre-release: nothing is published in spec 001
description = "Python distribution of Prompting Press — a thin wrapper around the Rust core via PyO3."
readme = "README.md"
# Pydantic v2 target floor (US3 generates Pydantic v2 models). Decoupled from the
# repo's pinned dev runtime (mise.toml python = 3.14.4) on purpose: the published
# wheel must support the broader supported-Python range, not just the dev pin.
requires-python = ">=3.10"
license = { text = "Apache-2.0" }
authors = [{ name = "Sjors Robroek" }]
# Intentionally empty: no runtime logic yet. Pydantic and friends arrive with the
# generated models in US3.
dependencies = []

[project.urls]
Repository = "https://github.com/srobroek/prompting-press"

[tool.maturin]
# The binding crate lives OUTSIDE this package dir, at the repo's
# `crates/prompting-press-py/` (workspace member). Reference it explicitly.
# NOTE: this path is materialized by US1 (crate stubs, tasks T005–T008); this
# skeleton task (T009) is [P] and intentionally lands before the crate exists.
manifest-path = "../../crates/prompting-press-py/Cargo.toml"
# The importable Python module name (distinct from the PyPI dist name above).
module-name = "prompting_press"
# Build as a CPython extension module (the PyO3 standard for embeddable wheels).
features = ["pyo3/extension-module"]
# Mixed Rust/Python layout: Python-side sources live under `python/`, so the
# compiled extension and any future generated Python (US3 Pydantic models) share
# one importable package. See `python/prompting_press/__init__.py`.
python-source = "python"
```

### `packages/python/python/prompting_press/__init__.py`

```python
"""Prompting Press — Python distribution.

Skeleton package marker (spec 001 / FR-004). This ships no runtime logic: the
real public API is provided by the compiled Rust extension (PyO3 binding crate
``crates/prompting-press-py``), built and merged into this package by maturin
once US1 (crate stubs) and US3 (codegen) land.

``__version__`` is a placeholder mirroring the unpublished ``0.0.0`` in
``pyproject.toml``; it will be sourced from package metadata once published.
"""

__version__ = "0.0.0"
```

### `packages/python/README.md`

A short status/layout note (skeleton banner + directory tree). Documentation
only, no logic. See the file for full contents.

## Validation performed

- `pyproject.toml` parses as valid TOML (`tomllib`), and every required key was
  asserted programmatically: build-backend = `maturin`, requires =
  `["maturin>=1.14,<2.0"]`, name = `prompting-press`, version = `0.0.0`,
  requires-python = `>=3.10`, license = `{text = "Apache-2.0"}`, dependencies =
  `[]`, manifest-path = `../../crates/prompting-press-py/Cargo.toml`,
  module-name = `prompting_press`, features = `["pyo3/extension-module"]`,
  python-source = `python`. All passed.
- maturin CLI confirmed present and responsive (no build run).

## Decisions

1. **`module-name = "prompting_press"`** — the importable module name (underscore),
   distinct from the PyPI distribution name `prompting-press` (hyphen). This is
   standard maturin practice and matches the task spec.
2. **`python-source = "python"` + minimal `__init__.py`** — chose the mixed
   Rust/Python layout rather than a pure-extension layout. Rationale: the package
   is a *thin wrapper*, and US3 will emit Pydantic v2 models
   (`datamodel-code-generator`) that need a Python-side home alongside the
   compiled extension. Establishing the source dir now avoids a layout migration
   later. The `__init__.py` is logic-free (docstring + `__version__` placeholder).
3. **`requires-python = ">=3.10"`** — honored the task spec (Pydantic v2 floor),
   deliberately decoupled from the repo's pinned dev runtime (`mise.toml`
   python = 3.14.4). The published wheel must support the broader supported-Python
   range; the dev pin governs the build/test environment, not the wheel's floor.
4. **`requires = ["maturin>=1.14,<2.0"]`** — explicit bounded range, not a floating
   `^`/`~`/`"latest"`/`"*"`. This is the recommended build-system requires form
   and is compatible with SEC-003 (the no-floating-versions rule targets the
   shorthand floats, which a CI lint enforces in T030a). The exact build-time
   maturin is pinned in `mise.toml`.

## How this package wraps `crates/prompting-press-py` (US1 + US3)

- **US1** lands the binding crate stub at `crates/prompting-press-py/` (a PyO3
  `cdylib`, workspace member). `[tool.maturin].manifest-path` already points there;
  at T009 completion the crate's `Cargo.toml` had been created by the parallel
  crate-stub tasks, so the path resolves.
- **US3** runs `datamodel-code-generator` (pinned `0.65.1` in `mise.toml`) to emit
  Pydantic v2 models from the JSON Schema into the `python/prompting_press/`
  source tree, and the binding crate gains real PyO3 `#[pyfunction]`/`#[pymodule]`
  exports.
- At that point `maturin build` (then `maturin develop` / wheel publish) compiles
  the Rust crate referenced by `manifest-path` into a `prompting_press` CPython
  extension (via the `pyo3/extension-module` feature) and merges it with the
  Python-side sources under `python-source = "python"` into one importable
  package. The published distribution name remains `prompting-press`; the import
  remains `import prompting_press`.

## Notes on scope

`git status` showed untracked `Cargo.lock` and `crates/` appearing during this
task — those are artifacts of concurrent parallel tasks (the US1 crate stubs), not
this task. T009's writes are confined to `packages/python/`.
