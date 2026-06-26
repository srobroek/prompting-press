#!/usr/bin/env bash
# T028 — FFI-isolation CI gate (FR-018, FR-020; constitution Principle II / C-02).
#
# Asserts that `pyo3` and `napi` do NOT appear in the dependency trees of the
# kernel/consumer crates listed in COVERED_CRATES below.
#
# Mechanism: `cargo tree -p <crate> -i <ffi-crate>` exits non-zero with
# "did not match any packages" when the FFI crate is absent. If it exits 0 the
# FFI crate has crept in and we FAIL.
#
# MAINTAINER NOTE: if you add a new crate that must stay FFI-free, add its
# package name to COVERED_CRATES. Do NOT auto-derive this list; explicit
# enumeration is the guard that prevents new crates from silently escaping the gate.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
cd "${REPO_ROOT}"

# --- Explicit, reviewable list of crates that MUST remain FFI-free. ---
# Add new FFI-free crates here when they are introduced to the workspace.
COVERED_CRATES=(
  "prompting-press-core"   # kernel — constitution Principle II / C-02
  "prompting-press"        # public Rust consumer surface — must stay FFI-free
)

# FFI toolkits that must never appear in the above crates' dep trees.
FFI_CRATES=("pyo3" "napi")

FAILED=()

# Preflight: verify every crate in COVERED_CRATES is a real workspace member.
# `cargo tree -p <nonexistent-crate>` silently falls back to the full workspace
# tree and exits 0, making the -i scoping useless and producing phantom failures.
# A typo in COVERED_CRATES must be reported as gate misconfiguration (exit 2),
# not treated as a phantom FFI dependency.
for crate in "${COVERED_CRATES[@]}"; do
  if ! cargo tree -p "${crate}" > /dev/null 2>&1; then
    echo "ERROR: gate misconfigured — unknown crate '${crate}' (not in workspace)."
    echo "Fix COVERED_CRATES in $(basename "${BASH_SOURCE[0]}") or add the crate to the workspace."
    exit 2
  fi
done

for crate in "${COVERED_CRATES[@]}"; do
  for ffi in "${FFI_CRATES[@]}"; do
    # cargo tree -i <pkg> exits 0 if the package IS in the dep tree (bad —
    # FFI found). It exits non-zero ("did not match any packages") when absent
    # (good — FFI-free). Use || true so set -e does not abort on the expected
    # non-zero exit; we disambiguate via the output text instead.
    output="$(cargo tree -p "${crate}" -i "${ffi}" 2>&1)" || true
    if echo "${output}" | grep -q "did not match any packages"; then
      # Absent — correct.
      :
    else
      # Present — violation.
      FAILED+=("${crate} depends on ${ffi}")
    fi
  done
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo ""
  echo "ERROR: FFI-isolation gate FAILED (Principle II / C-02)."
  echo "The following FFI toolkit(s) were found in crates that must remain FFI-free:"
  for msg in "${FAILED[@]}"; do
    echo "  - ${msg}"
  done
  echo ""
  echo "pyo3 and napi are permitted ONLY in crates/prompting-press-py and"
  echo "crates/prompting-press-node respectively. They must never reach the"
  echo "kernel (prompting-press-core) or the Rust consumer (prompting-press)."
  echo ""
  exit 1
fi

echo "FFI-isolation gate PASSED."
for crate in "${COVERED_CRATES[@]}"; do
  echo "  ${crate}: no pyo3, no napi in dependency tree"
done
