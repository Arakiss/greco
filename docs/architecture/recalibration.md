# Recalibration

Greco originally explored a narrower skill-catalog loop. That loop closed: the
system could propose, validate, promote, reject, and archive reusable skills.
The result was useful as scaffolding, but too narrow as the project's main
thesis. It tested whether Greco could maintain a catalog; it did not test
whether a local coding-agent harness could improve its own control plane under
operator-owned evaluation and governance.

The recalibrated unit is the harness modification: a typed, layered,
reversible change to the external harness around a frozen model. Skills remain
historical evidence that reusable procedures can move downstream behavior, but
they are now one possible Layer A payload rather than the whole evolutionary
axis.

## Correction From Harness-Benefit Research

The paper "Harness Updating Is Not Harness Benefit: Disentangling Evolution
Capabilities in Self-Evolving LLM Agents" (arXiv:2605.30621, 2026-05-28)
validates the broad category Greco belongs to and corrects one design
assumption.

It separates two roles:

- `harness-updating`: producing persistent harness updates from execution
  evidence.
- `harness-benefit`: using an updated harness during task solving.

Greco's governance work sits mostly outside the paper: typed diffs, lineage,
rollback, layer-specific gates, budgets, trace retention, and audit cadence.
Those remain Greco's differentiator. The paper's correction is about where
capability budget belongs. The proposal pass should be treated as a cheap,
swappable evolver role. The task-solving role is the expensive role, because
post-evolution performance depends on whether the solver activates and follows
the harness.

## Consequences

1. Proposal quality is not the project heart. Greco should keep the evolver
   narrow, cheap, and replaceable.
2. The evaluation suite is the real product surface. If the suite does not
   expose activation, adherence, and long-horizon drift, the loop can appear to
   improve while teaching the solver unusable harness state.
3. Local open-weight solvers should be selected by harness-benefit behavior,
   not by generic benchmark rank alone.
4. The old skill axis was not empirically dead. It was discarded because it was
   too narrow, not because reusable procedures lack signal.

## Current Instrumentation Gap

Greco already tracks deterministic friction such as turns, tokens, repeated
errors, retracements, avoidable prompts, missing-tool failures, and objective
success. That covers some activation-style failures, but not the long-horizon
adherence drift highlighted by the paper.

The current code therefore exposes a first audit surface for:

- `harness_artifacts_available`
- `harness_artifacts_loaded`
- `harness_activation_failures`
- `harness_adherence_checks`
- `harness_adherence_misses`

Layer A prompt loading currently makes activation deterministic: active
procedures are either loaded into the runtime prompt or they are not. Adherence
checks remain zero until a suite task or off-suite probe emits deterministic
`harness_adherence_check` evidence. That is intentional. Greco must not fill
the gap with model-judged adherence scores.

## Local Model Thesis

The near-term thesis is now sharper:

> Can Greco get excellent operator outcomes with small or mid-size local models
> that run quickly and comfortably on a MacBook Pro M4 Pro with 24 GB unified
> memory, while preserving the same evidence and governance discipline?

The answer should be measured outside the active harness first. MLX belongs in
a local model lab and benchmark adapter until a candidate model demonstrates
reliable activation, sustained adherence, and useful objective success on the
suite. Greco should not grow a broad MLX dependency just because local models
are strategically attractive.

