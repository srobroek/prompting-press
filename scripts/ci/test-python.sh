#!/usr/bin/env bash
# Spec 004 — Python binding test gate (code/security review I1).
#
# Builds the `prompting-press-py` extension and runs BOTH test suites that the
# OS-matrix `cargo build --workspace` does NOT exercise:
#   1. cargo test -p prompting-press-py  — Rust-side marshaling + SEC-004 scrub
#      unit tests (run under `Python::attach`).
#   2. pytest packages/python/tests       — the ONLY coverage for the
#      validate-in-Python path, the SEC-004-PY Pydantic scrub, real
#      `@field_validator` behavior, and the four-surface loader parity. None of
#      these are reachable from the Rust tests (they need a live Pydantic model).
#
# WHY THIS GATE EXISTS: without it, the 56 pytest + 30 binding tests can rot
# green — a regression in `validate_in_python` or the Pydantic error mapping
# would pass CI. The OS-matrix `build` job compiles the crate (catching link
# regressions) but never runs its tests.
#
# Runtime: uv (mise-pinned at 0.11.8) builds a hermetic venv; maturin is run via
# `uvx --from maturin==<pin>` (exact pin — invisible to the floating-version
# gate). `maturin develop` is invoked from packages/python so it resolves the
# `[tool.maturin] module-name = "prompting_press"` config (running it from the
# repo root with `-m <crate>/Cargo.toml` mis-derives the module name from the
# crate name `prompting-press-py`, whose hyphen maturin rejects).
#
# MAINTAINER NOTES:
#   - To upgrade maturin: change MATURIN_PIN below (exact pin, no ^/~/>=).
#   - pydantic is the wheel's declared runtime dep; pytest is the only extra the
#     test run needs. Both are installed into the throwaway venv.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

MATURIN_PIN="maturin==1.14.1"
PYTHON_PKG="${REPO_ROOT}/packages/python"

echo "Python binding test gate (spec 004 / review I1)"
echo ""

# The `cargo test` binary links libpython (extension-module OFF), so it must find it at
# runtime. build.rs embeds an rpath to the interpreter LIBDIR, but a mise-/pyenv-managed
# interpreter can sit outside the system loader path — belt-and-suspenders, also export the
# LIBDIR on the platform loader-path var so the test binary loads `libpythonX.Y.so`/`.dylib`.
PY_LIBDIR="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR") or "")' 2>/dev/null || true)"
if [ -n "${PY_LIBDIR}" ]; then
  export LD_LIBRARY_PATH="${PY_LIBDIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_FALLBACK_LIBRARY_PATH="${PY_LIBDIR}${DYLD_FALLBACK_LIBRARY_PATH:+:${DYLD_FALLBACK_LIBRARY_PATH}}"
  echo "  libpython LIBDIR: ${PY_LIBDIR}"
fi

# Step 1 — Rust-side binding tests (marshaling + scrub; needs no wheel).
echo "==> cargo test -p prompting-press-py"
cargo test -p prompting-press-py

# Step 2 — build the extension into a hermetic venv, then run pytest.
VENV_DIR="$(mktemp -d /tmp/pp-py-test-XXXXXX)"
trap 'rm -rf "${VENV_DIR}"' EXIT

echo ""
echo "==> uv venv + build extension (maturin develop) into ${VENV_DIR}"
uv venv "${VENV_DIR}" --python 3.12
# pydantic: the wheel's runtime dep (Vars facade + generated shape). pytest: the runner.
# hypothesis==6.155.7: property-based fuzzing (spec 009 T008 — test-only; exact pin per FR-009).
VIRTUAL_ENV="${VENV_DIR}" uv pip install --python "${VENV_DIR}/bin/python" pytest pydantic "hypothesis==6.155.7"

# maturin develop must run from packages/python (resolves [tool.maturin] module-name).
( cd "${PYTHON_PKG}" \
  && VIRTUAL_ENV="${VENV_DIR}" uvx --from "${MATURIN_PIN}" maturin develop )

echo ""
echo "==> pytest packages/python/tests"
"${VENV_DIR}/bin/python" -m pytest "${PYTHON_PKG}/tests" -q

echo ""
echo "Python binding test gate PASSED."
