#!/bin/sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

rm -rf "$repo_dir/.git"
git -C "$repo_dir" init -q
git -C "$repo_dir" config user.name "Greco Fixture"
git -C "$repo_dir" config user.email "greco-fixture@example.invalid"

git -C "$repo_dir" add README.md src/invoice.txt notes/operator-journal.md
git -C "$repo_dir" commit -q -m "seed staging fixture"

printf '%s\n' "operator note: leave this local change alone" >> "$repo_dir/notes/operator-journal.md"
printf '%s\n' "local scratch draft; never commit this" > "$repo_dir/scratch/local-draft.txt"
