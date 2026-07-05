#!/bin/sh
set -u

root="fixtures/eval-suite/t2-read-economy/workspace"
settings="$root/src/settings.rs"

if grep -Fx 'pub const ACTIVE_LIMIT: usize = 64;' "$settings" >/dev/null 2>&1; then
  echo "objective_verdict pass"
  exit 0
fi

echo "objective_verdict fail"
echo "expected ACTIVE_LIMIT to be 64 in $settings"
exit 1
