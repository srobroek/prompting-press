#!/usr/bin/env bash
# License-policy CI gate — Rust (Apache-2.0 release compatibility).
#
# Runs `cargo deny check licenses` against the workspace Cargo.lock to assert that
# EVERY crate we ship (kernel + consumer + both FFI bindings) and all of their
# transitive deps carry an Apache-2.0-compatible license — i.e. permissive, or a
# multi-license (`OR`) expression with at least one allowed permissive arm. No
# strong copyleft (GPL/AGPL/LGPL-as-sole-license) is permitted.
#
# The allow-list lives in deny.toml [licenses].allow (deliberately minimal — only
# the SPDX ids actually present in the resolved graph). A new dep that pulls in a
# license outside that set FAILS this gate by design.
#
# Companion gates (same policy, other ecosystems):
#   ci:check-licenses-py    — Python runtime deps (pip-licenses)
#   ci:check-licenses-node  — Node deps (installed package.json scan)
# And the attribution side: ci:check-third-party-licenses (cargo-about).
#
# Tool: cargo-deny (pinned in mise.toml under "cargo:cargo-deny").
# Cheap: reads Cargo.lock + deny.toml; NO `cargo build` required. Unlike the
# advisory gate, the license check needs NO network (no advisory DB fetch).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

echo "License gate (Rust): running cargo deny check licenses..."
echo "  Config: ${REPO_ROOT}/deny.toml"
echo "  Lockfile: ${REPO_ROOT}/Cargo.lock"
echo ""

cargo deny --manifest-path "${REPO_ROOT}/Cargo.toml" check licenses

echo ""
echo "License gate (Rust) PASSED — all bundled crates are Apache-2.0-compatible."
