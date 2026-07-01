#!/usr/bin/env bash
# Third-party license FRESHNESS gate (Apache-2.0 release compliance).
#
# Regenerates the bundled THIRD-PARTY-LICENSES.md files (via
# scripts/ci/gen-third-party-licenses.sh) and asserts `git diff` is clean. If the
# committed attribution has drifted from the current Cargo.lock (a dep was added,
# removed, or bumped without regenerating), this FAILS with the diff — the same
# determinism pattern as schemas:codegen-check.
#
# Fix a failure by running:  bash scripts/ci/gen-third-party-licenses.sh
# then committing the updated packages/*/THIRD-PARTY-LICENSES.md files.
#
# Cheap: reads Cargo.lock + local crate sources; NO `cargo build` required.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

TARGETS=(
  "packages/python/THIRD-PARTY-LICENSES.md"
  "packages/typescript/THIRD-PARTY-LICENSES.md"
)

echo "Third-party license freshness gate: regenerating and diffing..."
echo ""

bash "${SCRIPT_DIR}/gen-third-party-licenses.sh"

echo ""
if ! git diff --quiet -- "${TARGETS[@]}"; then
  echo "ERROR: bundled THIRD-PARTY-LICENSES.md is STALE — it does not match the" >&2
  echo "current Cargo.lock. Regenerate and commit:" >&2
  echo "    bash scripts/ci/gen-third-party-licenses.sh" >&2
  echo "" >&2
  echo "--- git diff ---" >&2
  git --no-pager diff -- "${TARGETS[@]}" >&2
  exit 1
fi

echo "Third-party license freshness gate PASSED — attribution is up to date."
