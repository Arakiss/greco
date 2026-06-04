# Local Model Lab

Greco should explore local open-weight solvers on the operator's MacBook Pro M4
Pro with 24 GB unified memory, but the lab must stay outside the active harness
until the evidence is good enough.

## Hardware Boundary

Observed local machine:

- Apple MacBook Pro, model identifier `Mac16,8`
- Apple M4 Pro
- 12 CPU cores: 8 performance, 4 efficiency
- 24 GB unified memory

This suggests the first target is not "largest model that can barely run". The
target is a model that leaves enough memory and thermal headroom for the Rust
harness, shell tools, git operations, and the operator's normal environment.

## Isolation Rule

MLX and local model runners should live in a lab adapter, not in Greco's core
crate. The core harness should continue to see the narrow provider contract:
request, response, tool calls, usage, and trace events.

Allowed early shapes:

- External command adapter that runs a local model executable and writes a
  normalized JSON response.
- Separate benchmark script that replays Greco eval tasks against candidate
  models.
- Optional git-ignored config in an operator-local directory.

Not allowed yet:

- Adding MLX as a runtime dependency of the core Greco crate.
- Expanding the provider trait for speculative local-model features.
- Changing the active autonomous loop based on unvalidated local-model output.

## Candidate Selection

A candidate model should be evaluated in three bands.

1. Capability: objective success on the existing suite.
2. Harness-benefit: activation rate and adherence over long tasks with active
   procedures.
3. Operator cost: wall time, memory pressure, startup latency, and thermal
   comfort on the laptop.

The winner is the model with the best useful harness-benefit under local
constraints, not necessarily the strongest benchmark model.

## Suggested Matrix

The first lab matrix should include:

- Baseline remote solver: current OpenAI model, to preserve a reference point.
- Small local candidates that fit comfortably.
- Mid-size local candidates that still leave headroom.
- One intentionally-too-large candidate if useful, only to measure the cliff.

Each candidate should run the same suite with:

- no active harness artifact,
- the current active Layer A procedures,
- an adherence-probe version of the task with deterministic checkpoints.

## Acceptance Rule

A local solver can be considered for Greco integration only when it demonstrates:

- non-zero objective success on representative suite tasks,
- no activation failures on tasks with declared relevant artifacts,
- adherence misses below a declared threshold,
- stable wall time and memory use across repeated runs,
- no weakening of Greco's release and secret-handling gates.

