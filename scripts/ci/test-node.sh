#!/usr/bin/env bash
# Spec 005 — Node binding test gate (the spec-004 review-I1 lesson, applied to the TS side).
#
# Builds the prompting-press-node addon + the TS facade and runs BOTH test suites that the
# OS-matrix `cargo build --workspace` does NOT exercise:
#   1. cargo test -p prompting-press-node  — Rust-side marshaling + SEC-004 scrub unit tests.
#   2. pnpm -C packages/typescript test    — the ONLY coverage for the Zod validate-at-render
#      path, the ZodError->rows scrub, the napi-error->Error-subclass decoder, the dual-input
#      loader parity, and composition. None of these are reachable from the Rust tests.
#
# WHY THIS GATE EXISTS: without it the Rust + TS binding test suites can rot green — a regression in
# the facade or the marshaling path would pass CI. The OS-matrix `build` job compiles the crate
# (catching link regressions) but never runs its tests.
#
# Runtime: mise-pinned cargo + pnpm + node. The `cargo test` harness needs NO live Node runtime
# (the #[cfg(test)] tests build serde_json::Value / minijinja::Value / KernelError directly, and
# napi's default dyn-symbols feature resolves N-API symbols lazily — see the node crate Cargo.toml
# rlib comment), so there is no libpython-style runtime-link dance like the Python gate has.
#
# The TS tests run via `node --test` against the BUILT addon + facade (pnpm's `pretest` builds the
# facade; this script builds the addon first so the native binary is present on the CI runner —
# the highest-probability CI surprise is the addon failing to load on Linux, so we build+run here).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

TS_PKG="${REPO_ROOT}/packages/typescript"

echo "Node binding test gate (spec 005 / review I1 lesson)"
echo ""

# Step 1 — Rust-side binding tests (marshaling + scrub; needs no Node host).
echo "==> cargo test -p prompting-press-node"
cargo test -p prompting-press-node

# Step 2 — build the native addon (so the .node loads at test time), then run the TS suites.
echo ""
echo "==> build the napi addon + run the TS test suites"
# `build` runs napi build (the addon) then tsc (the facade); `pretest` rebuilds the facade.
pnpm -C "${TS_PKG}" build
pnpm -C "${TS_PKG}" test

echo ""
echo "Node binding test gate PASSED."
