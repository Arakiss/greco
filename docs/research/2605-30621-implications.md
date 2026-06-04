# arXiv 2605.30621 Implications For Greco

Source: "Harness Updating Is Not Harness Benefit: Disentangling Evolution
Capabilities in Self-Evolving LLM Agents", arXiv:2605.30621v1, submitted
2026-05-28.

## What The Paper Says

The paper studies agents built from a frozen model plus an editable external
harness: prompts, skills, memories, and tools. Harness self-evolution updates
that harness from execution evidence without changing model weights.

The important move is that it decomposes the loop:

- `harness-updating` is the evolver's ability to produce useful persistent
  updates.
- `harness-benefit` is the solver's ability to benefit from those updates.

The reported findings are directly relevant to Greco:

- Harness-updating is flat across base capability. The paper reports at most a
  3.1 percentage-point gap between evolvers on any benchmark.
- Harness-benefit is non-monotonic. Weak-tier models benefit little, mid-tier
  models benefit most, and strong-tier models benefit less because they are
  closer to the ceiling.
- Weak-tier failures are mostly activation and adherence failures. Some models
  do not reliably load the relevant harness artifact; others load it but drift
  away from the procedure over long tasks.

## What Greco Already Has

Greco is in the same family but has a different objective. It is not primarily
a capability study. It is a governed engineering system:

- Typed, layered, reversible harness modifications.
- Persistent archive and lineage.
- Empirical validation before application.
- Budget caps and freeze behavior.
- Operator audit cadence rather than per-proposal approval.
- Deterministic gates over trace evidence.

Those features are not invalidated by the paper. They become more important
once local, lower-cost solvers enter the system.

## Design Corrections

The proposal pass should not be assumed to require the strongest model.
Greco's default architecture should support a cheap evolver role, eventually a
small local model or a deterministic summarizer-plus-small-model pass, while
reserving expensive capability for task solving or validation.

The solver selection criterion changes. A local model candidate should not win
because it has the best generic coding leaderboard score. It should win because
it reliably:

1. Loads the relevant harness artifact.
2. Keeps following the artifact across long trajectories.
3. Completes suite criteria with lower cost and acceptable wall time.
4. Does not regress protected safety and governance signals.

The evaluation suite must therefore include tasks that make activation and
adherence observable without model judgment.

## Deterministic Metrics To Add

Activation can be measured with counters:

- Artifact available for the task.
- Artifact loaded into the working context.
- Load request rejected or malformed.
- Artifact selected but never made visible to the solver.

Adherence is harder. Greco should prefer procedure-level deterministic anchors:

- A cached procedure declares required observable checkpoints.
- Each checkpoint maps to a trace event, command result, file existence check,
  or diff pattern.
- The audit computes adherence as observed checkpoints over expected
  checkpoints at phases such as load, middle turn, and final turn.

This avoids asking the model whether it followed instructions. The model can
emit events, but the criterion must be grounded in external evidence.

## Near-Term Implementation Position

The current implementation adds audit fields for activation and adherence. It
does not yet create model-judged adherence. In current Layer A prompt loading,
active procedures are automatically injected, so activation failures should be
zero unless active procedure loading breaks.

The next meaningful implementation step is to let selected eval tasks declare
adherence anchors. That can happen without changing the provider boundary and
without importing MLX into the harness.

