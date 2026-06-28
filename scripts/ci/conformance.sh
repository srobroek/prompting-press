#!/usr/bin/env bash
# Spec 006 — Conformance corpus gate (FR-012 / FR-015).
#
# Builds both FFI bindings and runs all three conformance legs — Rust consumer,
# Python, and TypeScript — in a single locally-reproducible gate. This is the
# CI-authoritative command for the spec-006 conformance corpus.
#
# Three legs:
#   1. cargo test -p prompting-press --test conformance_marshaling \
#                                    --test conformance_schema
#      — the Rust consumer conformance tests. These close the CI gap: the
#        OS-matrix `cargo build --workspace` compiles the consumer crate but
#        NEVER runs its tests, so a regression in marshaling or schema round-trip
#        would pass CI silently. `--test conformance_goldens` is intentionally
#        OMITTED — that file is #[ignore]d and is a deliberate regen tool, not a
#        gate (see `moon run conformance:regen`).
#   2. maturin develop (into hermetic uv venv) + pytest on JUST the two
#      conformance files (test_conformance_marshaling.py,
#      test_conformance_schema.py). The full Python suite is covered by
#      ci:test-python; this leg exercises only the conformance corpus paths.
#   3. pnpm -C packages/typescript build + node --test on JUST the two
#      conformance test files (conformance.marshaling.test.mjs,
#      conformance.schema.test.mjs). The full TS suite is covered by
#      ci:test-node; this leg exercises only the conformance corpus paths.
#
# WHY THIS GATE EXISTS: the Rust consumer conformance tests (leg 1) were
# unexercised in CI before spec 006. Legs 2 and 3 are scoped to the conformance
# corpus only, making this script a single, self-contained, locally-runnable
# proof that all three bindings agree on marshaling and schema round-trip.
#
# Runtime: mise-pinned cargo + uv + uvx maturin + pnpm + node.
#
# MAINTAINER NOTES:
#   - To upgrade maturin: change MATURIN_PIN below (exact pin, no ^/~/>=).
#   - The libpython LIBDIR export (belt-and-suspenders for mise/pyenv paths) is
#     copied from test-python.sh — required by the `cargo test -p
#     prompting-press-py`-linked binary in test-python; not needed here (we do
#     not run cargo test -p prompting-press-py in this gate) but kept for
#     consistency and because the maturin develop step runs under the same venv.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

MATURIN_PIN="maturin==1.14.1"
PYTHON_PKG="${REPO_ROOT}/packages/python"
TS_PKG="${REPO_ROOT}/packages/typescript"

echo "Conformance corpus gate (spec 006 / FR-012 / FR-015)"
echo ""

# ---------------------------------------------------------------------------
# Step 1 — Rust consumer conformance tests (the unique CI value of this gate).
# ---------------------------------------------------------------------------
echo "==> cargo test -p prompting-press --test conformance_marshaling --test conformance_schema"
cargo test -p prompting-press --test conformance_marshaling --test conformance_schema

# ---------------------------------------------------------------------------
# Step 2 — Python conformance: build extension into hermetic venv, run pytest
# on the two conformance files only.
# ---------------------------------------------------------------------------
echo ""
echo "==> Python conformance leg: maturin develop + pytest conformance files"

# The `cargo test` binary links libpython; export its LIBDIR so the loader
# finds libpythonX.Y.so/.dylib on mise-/pyenv-managed interpreters.
PY_LIBDIR="$(python3 -c 'import sysconfig; print(sysconfig.get_config_var("LIBDIR") or "")' 2>/dev/null || true)"
if [ -n "${PY_LIBDIR}" ]; then
  export LD_LIBRARY_PATH="${PY_LIBDIR}${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}"
  export DYLD_FALLBACK_LIBRARY_PATH="${PY_LIBDIR}${DYLD_FALLBACK_LIBRARY_PATH:+:${DYLD_FALLBACK_LIBRARY_PATH}}"
  echo "  libpython LIBDIR: ${PY_LIBDIR}"
fi

VENV_DIR="$(mktemp -d /tmp/pp-conformance-XXXXXX)"
trap 'rm -rf "${VENV_DIR}"' EXIT

echo "==> uv venv + build extension (maturin develop) into ${VENV_DIR}"
uv venv "${VENV_DIR}" --python 3.10
VIRTUAL_ENV="${VENV_DIR}" uv pip install --python "${VENV_DIR}/bin/python" pytest pydantic

# maturin develop must run from packages/python (resolves [tool.maturin] module-name).
( cd "${PYTHON_PKG}" \
  && VIRTUAL_ENV="${VENV_DIR}" uvx --from "${MATURIN_PIN}" maturin develop )

echo ""
echo "==> pytest conformance files (marshaling + schema)"
"${VENV_DIR}/bin/python" -m pytest \
  "${PYTHON_PKG}/tests/test_conformance_marshaling.py" \
  "${PYTHON_PKG}/tests/test_conformance_schema.py" \
  -q

# ---------------------------------------------------------------------------
# Step 3 — TypeScript conformance: build addon + facade, run node:test on the
# two conformance test files only.
# ---------------------------------------------------------------------------
echo ""
echo "==> TypeScript conformance leg: pnpm build + node --test conformance files"

# build runs napi build (addon) then tsc (facade); pretest rebuilds the facade.
pnpm -C "${TS_PKG}" build

echo ""
echo "==> node --test conformance files (marshaling + schema)"
( cd "${TS_PKG}" \
  && node --test \
       test/conformance.marshaling.test.mjs \
       test/conformance.schema.test.mjs )

echo ""
echo "Conformance gate PASSED."
