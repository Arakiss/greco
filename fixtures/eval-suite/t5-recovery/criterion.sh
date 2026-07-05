#!/bin/sh
set -u

crate="fixtures/eval-suite/t5-recovery/workspace"
target="${TMPDIR:-/tmp}/greco-eval-target/t5-recovery"
out="${TMPDIR:-/tmp}/greco-t5-cargo.out"

CARGO_TARGET_DIR="$target" cargo test --quiet --manifest-path "$crate/Cargo.toml" >"$out" 2>&1
status=$?

if [ "$status" -eq 0 ]; then
  echo "objective_verdict pass"
  exit 0
fi

echo "objective_verdict fail"
cat "$out"
exit 1
