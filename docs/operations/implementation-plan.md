# Implementation Plan

This plan covers the recalibrated v0 axis (harness self-improvement). It supersedes the original Phase 0-5 plan that targeted the skill catalog. Phases 0-5 of the alpha cycle are recorded here as completed historical context; the active plan begins at Phase 1.

## Phase 0 (completed) — Repository foundation

Delivered in `0.1.0-alpha.1`. Secret-safe shell, ignored `.env.local`, research and design docs, atomic Lore commits.

## Phase 0.5 (completed) — Alpha skill loop

Delivered through `0.3.0-alpha.1`. Closed the original Kappa loop: structured skill proposal, candidate persistence, empirical (self-referential) validation, promotion/rejection, scores, JSONL traces, CLI lifecycle commands. This phase confirmed the loop closes but exposed the thesis weakness recorded in [`../architecture/recalibration.md`](../architecture/recalibration.md).

## Phase 1 — Instrumentation and baseline

Goal: produce reliable, low-noise friction signals from real sessions before any modification mechanism exists. If signals are too noisy to support measurement, the project closes here.

Build:

- `eval` module: suite loader, criterion runner, budget enforcer.
- Five baseline suite tasks under `.greco/eval/` covering refactor, bug fix, doc update, test addition, and search/answer. Defined by the operator on a real local project.
- `agent.rs` instrumentation: emit deterministic friction events (`turns`, `tokens`, `repeated_errors`, `retracements`, `avoidable_prompts`, `missing_tool_failures`, `objective_success`) into the existing trajectory JSONL.
- `audit` module: aggregate report renderer (markdown + JSON), `greco audit --since <window>` command.
- Operator command surface: `greco eval list`, `greco eval run <task-id>`, `greco audit --since <window>`.

Exit criteria:

- Baseline friction metrics computed for each suite task across at least 10 sessions.
- Variance of `turns_per_task` and `tokens_per_task` is small enough for delta detection at 5% to be meaningful (declared threshold; operator validates the variance band manually).
- Audit report renders correctly with at least one window of real data.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass.

Decision gate at the end of Phase 1: if signals are unusably noisy on a real project, the project closes per the RFC Appendix B clause. The recalibration document acknowledges this.

## Phase 2 — Proposal pass with manual application

Goal: validate that the proposal pass produces useful candidate modifications. The operator applies accepted proposals by hand to keep autonomous behavior off until candidate quality is verified.

Build:

- `proposal` module rewritten: friction-detection pass over recent traces, structured-output proposal of Layer A and S1 modifications (cached procedures, subagent prompt edits) only.
- `catalog` module rewritten: registry of harness modifications with states `proposed`, `validated`, `active`, `rejected`, `retired` and lineage fields described in `design.md`.
- `validation` module rewritten: run the suite with and without a candidate, compute deltas, write a validation trace.
- New commands: `greco propose [--since <window>]`, `greco modification list --state <state>`, `greco modification show <id>`, `greco modification validate <id>`, `greco modification apply <id>`, `greco modification revert <id>`.
- Subagent loader: minimum support for declared subagents in `.greco/subagents/`. The runtime invocation surface from the main agent can wait for Phase 3.

Exit criteria:

- At least 20 candidate modifications produced over the baseline window.
- Operator-rated precision of proposals (manually labelled as "useful" vs "noise" post-hoc) at or above 50%. The exact threshold is recorded in the audit; if proposals are pure noise, the project closes.
- A handful of useful proposals applied manually show measurable friction reduction on the suite. The exact number is operator judgment, not a fixed target.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` all pass.

Decision gate at the end of Phase 2: if proposal precision is too low or no applied modification moves the suite, the project closes.

## Phase 3 — Autonomous application, Layers A and S1

Goal: close the loop. Modifications on Layers A and S1 are applied autonomously within budgets and thresholds. Operator only audits in aggregate.

Build:

- Budget enforcement in `eval` and `validation`: `max_tokens_per_window`, `max_modifications_per_window`, `max_chained_modifications`, `max_tokens_per_validation`, `max_wall_seconds_per_validation`, `early_stop_on_first_regression`.
- Threshold logic in `validation`: `min_relative_improvement`, `regression_tolerance`, `validation_runs_required`, `pareto_keep_when_uncomparable`.
- Scheduler: the proposal pass runs on cadence (post-session or scheduled), validates within budget, applies or archives, never asks the operator at proposal time.
- Freeze caps: when modifications applied exceed `max_modifications_per_window`, the system freezes further auto-application until the next audit.
- Rollback: `greco modification revert <id>` and `greco harness checkpoint restore <id>`. Every applied modification creates an implicit checkpoint.
- Audit report extended: applied modifications with deltas, rolled-back modifications, suite coverage notes, budget consumption against caps.

Exit criteria:

- One audit window completed under autonomous operation on Layers A and S1 with no operator approval during the window.
- Aggregate friction on the suite shows measurable improvement vs. the Phase 1 baseline. The exact delta is operator judgment based on the audit.
- Zero security incidents: no modification escaped the path guard, no settings/permissions modified at this phase, no suite file modified by the system.
- Rollback verified end to end.

Decision gate at the end of Phase 3: this is the v0 acceptance gate. If aggregate friction on the suite has not improved meaningfully and predictably, the recalibrated thesis has not been validated and the project closes.

## Phase 4 — Higher-risk layers under stricter audit

Goal: unlock Layers B, C, D, S2, S3. Layer E remains operator-gated explicitly.

Build:

- Layer B autonomous application with the same threshold structure as A.
- Layer C with stricter thresholds and a mandatory two-validation rule.
- Layer D (system prompt edits) requires the proposed diff to appear verbatim in the audit report and to be applied only after the next audit cadence passes without operator veto.
- Layer S2 (subagent toolset) and S3 (new subagents) autonomous within stricter thresholds; the operator audits the subagent declarations as part of the audit report.
- Layer E (settings, hooks, permissions) stays frozen for explicit operator approval per-modification.

Exit criteria:

- One audit window where modifications across A, B, C, S1, S2, S3 coexist and the aggregate suite trend remains positive.
- Diff-visible Layer D edits flagged in audit reports.
- Documented procedure for the operator to handle Layer E proposals.

## Phase 5 — Hardening and second-project validation

Goal: ensure Greco operates on a project other than the one it was tuned on.

Build:

- Suite-independence checks: aggregate metrics on a second operator project, with its own suite, replicated independently.
- Audit cadence templates.
- Catalog lint: detect near-duplicate active modifications, contradictory edits, dead procedures.
- `greco report bundle --redact`: a portable bundle of suite, catalog, traces, and audit reports with secrets stripped.

Exit criteria:

- Second project shows non-zero friction reduction.
- Catalog lint clean.
- Bundle reproducible.

## Beta Gate

Greco does not call itself beta until:

- Phase 3 acceptance gate is passed.
- A second project validates that the mechanism is not a single-project artifact.
- The README reflects measured behavior, not aspirational claims.
- Secret scans and CI are green.

## Verification, every phase

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Secret scan:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```

## Implementation owner

Codex is the implementation owner from Phase 1 onward. This documentation set is the contract Codex implements against. Open questions on contract details should be answered by amending the design or recalibration documents, not by adding implementation-only conventions.
