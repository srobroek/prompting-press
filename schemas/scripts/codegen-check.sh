#!/usr/bin/env bash
# T027 — Determinism verification gate (SC-003).
#
# Called AFTER :codegen has already run (moon deps). Asserts that the three
# generated files are byte-identical to what is committed — i.e. that
# regeneration produced NO change. Uses `git add -N` so newly-created but
# untracked files (partial regen) are also caught as a diff.
#
# Exits 0 if the working tree is clean over the three generated paths.
# Exits 1 with a message naming every drifted file otherwise.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

RS="crates/prompting-press/src/generated/prompt_definition.rs"
PY="packages/python/python/prompting_press/generated/prompt_definition.py"
TS="packages/typescript/src/generated/prompt-definition.ts"

# Register any newly-created (untracked) files so git diff can see them.
git add -N "${RS}" "${PY}" "${TS}" 2>/dev/null || true

FAILED=()
for f in "${RS}" "${PY}" "${TS}"; do
  if ! git diff --exit-code -- "${f}" > /dev/null 2>&1; then
    FAILED+=("${f}")
  fi
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo ""
  echo "ERROR: codegen-check FAILED — the following generated files drifted:"
  for f in "${FAILED[@]}"; do
    echo "  - ${f}"
  done
  echo ""
  echo "The schema changed but the generated artifacts were not regenerated."
  echo "Run: mise exec -- moon run :codegen"
  echo "Then commit the updated generated files."
  echo ""
  git diff -- "${RS}" "${PY}" "${TS}" || true
  exit 1
fi

echo "codegen-check PASSED — all three generated files are up-to-date."
echo "  ${RS}"
echo "  ${PY}"
echo "  ${TS}"
