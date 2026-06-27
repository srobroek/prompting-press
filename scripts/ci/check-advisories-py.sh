#!/usr/bin/env bash
# T028 — Python dependency advisory CI gate (FR-025, SC-011, SEC-101).
#
# Scans Python dependencies locked in packages/python/uv.lock for known CVEs
# and security advisories via pip-audit + the OSV advisory database.
#
# PRIMARY PURPOSE: catch known vulnerabilities in Python workspace dependencies
# (pydantic, datamodel-code-generator, maturin build deps) before they land in
# a shipped release. Mirrors ci:check-advisories (Rust/RustSec gate).
#
# Approach:
#   1. Export the locked dependency set from packages/python/uv.lock to a
#      requirements-txt file with integrity hashes (--frozen enforces lock
#      integrity; --no-emit-project excludes the local `-e .` editable entry
#      which cannot be hashed; --all-groups includes the codegen dep-group).
#   2. Run pip-audit against the hashed requirements file with --disable-pip
#      (bypasses pip's internal venv resolver — hashes make it unnecessary)
#      to perform a pure CVE lookup against the OSV advisory database.
#
# Tool: pip-audit (pinned via `uv run --with pip-audit==2.9.0` — exact version,
#       not a manifest entry, so it is invisible to the floating-version gate).
# Runtime: uv (mise-pinned at 0.11.8) — hermetic, no system Python required.
#
# MAINTAINER NOTES:
#   - pip-audit queries the OSV advisory database at runtime; the gate requires
#     outbound HTTPS to api.osv.dev. In air-gapped environments, use
#     `--vulnerability-service pypi` or a local OSV mirror.
#   - To suppress a specific advisory (after security review), add
#     `--ignore-vuln GHSA-xxxx-xxxx-xxxx` to the pip-audit invocation below
#     with a comment explaining the rationale and a link to the advisory.
#   - This gate does NOT require building the Rust extension; it reads only
#     uv.lock via `uv export`.
#   - To upgrade pip-audit: change the pinned version in the `uv run --with`
#     flag below. The version must remain an exact pin (no ^/~/>=) so the
#     floating-version gate (ci:check-floating-versions) stays clean.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

PYTHON_PKG="${REPO_ROOT}/packages/python"
LOCKFILE="${PYTHON_PKG}/uv.lock"

echo "Advisory gate (Python): scanning ${LOCKFILE} for known CVEs..."
echo "  Lockfile: ${LOCKFILE}"
echo "  Scope: all dependency groups (default + codegen)"
echo ""

# Step 1 — export the locked requirements with hashes.
# --frozen: assert uv.lock will not be updated (CI-safe).
# --no-emit-project: omit the local `-e .` editable entry (not hashable).
# --all-groups: include the codegen dep-group (datamodel-code-generator etc.).
REQS_FILE="$(mktemp /tmp/pp-py-audit-XXXXXX.txt)"
trap 'rm -f "${REQS_FILE}"' EXIT

uv export \
  --project "${PYTHON_PKG}" \
  --format requirements-txt \
  --frozen \
  --no-emit-project \
  --all-groups \
  > "${REQS_FILE}"

echo "  Exported $(grep -c '==' "${REQS_FILE}" || true) pinned packages to ${REQS_FILE}"
echo ""

# Step 2 — audit the hashed requirements file.
# --disable-pip: skip pip's internal venv resolver; hashes make it unnecessary.
#   (Required when the requirements file contains --hash entries.)
# pip-audit==2.9.0: exact pin — not a manifest entry; invisible to SEC-003 gate.
uv run --with pip-audit==2.9.0 pip-audit \
  -r "${REQS_FILE}" \
  --disable-pip

echo ""
echo "Advisory gate (Python) PASSED — no known vulnerabilities in Python dependencies."
