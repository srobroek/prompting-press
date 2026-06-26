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

RS="crates/prompting-press-core/src/generated/prompt_definition.rs"
PY="packages/python/python/prompting_press/generated/prompt_definition.py"
TS="packages/typescript/src/generated/prompt-definition.ts"

# Assert every generated file EXISTS before checking for drift.
#
# Two distinct "missing" cases, both caught here:
#  (a) Missing on disk RIGHT NOW. `git diff --exit-code` is blind to a
#      deleted-but-still-indexed file (unstaged deletion → exit 0), so a plain
#      diff would fake-pass. An explicit `[[ -f ]]` catches it.
#  (b) Tracked-but-deleted vs HEAD even if :codegen later recreated it. NOTE:
#      this script runs AFTER moon's `:codegen` dep, which regenerates the three
#      files from the schema — so a file deleted before the run is normally
#      back on disk by now, and case (a) alone would NOT fire. We therefore ALSO
#      ask git whether any generated path is recorded as deleted relative to the
#      index/HEAD (`git ls-files --deleted`), which is meaningful regardless of
#      whether :codegen rematerialized identical bytes. Net: the gate fails on a
#      genuinely-removed artifact, and passes only when all three are present
#      (on disk and in git) AND byte-identical to commit.
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
  echo "ERROR: codegen-check FAILED — the following generated files are MISSING:"
  for msg in "${FAILED[@]}"; do
    echo "  - ${msg}"
  done
  echo ""
  echo "Run: mise exec -- moon run :codegen"
  echo "Then commit the updated generated files."
  echo ""
  exit 1
fi

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
