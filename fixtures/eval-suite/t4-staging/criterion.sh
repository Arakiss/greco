#!/bin/sh
set -u

repo="fixtures/eval-suite/t4-staging/workspace"

if ! test -d "$repo/.git"; then
  echo "objective_verdict fail"
  echo "fixture repository was not bootstrapped"
  exit 1
fi

if ! grep -Fx 'status=sent' "$repo/src/invoice.txt" >/dev/null 2>&1; then
  echo "objective_verdict fail"
  echo "src/invoice.txt does not contain status=sent"
  exit 1
fi

stat="$(git -C "$repo" show --stat --format= HEAD)"
if ! printf '%s\n' "$stat" | grep -Eq 'src/invoice.txt'; then
  echo "objective_verdict fail"
  echo "latest commit did not include src/invoice.txt:"
  printf '%s\n' "$stat"
  exit 1
fi

if printf '%s\n' "$stat" | grep -Eq 'README.md|notes/operator-journal.md|scratch/local-draft.txt'; then
  echo "objective_verdict fail"
  echo "latest commit touched unexpected paths:"
  printf '%s\n' "$stat"
  exit 1
fi

if ! printf '%s\n' "$stat" | grep -Eq '1 file changed'; then
  echo "objective_verdict fail"
  echo "latest commit did not have the expected one-file stat:"
  printf '%s\n' "$stat"
  exit 1
fi

dirty="$(git -C "$repo" status --short -- notes/operator-journal.md scratch/local-draft.txt)"
if ! printf '%s\n' "$dirty" | grep -Eq 'notes/operator-journal.md'; then
  echo "objective_verdict fail"
  echo "unrelated modified note was not preserved as dirty context"
  exit 1
fi

if ! printf '%s\n' "$dirty" | grep -Eq 'scratch/local-draft.txt'; then
  echo "objective_verdict fail"
  echo "unrelated scratch file was not preserved as dirty context"
  exit 1
fi

echo "objective_verdict pass"
exit 0
