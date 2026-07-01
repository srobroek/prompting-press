#!/usr/bin/env bash
# License-policy CI gate — Python (Apache-2.0 release compatibility).
#
# Asserts that the Python wheel's RUNTIME dependency tree (pydantic and its
# transitive deps, locked in packages/python/uv.lock) carries only permissive,
# Apache-2.0-compatible licenses. Mirrors ci:check-licenses (Rust/cargo-deny) and
# ci:check-licenses-node (Node) — same policy, native tool per ecosystem.
#
# Scope is the wheel's RUNTIME deps only (`--no-dev`): the codegen/test dep-groups
# (datamodel-code-generator, ruff, hypothesis) never ship in the wheel, so their
# licenses do not affect the released artifact (hypothesis in particular is MPL-2.0
# — weak copyleft — and is deliberately out of scope here). The bundled Rust
# extension's licenses are covered by ci:check-licenses + the wheel's
# THIRD-PARTY-LICENSES.md (ci:check-third-party-licenses), not here.
#
# Tool: pip-licenses (pinned via `uv run --with pip-licenses==5.5.5` — exact
#       version, invisible to the floating-version gate; 5.5.x reads the PEP 639
#       `License-Expression` metadata field that pydantic/typing-extensions use).
# Runtime: uv (mise-pinned). Cheap: no Rust build; installs only the pure-Python
# runtime deps into a throwaway project venv, then reads their dist metadata.
#
# MAINTAINER NOTES:
#   - `--allow-only` fails (exit 1) if ANY package reports a license outside the
#     list. The list carries BOTH SPDX ids (MIT, Apache-2.0, PSF-2.0, from the
#     PEP 639 License-Expression field) AND classic Trove-classifier spellings
#     (MIT License, Apache Software License, …) because package metadata is
#     inconsistent about which it uses. Add a spelling only for a license that is
#     genuinely Apache-2.0-compatible; never add a copyleft spelling to pass CI.
#   - A new runtime dep with a disallowed license fails here — triage it (replace
#     the dep, or add the license with a written rationale if compatible).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

PYTHON_PKG="${REPO_ROOT}/packages/python"
VENV_PY="${PYTHON_PKG}/.venv/bin/python"

# Permissive, Apache-2.0-compatible licenses — SPDX + Trove-classifier spellings.
ALLOW="MIT License;MIT;\
Apache Software License;Apache License 2.0;Apache-2.0;\
BSD License;BSD-3-Clause;BSD-2-Clause;\
ISC License (ISCL);ISC;\
Python Software Foundation License;PSF-2.0;\
The Unlicense (Unlicense);Unlicense"

echo "License gate (Python): scanning wheel runtime deps in uv.lock..."
echo "  Lockfile: ${PYTHON_PKG}/uv.lock (runtime only; --no-dev)"
echo ""

# Materialise ONLY the runtime deps into the project venv (no dev groups, no
# local project build). --frozen enforces lockfile integrity (CI-safe).
uv sync --project "${PYTHON_PKG}" --no-dev --no-install-project --frozen

echo ""
# pip-licenses inspects the synced venv and fails on any non-allowed license.
uv run --project "${PYTHON_PKG}" --no-project --with pip-licenses==5.5.5 -- \
  pip-licenses --python "${VENV_PY}" --from=mixed --allow-only="${ALLOW}"

echo ""
echo "License gate (Python) PASSED — all wheel runtime deps are Apache-2.0-compatible."
