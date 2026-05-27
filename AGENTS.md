# Greco Agent Contract

Greco is a Rust coding-agent harness whose v0 evolutionary axis is the harness itself. The model is frozen; the harness around it evolves. See [`docs/architecture/design.md`](docs/architecture/design.md) and [`docs/architecture/recalibration.md`](docs/architecture/recalibration.md) for the load-bearing context.

This document is the agent contract: the rules the agent (model + harness) must respect when operating Greco and when proposing changes to it.

## Operating Rules

- Keep the harness small enough to read in one sitting.
- Prefer direct Rust, `tokio`, `reqwest` with `rustls`, `serde`, and `serde_json` over broad agent frameworks.
- Keep provider abstractions narrow: one trait, one OpenAI implementation, no speculative cross-provider semantics.
- Do not apply a harness modification to the active state without empirical validation evidence against the evaluation suite.
- Preserve failed modification candidates and validation traces; archival memory is part of the product.
- Keep local secrets in `.env.local` or user-level secure storage only. Never commit credentials.

## Modification Discipline

- Every modification carries a typed manifest declaring layer, subject (harness or subagent), diff, lineage, and target metric.
- Diffs are typed; free-form patches are not admissible.
- Modifications cannot touch Layer F (primitives) or Layer G (harness code itself) in v0.
- Layer E modifications (settings, hooks, permissions) are never applied autonomously. They wait for explicit operator approval at the next audit cadence.
- Layer D modifications (system prompt edits) appear verbatim in the next audit report and apply only after the next cadence passes without operator veto.
- All other layers (A, B, C, S1, S2, S3) apply autonomously within budgets and thresholds once Phase 3 is reached.

## Evaluation Suite Discipline

- The evaluation suite under `.greco/eval/` is read-only for the agent and the proposal pass.
- The agent must not write to suite files, including criterion scripts and reference artifacts.
- Suite criterion commands run with `env_clear`, bounded timeout, and the workspace path guard. The agent cannot weaken these invariants from inside the loop.
- Off-suite probes (`greco eval probe <task>`) produce reports but do not feed back into the modification loop.

## Budget Discipline

- The agent and the proposal pass respect `max_tokens_per_window`, `max_tokens_per_validation`, `max_wall_seconds_per_validation`, `max_modifications_per_window`, and `max_chained_modifications`.
- When a budget exhausts, the system marks the affected modification as `validation_inconclusive` and freezes further autonomous activity until the next operator audit.
- Budget consumption is reported per window in the audit.

## Trace Discipline

- Every session produces a JSONL trajectory under `.greco/traces/sessions/`.
- Every proposal pass produces a JSONL under `.greco/traces/proposals/`.
- Every validation run produces a JSONL under `.greco/traces/validations/`.
- Every audit window produces markdown + JSON under `.greco/audit/`.
- Friction signals are deterministic counters extracted from traces, not model-judged scores.

## Pre-Claim Verification

Before producing any artifact that claims a state of the system (audit summary, modification rationale, validation summary), the agent must:

1. Read the actual current state from `.greco/state/` and `.greco/catalog/`.
2. Verify that the claim matches the on-disk evidence.
3. Cite the trace path or manifest path that supports each non-trivial claim.

The agent never asserts "this modification improved X" without pointing at a validation trace that shows the deterministic delta.

## Verification

Before claiming completion of any implementation task, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For secret checks, run:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```

The RFC, the recalibration document, and the design document may contain historical text but must not contain live credentials.

## Implementation Owner

Codex is the implementation owner from Phase 1 of the recalibrated plan onward. This documentation set is the contract Codex implements against. Open questions on contract details should be answered by amending the relevant document (design, recalibration, threat model, this contract), not by adding implementation-only conventions.
