#!/usr/bin/env bash
# License-policy CI gate — Node (Apache-2.0 release compatibility).
#
# Thin wrapper: ensures the TypeScript workspace deps are installed, then runs the
# offline license scanner (scripts/ci/check-licenses-node.mjs) which asserts every
# installed package's license is permissive / Apache-2.0-compatible. Mirrors
# ci:check-licenses (Rust) and ci:check-licenses-py (Python).
#
# Runtime: node + pnpm (mise-pinned). Cheap: no native addon build — reads the
# installed package.json metadata. See the .mjs header for why we do NOT use
# `pnpm licenses list` (offline-store index fragility).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

TS_PKG="${REPO_ROOT}/packages/typescript"

# Ensure deps are present (idempotent; --frozen-lockfile is hermetic/CI-safe).
# The CI job may already have installed; this makes the gate self-contained locally.
if [ ! -d "${TS_PKG}/node_modules/.pnpm" ]; then
  echo "Installing TypeScript workspace deps (pnpm --frozen-lockfile)..."
  pnpm -C "${TS_PKG}" install --frozen-lockfile
fi

node "${SCRIPT_DIR}/check-licenses-node.mjs"
