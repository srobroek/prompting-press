#!/usr/bin/env bash
# Deterministic Python shape codegen: prompt-definition.schema.json -> Pydantic v2.
#
# Single source of the EXACT datamodel-code-generator invocation (T020). The US4
# freshness gate (T026) re-runs this and asserts the working tree is unchanged;
# CI installs the tool hermetically from the hash-pinned uv.lock first:
#   (cd packages/python && uv sync --group codegen --no-install-project --frozen)
#
# Determinism contract (research D1): the flags below make output BYTE-STABLE
# across machines and tool upgrades. Do not add/remove flags without re-proving
# the twice-run byte-identical check.
set -euo pipefail

# Resolve repo root from this script's location (packages/python/scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
cd "${REPO_ROOT}"

SCHEMA="schemas/jsonschema/prompt-definition.schema.json"
OUT="packages/python/python/prompting_press/generated/prompt_definition.py"

HEADER='# GENERATED FILE — DO NOT EDIT.
#
# This module is code-generated from the single source of truth:
#   schemas/jsonschema/prompt-definition.schema.json
# by datamodel-code-generator (pinned via mise / packages/python uv.lock).
#
# Regenerate with: packages/python/scripts/codegen.sh  (re-run on schema change).
# Hand edits are overwritten and will fail the US4 freshness gate. Edit the schema.'

mise exec -- datamodel-codegen \
  --input "${SCHEMA}" \
  --input-file-type jsonschema \
  --output-model-type pydantic_v2.BaseModel \
  --target-python-version 3.10 \
  --use-annotated \
  --disable-future-imports \
  --disable-timestamp \
  --formatters builtin \
  --custom-file-header "${HEADER}" \
  --output "${OUT}"

echo "Generated ${OUT}"
