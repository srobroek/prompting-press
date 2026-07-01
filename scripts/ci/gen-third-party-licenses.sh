#!/usr/bin/env bash
# Third-party license attribution GENERATOR (Apache-2.0 release compliance).
#
# Regenerates the bundled THIRD-PARTY-LICENSES.md files for the two native
# artifacts that statically link the Rust dependency graph:
#
#   packages/python/THIRD-PARTY-LICENSES.md      <- prompting-press-py    graph
#   packages/typescript/THIRD-PARTY-LICENSES.md  <- prompting-press-node  graph
#
# These files reproduce the upstream copyright + license notices that MIT / BSD /
# ISC / Apache-2.0 require to be preserved in BINARY distributions (the wheel and
# the .node addon bundle the compiled Rust code). This script WRITES the files;
# the ci:check-third-party-licenses gate runs it and asserts `git diff` is clean.
#
# Tool: cargo-about (pinned in mise.toml under "cargo:cargo-about").
# Config: about.toml at the repo root; template: ci/about.hbs.
#         about.toml's `accepted` list MUST match deny.toml's [licenses].allow.
#
# MAINTAINER NOTES:
#   - cargo-about harvests license TEXT from each crate's local source (the
#     Cargo registry cache); it optionally queries clearlydefined.io to fill
#     gaps. Our graph resolves fully from local sources, so the clearlydefined
#     WARN lines are harmless and the output is deterministic offline.
#   - Regenerate after ANY change to Cargo.lock, about.toml, or ci/about.hbs,
#     then commit the updated THIRD-PARTY-LICENSES.md files.
#   - A NEW bundled crate under a license absent from about.toml's `accepted`
#     will surface here (and fail ci:check-licenses first); triage per deny.toml.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

CONFIG="${REPO_ROOT}/about.toml"
TEMPLATE="${REPO_ROOT}/ci/about.hbs"

# artifact-crate:output-path pairs — the two bundled bindings.
generate() {
  local crate="$1" out="$2"
  echo "  ${crate} -> ${out}"
  # --offline: crawl ONLY local crate sources for license info — no clearlydefined.io
  # lookups. This makes the output DETERMINISTIC (network state can't change it), so
  # the ci:check-third-party-licenses freshness diff is stable across machines/CI.
  # Requires crate sources in the cargo cache; CI runs `cargo fetch --locked` first.
  cargo about generate \
    --offline \
    -c "${CONFIG}" \
    "${TEMPLATE}" \
    --manifest-path "${REPO_ROOT}/crates/${crate}/Cargo.toml" \
    -o "${REPO_ROOT}/${out}"
}

echo "Generating third-party license attribution (cargo-about)..."
generate "prompting-press-py"   "packages/python/THIRD-PARTY-LICENSES.md"
generate "prompting-press-node" "packages/typescript/THIRD-PARTY-LICENSES.md"

echo ""
echo "Third-party license files regenerated."
