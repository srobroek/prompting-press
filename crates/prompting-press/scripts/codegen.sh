#!/usr/bin/env bash
# Deterministic Rust shape codegen: prompt-definition.schema.json -> serde structs.
#
# Single source of the EXACT cargo-typify invocation (T025). The US4 freshness
# gate (T026) re-runs this and asserts the working tree is unchanged. The tool is
# hash-pinned via mise ("cargo:cargo-typify" = "0.7.0"); rustfmt is the pinned
# rustup toolchain component (rust = "1.95.0"). Both are invoked under `mise exec`.
#
# Determinism contract (research D1 / US4): cargo-typify emits NO timestamp, NO
# version stamp, and a stable (sorted) field/variant order. The header below is
# STATIC — no version, no timestamp, no host data — so the committed file stays
# BYTE-STABLE across machines and repeated runs. Do not add anything time- or
# host-varying. The rustfmt pass is a determinism guard against future typify
# formatting drift (verified a no-op on this schema in the T022 spike).
#
# WORKAROUND (T022 Finding F1): cargo-typify 0.7.0 PANICS on the schema as-is
# because `properties.variants.propertyNames = { "not": { "const": "default" } }`
# hits an unimplemented `not`-subschema path in typify. The schema is the
# cross-language single source of truth and MUST NOT change. So this script
# generates from a TRANSFORMED COPY with ONLY that one validation key stripped
# (`jq 'del(.properties.variants.propertyNames)'`). This is sound: `propertyNames`
# is a VALIDATION constraint that no generated type in ANY language can encode
# ("map key must not equal 'default'"); the reserved-`default` rule (FR-011b) is
# enforced by the US2 validation gate (the `variant-named-default.json` reject
# fixture), NOT by the Rust type. The strip affects ONLY the bytes handed to
# typify; the on-disk schema is never modified.
set -euo pipefail

# Resolve repo root from this script's location (crates/prompting-press/scripts/).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
cd "${REPO_ROOT}"

SCHEMA="schemas/jsonschema/prompt-definition.schema.json"
OUT="crates/prompting-press/src/generated/prompt_definition.rs"

# Static do-not-edit header. cargo-typify emits crate-level `#![allow(clippy::...)]`
# INNER attributes that MUST be the first non-comment items in the file, so the
# header is written as leading `//` line comments (comments are permitted before
# inner attributes; `//!` doc comments are NOT, as they are themselves inner items
# that would have to follow the `#![...]`). This keeps the generated file a valid
# module file (`mod generated; mod prompt_definition;`), never `include!`d.
HEADER='// GENERATED FILE — DO NOT EDIT.
//
// This module is code-generated from the single source of truth:
//   schemas/jsonschema/prompt-definition.schema.json
// by cargo-typify (pinned via mise: "cargo:cargo-typify" = "0.7.0").
//
// Regenerate with: crates/prompting-press/scripts/codegen.sh  (re-run on schema change).
// Hand edits are overwritten and will fail the US4 freshness gate. Edit the schema.
//
// NOTE: the schema'\''s `variants.propertyNames` (reserved-"default" rejection,
// FR-011b) is a VALIDATION constraint with no representable Rust type; it is
// stripped before typify (which cannot parse its `not`/`const` form) and is
// enforced by the US2 validation gate, not by the types below. See codegen.sh.'

# Scratch copy of the schema with ONLY the typify-incompatible validation key
# removed. The on-disk schema is the source of truth and is left untouched.
TMP_SCHEMA="$(mktemp -t pp-typify-schema.XXXXXX.json)"
trap 'rm -f "${TMP_SCHEMA}"' EXIT

mise exec -- jq 'del(.properties.variants.propertyNames)' "${SCHEMA}" > "${TMP_SCHEMA}"

# typify does not create the parent directory; ensure the segregated generated
# module dir exists (FR-016).
mkdir -p "$(dirname "${OUT}")"

# `--no-builder`: emit the plain serde types only (no builder boilerplate; 754 vs
# 1165 lines — T022 F2). typify writes the `#![allow(...)]` inner attrs + types.
mise exec -- cargo typify \
  --no-builder \
  --output "${OUT}" \
  "${TMP_SCHEMA}"

# Determinism guard: re-run rustfmt with the pinned edition (verified no-op on
# this schema in T022, kept so future typify formatting drift can't slip past
# the freshness gate).
mise exec -- rustfmt --edition 2021 "${OUT}"

# Prepend the static header. typify's output begins with the `#![allow(...)]`
# inner attributes; `//` line comments are legal before them, so the header goes
# at the very top of the file.
TMP_OUT="$(mktemp -t pp-typify-out.XXXXXX.rs)"
trap 'rm -f "${TMP_SCHEMA}" "${TMP_OUT}"' EXIT
{
  printf '%s\n\n' "${HEADER}"
  cat "${OUT}"
} > "${TMP_OUT}"
mv "${TMP_OUT}" "${OUT}"

echo "Generated ${OUT}"
