# Greco WS1 Eval Fixtures

This directory is the committed source for the movable Phase 3 evaluation
suite. The live `.greco/eval/` directory remains local runtime state; the eval
loader prefers this committed suite when it is present and falls back to
`.greco/eval/` for older local homes and tests.

Each task keeps the current Greco `task.json` shape and a deterministic
`criterion.sh` script. Criteria use only local files, local Cargo builds, grep,
and Git state created inside task fixtures.
