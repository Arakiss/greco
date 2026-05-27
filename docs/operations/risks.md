# Risks and Mitigations

This document covers the risks of the recalibrated v0 axis (harness self-improvement). Risks specific to the alpha skill cycle are kept at the bottom for historical reference.

## Overfitting to the suite

Risk:

The autonomous loop optimizes for the evaluation suite without producing real utility improvements on tasks outside the suite. Aggregate metrics rise while the operator's day-to-day experience stagnates or worsens.

Mitigation:

- Audit reports include suite-coverage notes and prompt the operator to test on at least one off-suite task per window.
- Suite refresh cadence: the operator is expected to rotate one or two suite tasks per quarter to dilute overfit signal.
- Pareto retention: when modifications trade off metrics, both are retained on the frontier instead of forcing a single winner. This reduces premature convergence on a narrow target.
- Off-suite probe: an optional `greco eval probe <off-suite-task>` runs a one-off task that does not feed back into the modification loop and reports its delta separately.

## Drift

Risk:

Modifications accumulate in a direction the operator would not endorse if shown the cumulative state, even though each individual modification passed thresholds.

Mitigation:

- `max_modifications_per_window` and `max_chained_modifications` cap accumulation and force a freeze for explicit operator audit.
- The audit report shows cumulative diff against a chosen baseline checkpoint, not just per-window deltas.
- `greco harness checkpoint restore <id>` returns the harness to any prior checkpoint.
- Mandatory checkpoint is created before every applied modification.

## Cascading

Risk:

One modification enables a second modification that enables a third. After a few cascades the harness reaches a state the operator would not have approved as a single step, but each individual step passed validation.

Mitigation:

- `max_chained_modifications` enforces a hard cap on stacked applications before mandatory audit.
- Lineage fields on every manifest expose the cascade chain in audit reports.
- Audit report annotates modifications that depend on prior modifications applied in the same window.

## Gaming the suite

Risk:

The autonomous loop discovers shortcuts that satisfy the suite criteria without solving the intended task. Test-shaped objectives are particularly exposed.

Mitigation:

- The suite is read-only for the system. Only the operator can edit suite files.
- Criteria are external: run a command, match a file, all-of composite. Model claims of success are ignored.
- Audit report includes anomaly heuristics: very large improvements on a single task, modifications that affect only one task, modifications that change exit codes without changing semantics.
- Off-suite probes catch suite-specific shortcuts.

## Cost explosion in validation

Risk:

Autonomous validation runs consume model and compute budget without bound.

Mitigation:

- `max_tokens_per_window` and `max_tokens_per_validation` are hard, enforced at runtime.
- `early_stop_on_first_regression` aborts validation when the first suite task regresses beyond tolerance.
- Budget consumption is reported per window in the audit. Exceeding budget freezes the loop until the next operator audit.
- Validation runs prefer executable criteria over model-judged criteria.

## Operator becomes a bottleneck

Risk:

The operator review burden grows until autonomous behavior is effectively blocked. The system reverts to manual operation and the thesis cannot be tested.

Mitigation:

- Phases 1 and 2 are operator-heavy by design (baseline, manual application). Phase 3 explicitly removes per-proposal approval.
- The audit cadence is the only mandatory operator interaction in Phase 3 onward. Default cadence: weekly. Operator can move to bi-weekly when stable.
- Layers requiring per-modification approval (E, and D until cadence passes) are kept small. The bulk of activity happens on A, B, C, S1, S2, S3 autonomously.

## Operator becomes a notary again

Risk:

Phase 4 introduces enough higher-risk modifications that the audit becomes a per-item review queue, defeating the recalibration.

Mitigation:

- Layers B, C, S2, S3 are autonomous within stricter thresholds; the audit summarizes them, it does not enumerate each.
- Layer D applies after the next cadence without veto. The operator sees the diff and skips it for any reason.
- Layer E is the only layer that requires per-modification approval. It is small by design.

## Rollback insufficient

Risk:

An applied modification produces side effects that are not captured by a checkpoint, so a "revert" leaves residue.

Mitigation:

- Modifications must be expressible as typed diffs (add tool, edit prompt, edit setting, add subagent). Free-form patches are not allowed at v0.
- Each modification declares its exact pre-state and post-state in the manifest.
- `greco harness checkpoint restore <id>` validates that post-state matches the recorded baseline after rollback.
- Workspace mutations during validation occur in temporary directories, not the live workspace.

## False friction signals

Risk:

Friction instrumentation flags noise as signal, so proposals optimize against random variation.

Mitigation:

- Phase 1 explicitly requires baseline variance characterization before any modification mechanism is wired.
- `validation_runs_required` defaults to 2 independent runs; a candidate must improve on both to apply.
- Friction signals are deterministic counters, not model-judged scores.
- Audit reports surface the variance band per metric so the operator can spot signal collapse.

## Subagent sprawl

Risk:

Layer S3 produces many specialized subagents that collectively are worse than a leaner system.

Mitigation:

- Catalog lint (Phase 5) detects near-duplicate subagent definitions and unused subagents.
- Subagent activations are reported per window in the audit; inactive subagents are flagged.
- Pareto retention applies to subagent variants: when two cover overlapping scope, both are kept on the frontier and the underused one is auto-retired after a configurable window of zero activations.

## Secret exposure

Risk:

The initial API key was pasted into an assistant transcript and is considered exposed. Local secret hygiene must be enforced.

Mitigation:

- Store credentials only in ignored local files with `0600` permissions.
- Never commit, never embed in docs, tests, fixtures, commit messages, transcripts.
- Rotate before any public release.
- Secret scan in CI gates release.

## Provider abstraction drift

Risk:

The provider trait grows speculative cross-provider semantics before a second provider exists.

Mitigation:

- One provider in v0. OpenAI-specific capabilities live inside the adapter.
- A second provider is added only after concrete integration pressure.

## Historical: alpha skill cycle risks

The following risks belonged to the v0.1-v0.3-alpha exploration of the skill axis and are retained as context. They are no longer load-bearing for the active plan but inform the audit anomalies to watch for.

- *False skill positives* — narrow validation passes that hide regression. Recalibration substitute: false friction signals + objective criteria on the suite.
- *Catalog debt* — junk drawer of near-duplicates. Recalibration substitute: catalog lint + subagent sprawl + lineage fields.
- *Scaling skill selection* — prompt stuffing and skill ranking. Recalibration substitute: the harness modification archive is operator-audited in aggregate; the model does not pick from a large catalog in real time.
- *Skill validation cost* — autonomous validation eats budget. Same risk shape under the new axis, mitigated by the budget block above.
- *Harness self-modification* (declared as risk in alpha) — now the explicit objective. The mitigation is structural: layered gates, suite read-only for the system, mandatory checkpoints, rollback discipline.
