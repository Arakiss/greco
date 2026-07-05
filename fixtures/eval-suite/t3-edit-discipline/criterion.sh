#!/bin/sh
set -u

crate="fixtures/eval-suite/t3-edit-discipline/workspace"
target="${TMPDIR:-/tmp}/greco-eval-target/t3-edit-discipline"
out="${TMPDIR:-/tmp}/greco-t3-cargo.out"

CARGO_TARGET_DIR="$target" cargo test --quiet --manifest-path "$crate/Cargo.toml" >"$out" 2>&1
status=$?

if [ "$status" -eq 0 ]; then
  echo "objective_verdict pass"
  exit 0
fi

echo "objective_verdict fail"
cat "$out"
exit 1
