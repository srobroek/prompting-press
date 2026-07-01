#!/usr/bin/env bash
# Compile + run every Rust doc-sample program. A sample that fails to compile
# (drift from the live API) or whose in-program assertions panic fails the gate
# citing the example name. Run from this directory.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

cargo build --examples --quiet

fail=0
for src in examples/*.rs; do
  name="$(basename "$src" .rs)"
  if ! cargo run --quiet --example "$name" >/dev/null; then
    echo "FAILED: examples/${name}.rs" >&2
    fail=1
  fi
done

if [[ "$fail" -ne 0 ]]; then
  echo "One or more Rust doc-sample examples failed." >&2
  exit 1
fi
echo "All Rust doc-sample examples compiled + ran green."
