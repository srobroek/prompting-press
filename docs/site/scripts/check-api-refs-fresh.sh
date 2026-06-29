#!/usr/bin/env bash
# T014 — API-ref freshness gate (SC-003 / spec 011).
#
# Mirrors schemas/scripts/codegen-check.sh in structure and style.
#
# Regenerates the three language API reference pages by running
# gen-api-refs.mjs, then asserts the working tree is byte-identical to what
# is committed — i.e. that regeneration produced NO change.
#
# Run twice for determinism (SC-003): if the two generations differ the page
# renderer is not stable and the gate fails naming the drifted page.
#
# The orchestrator's doc:null throw (FR-008) also fails here — a public symbol
# with no doc comment causes gen-api-refs.mjs to exit non-zero.
#
# Exits 0 if all three pages are committed, up-to-date, and deterministic.
# Exits 1 with a message naming every drifted / missing page otherwise.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SITE_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${SITE_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

RS="docs/site/src/content/docs/reference/rust.mdx"
PY="docs/site/src/content/docs/reference/python.mdx"
TS="docs/site/src/content/docs/reference/typescript.mdx"

# ---------------------------------------------------------------------------
# Step 1: Assert every generated file EXISTS before checking for drift.
# (Mirrors codegen-check.sh — two distinct "missing" cases handled.)
# ---------------------------------------------------------------------------
FAILED=()
DELETED_TRACKED="$(git ls-files --deleted -- "${RS}" "${PY}" "${TS}" 2>/dev/null || true)"
for f in "${RS}" "${PY}" "${TS}"; do
  if [[ ! -f "${f}" ]]; then
    FAILED+=("${f} (file missing on disk — not regenerated or accidentally deleted)")
  elif printf '%s\n' "${DELETED_TRACKED}" | grep -qxF "${f}"; then
    FAILED+=("${f} (tracked file recorded as deleted in git)")
  fi
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo ""
  echo "ERROR: check-api-refs-fresh FAILED — the following generated files are MISSING:"
  for msg in "${FAILED[@]}"; do
    echo "  - ${msg}"
  done
  echo ""
  echo "Run: node docs/site/scripts/gen-api-refs.mjs"
  echo "Then commit the updated generated files."
  echo ""
  exit 1
fi

# ---------------------------------------------------------------------------
# Step 2: First regeneration pass — fail loudly on extractor/render errors
# (includes doc:null FR-008 violations).
# ---------------------------------------------------------------------------
echo "[check-api-refs-fresh] First regeneration pass…"
node "${SCRIPT_DIR}/gen-api-refs.mjs"

# Register any newly-created (untracked) files so git diff can see them.
git add -N "${RS}" "${PY}" "${TS}" 2>/dev/null || true

# ---------------------------------------------------------------------------
# Step 3: Drift check after first pass.
# ---------------------------------------------------------------------------
FAILED=()
for f in "${RS}" "${PY}" "${TS}"; do
  if ! git diff --exit-code -- "${f}" > /dev/null 2>&1; then
    FAILED+=("${f}")
  fi
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo ""
  echo "ERROR: check-api-refs-fresh FAILED — the following pages drifted from committed:"
  for f in "${FAILED[@]}"; do
    echo "  - ${f}"
  done
  echo ""
  echo "The source doc comments changed but the reference pages were not regenerated."
  echo "Run: node docs/site/scripts/gen-api-refs.mjs"
  echo "Then commit the updated reference pages."
  echo ""
  git diff -- "${RS}" "${PY}" "${TS}" || true
  exit 1
fi

# ---------------------------------------------------------------------------
# Step 4: Second regeneration pass — determinism check (SC-003).
# Run again; if any page changes between pass 1 and pass 2 the renderer is
# not stable.
# ---------------------------------------------------------------------------
echo "[check-api-refs-fresh] Second regeneration pass (determinism check)…"
node "${SCRIPT_DIR}/gen-api-refs.mjs"

FAILED=()
for f in "${RS}" "${PY}" "${TS}"; do
  if ! git diff --exit-code -- "${f}" > /dev/null 2>&1; then
    FAILED+=("${f}")
  fi
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo ""
  echo "ERROR: check-api-refs-fresh FAILED — pages are NOT deterministic (differ between runs):"
  for f in "${FAILED[@]}"; do
    echo "  - ${f}"
  done
  echo ""
  echo "The renderer produced different output on two consecutive runs."
  echo "This is a bug in the extractor or renderer — please investigate."
  echo ""
  git diff -- "${RS}" "${PY}" "${TS}" || true
  exit 1
fi

echo ""
echo "check-api-refs-fresh PASSED — all three reference pages are up-to-date and deterministic."
echo "  ${RS}"
echo "  ${PY}"
echo "  ${TS}"
