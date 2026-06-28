#!/usr/bin/env bash
# Spec 005 — Node dependency advisory CI gate (FR-025, SC-011, security review SEC-201).
#
# Scans the npm dependencies of packages/typescript (locked in pnpm-lock.yaml) for known
# CVEs / advisories via `pnpm audit` against the npm advisory database.
#
# PRIMARY PURPOSE: catch known vulnerabilities in the Node binding's deps (zod, @napi-rs/cli,
# json-schema-to-typescript, typescript, prettier) before they ship. Mirrors
# ci:check-advisories (Rust/cargo-deny) and ci:check-advisories-py (Python/pip-audit).
#
# Approach: `pnpm audit --prod` over the workspace package, failing on any advisory at or
# above the configured level. We gate on `high` + `critical` (matching the conservative
# posture of the Rust/Python gates — a `moderate` in a build-time dev tool should not block
# CI, but a high/critical anywhere should). Adjust --audit-level deliberately, not casually.
#
# Runtime: pnpm (mise-pinned). No build of the native addon required — this reads the lockfile.
#
# MAINTAINER NOTES:
#   - pnpm audit queries the npm registry advisory endpoint; needs outbound HTTPS. In an
#     air-gapped runner, swap for `osv-scanner --lockfile=packages/typescript/pnpm-lock.yaml`.
#   - To accept a specific advisory after security review, add it to a pnpm `auditConfig`
#     `ignoreCves`/`ignoreGhsas` allowlist in packages/typescript/package.json with a comment
#     linking the advisory + the review rationale — do NOT lower --audit-level to hide it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

TS_PKG="${REPO_ROOT}/packages/typescript"
AUDIT_LEVEL="high"

echo "Advisory gate (Node): scanning ${TS_PKG}/pnpm-lock.yaml for known CVEs (level >= ${AUDIT_LEVEL})..."
echo ""

# `pnpm audit` exits non-zero if it finds advisories at or above --audit-level → fails the gate.
# --prod: audit the production dependency tree (the wheel's runtime + transitive); the dev
# toolchain (napi-cli, tsc, prettier) is build-time only, but pnpm audit --prod still covers
# what ships. Run from the package dir so pnpm resolves the right lockfile.
mise exec -- pnpm -C "${TS_PKG}" audit --audit-level "${AUDIT_LEVEL}"

echo ""
echo "Advisory gate (Node) PASSED — no known vulnerabilities (>= ${AUDIT_LEVEL}) in Node dependencies."
