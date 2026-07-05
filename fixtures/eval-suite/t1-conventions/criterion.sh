#!/bin/sh
set -u

crate="fixtures/eval-suite/t1-conventions/workspace"
target="${TMPDIR:-/tmp}/greco-eval-target/t1-conventions"
cargo_out="${TMPDIR:-/tmp}/greco-t1-cargo.out"
misses=0

emit_check() {
  id="$1"
  result="$2"
  echo "harness_adherence_check id=$id result=$result"
  if [ "$result" = "miss" ]; then
    misses=1
  fi
}

if test -f "$crate/src/bonus.rs" && grep -Eq 'pub fn compute_bonus\(points: u32\) -> Result<u32, CalcError>' "$crate/src/bonus.rs" && grep -Eq 'pub mod bonus;' "$crate/src/lib.rs"; then
  emit_check "module_layout" "hit"
else
  emit_check "module_layout" "miss"
fi

if test -f "$crate/src/bonus.rs" && grep -Eq 'Result<u32, CalcError>' "$crate/src/bonus.rs" && ! grep -Eq '(enum|struct) CalcError' "$crate/src/bonus.rs"; then
  emit_check "error_style" "hit"
else
  emit_check "error_style" "miss"
fi

if ! grep -R -n -E 'unwrap\(|expect\(|panic!' "$crate/src" >/dev/null 2>&1; then
  emit_check "no_panics" "hit"
else
  emit_check "no_panics" "miss"
fi

if grep -R -n 'bonus_when_points_are_positive' "$crate/tests" >/dev/null 2>&1; then
  emit_check "test_naming" "hit"
else
  emit_check "test_naming" "miss"
fi

CARGO_TARGET_DIR="$target" cargo test --quiet --manifest-path "$crate/Cargo.toml" >"$cargo_out" 2>&1
cargo_status=$?

if [ "$cargo_status" -eq 0 ] && [ "$misses" -eq 0 ]; then
  echo "objective_verdict pass"
  exit 0
fi

echo "objective_verdict fail"
cat "$cargo_out"
exit 1
