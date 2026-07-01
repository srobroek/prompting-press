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

# Resolve cargo-about's ABSOLUTE path via mise. Why not just `cargo about`
# (subcommand form)? It needs cargo-about on PATH, but mise installs it into its
# own tool dir (NOT ~/.cargo/bin like cargo-deny) and moon does not propagate
# mise's shim PATH into task subprocesses — so a bare `cargo about` fails with
# "no such command: about" under `moon run`. We invoke the concrete binary path
# instead, which works under moon, `mise exec`, and standalone.
#
# CRUCIAL: `mise which` only QUERIES — it does NOT install a missing tool. On a
# fresh CI runner cargo-about isn't built yet (the pr-gate job installs tools
# lazily via `mise exec -- moon run`, and moon's earlier gates never invoked
# cargo-about), so `mise which` returns empty there. `mise install` first (it is
# idempotent / a fast no-op once present) so the path always resolves. Fall back
# to a PATH lookup for non-mise environments.
if command -v mise >/dev/null 2>&1; then
  mise install "cargo:cargo-about" >/dev/null 2>&1 || true
fi
CARGO_ABOUT="$(mise which cargo-about 2>/dev/null || command -v cargo-about || true)"
if [ -z "${CARGO_ABOUT}" ]; then
  echo "ERROR: cargo-about not found (mise install + which failed, not on PATH)." >&2
  echo "Install it: mise install 'cargo:cargo-about'" >&2
  exit 1
fi

# artifact-crate:output-path pairs — the two bundled bindings.
generate() {
  local crate="$1" out="$2"
  echo "  ${crate} -> ${out}"
  # --offline: crawl ONLY local crate sources for license info — no clearlydefined.io
  # lookups. This makes the output DETERMINISTIC (network state can't change it), so
  # the ci:check-third-party-licenses freshness diff is stable across machines/CI.
  # Requires crate sources in the cargo cache; CI runs `cargo fetch --locked` first.
  "${CARGO_ABOUT}" generate \
    --offline \
    -c "${CONFIG}" \
    "${TEMPLATE}" \
    --manifest-path "${REPO_ROOT}/crates/${crate}/Cargo.toml" \
    -o "${REPO_ROOT}/${out}"
  # Normalize the trailing newlines to EXACTLY ONE, matching what the repo's
  # end-of-file-fixer pre-commit hook enforces. cargo-about's Handlebars output
  # ends with a blank line ("...\n\n"); the hook collapses that to a single "\n".
  # If gen leaves the doubled newline, the committed (hook-normalized) file and a
  # fresh regen differ by one blank line and the ci:check-third-party-licenses
  # freshness diff fails forever. Strip all trailing newlines, then re-add one.
  # perl -0777 slurps the whole file; s/\n+\z/\n/ replaces the final run of
  # newlines with a single one (portable, no in-place-sed newline quirks).
  local abs="${REPO_ROOT}/${out}"
  perl -0777 -i -pe 's/\n+\z/\n/' "${abs}"
}

echo "Generating third-party license attribution (cargo-about)..."
generate "prompting-press-py"   "packages/python/THIRD-PARTY-LICENSES.md"
generate "prompting-press-node" "packages/typescript/THIRD-PARTY-LICENSES.md"

echo ""
echo "Third-party license files regenerated."
