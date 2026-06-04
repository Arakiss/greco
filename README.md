<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/banner.svg">
    <img src="assets/banner.svg" alt="Greco - a Rust harness that improves itself" width="100%" />
  </picture>
</p>

<p align="center">
  <a href="https://github.com/Arakiss/greco/actions/workflows/ci.yml"><img src="https://github.com/Arakiss/greco/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/rust-1.90%2B-orange.svg" alt="Rust 1.90+"></a>
  <a href="docs/architecture/design.md"><img src="https://img.shields.io/badge/status-recalibrated--alpha-blue.svg" alt="Recalibrated alpha"></a>
</p>

# Greco

> A Rust coding-agent harness that observes its own use and improves itself, within budgets and a suite the operator defines.

**Development status: recalibrated alpha.** The original alpha cycle (`0.1.0-alpha.1` through `0.3.0-alpha.1`) explored a skill-catalog evolutionary axis. After the loop closed, a critical review concluded the axis was self-referential and did not test the deeper aspiration of the project. The axis was replaced at `0.4.0-alpha.1`; `0.5.0-alpha.1` adds the first manual, reversible modification lifecycle; `0.6.0-alpha.1` adds the first bounded autonomous loop for Layer A/S1; `0.6.1-alpha.1` makes promotion depend on stored comparative evidence instead of pass/fail validation alone; `0.7.0-alpha.1` adds the deterministic Phase 3 gate command. Current work incorporates the harness-benefit correction from arXiv:2605.30621: keep the evolver cheap, measure whether the solver activates and follows the harness, and evaluate local open-weight solvers in an isolated MLX lab before touching the core runtime. The skill code is preserved as historical scaffolding.

Greco's evolutionary unit is the *harness modification*: a typed, layered, reversible change to the control plane around a frozen model. Session traces reveal friction. The agent proposes modifications. Modifications are validated empirically against an operator-defined evaluation suite within strict budgets. Modifications that meet thresholds are applied autonomously. The operator does not approve per proposal; the operator designs the experiment and audits aggregate behavior on a cadence.

## Where It Fits

A serious coding-agent harness has several layers:

1. **Model provider**: OpenAI `gpt-5.4` through the Responses API.
2. **Primitive tools**: read, write, edit, bash.
3. **Subagent definitions**: declared, scoped, version-able.
4. **Active harness state**: system prompt, tool surface, settings, hooks, cached procedures.
5. **Evaluation suite**: operator-owned, read-only for the system.
6. **Modification loop**: friction detection, proposal, validation, application, archive, rollback.
7. **Audit surface**: aggregate reports for the operator on cadence.
8. **Operator surface**: CLI and plain-text TUI snapshots.

Greco owns layers 2-8. It does not try to be an IDE, a platform, a subagent runtime, or a hosted service.

## Why

Most "self-improving agent" talk collapses three different things: reasoning better inside one session, changing prompts/tools around a frozen model, and changing model weights. Greco targets the middle layer.

The bet:

- **The harness is the right unit.** Shared coding-agent products keep harness evolution conservative because their control plane affects many users. A *local, single-operator* harness can test autonomous iteration under strict budgets and audit.
- **Layered modifications.** Cached procedures and subagent prompt edits are cheap and low-risk; settings and permissions are expensive and high-risk. The system gates by layer, not by uniform rule.
- **Empirical admission against a real suite.** A modification is "an improvement" only if it reduces measurable friction on tasks the operator actually cares about. Proposal-level approval is replaced by suite-level evidence.
- **The human is not a notary.** The operator designs the suite, the budgets, and the thresholds. The operator audits aggregate behavior on cadence. Per-proposal approval is eliminated for the autonomous layers.
- **Persistent archive with lineage.** Every applied modification creates a checkpoint. Rollback is one command. Failed modifications stay archived because the archive is the memory substrate.

## Levels of Modification

| Layer | What changes | Risk | Gate |
|-------|---|---|---|
| **A** | Cached procedure | Low | Autonomous within budgets |
| **B** | Tool description or schema | Low | Autonomous within budgets |
| **C** | Composite tool from primitives | Medium | Autonomous, stricter thresholds |
| **D** | System prompt edit | Medium | Diff visible in audit, applies after cadence |
| **E** | Settings, hooks, permissions | High | Explicit per-modification operator approval |
| **S1** | Subagent prompt | Low-medium | Autonomous within budgets |
| **S2** | Subagent toolset | Medium | Autonomous, stricter thresholds |
| **S3** | New subagent definition | Medium | Autonomous, stricter thresholds |
| F | Primitive implementations | Very high | Out of scope v0 |
| G | Agent loop / harness code | DGM scale | Out of scope v0 |

## Current Surface

The alpha skill commands remain operational as historical scaffolding. The recalibrated surface now has measurable eval/audit commands, a manual modification lifecycle, and a bounded autonomous loop.

Alpha skill commands (historical):

```sh
greco --version
greco status --json
greco ask --input "Read README.md and summarize the project" --max-turns 6
greco tool read README.md
greco tool write scratch.txt "hello"
greco tool edit scratch.txt hello goodbye
greco tool bash "cargo test" --timeout 120
greco propose-skill --task "..."
greco catalog list --state all --json
greco catalog validate <id> --json
greco catalog promote <id> --json
greco catalog reject <id> --reason "..."
greco validate-skill examples/skills/pass --json
greco tui --snapshot
```

Recalibrated commands (Phase 1+):

```sh
greco eval list
greco eval run <task-id|all>
greco audit --since <window>
greco propose --since <window>
greco modification list --state <state>
greco modification show <id> [--diff]
greco modification validate <id>
greco modification apply <id>
greco modification revert <id>
greco loop run --since <window> [--dry-run|--apply]
greco loop status
greco loop gate --since <window>
greco loop freeze --reason <text>
greco loop unfreeze
```

Planned commands:

```sh
greco eval probe <off-suite-task>
greco harness checkpoint list
greco harness checkpoint restore <id>
```

`greco ask` continues to use OpenAI and requires `OPENAI_API_KEY`. Each run prints the local trace path on stderr. Local tool commands and validation do not require network access.

## Install From Source

```sh
cargo install --path . --force
```

The future crates.io package is `greco-cli` because `greco` is already occupied on crates.io. The installed binary remains `greco`.

## Local Configuration

```sh
cp .env.example .env.local
chmod 600 .env.local
```

`.env.local` is ignored by git. Greco also reads `~/.config/greco/env` for user-level local credentials.

```sh
OPENAI_API_KEY=...
GRECO_PROVIDER=openai
GRECO_MODEL=gpt-5.4
GRECO_HOME=.greco
```

Any API key copied through an untrusted channel must be rotated before public release.

## Modification Lifecycle

```text
session
  -> trajectory + friction instrumentation
  -> offline proposal pass over recent traces
  -> typed modification candidate (layer A/B/C/D/E or S1/S2/S3)
  -> validation against the suite within budgets
  -> apply autonomously | archive as rejected | escalate to next audit
  -> if applied: monitor over subsequent sessions
  -> aggregate audit report on cadence
  -> operator may rollback to any prior checkpoint
```

States in `.greco/modifications/`: `proposed/`, `validated/`, `active/`, `rejected/`, `retired/`. (`.greco/catalog/` is the separate historical *skill* archive, not the modification store.) No artifact is ever deleted in v0: a colliding destination is retired, never removed. Every manifest carries `layer`, `friction_source`, typed `payload`, `lineage`, and—once validated—`validation` evidence and `rollback` metadata.

Only layers `A` (cached procedure) and `S1` (subagent prompt) are implemented in v0; the `ModificationLayer` enum cannot represent `B`–`E`/`S2`/`S3`, so those rows in the table above are roadmap, not current behavior. This is also the safety property that makes a Layer E (settings/permissions) change impossible to apply autonomously: it cannot be constructed.

Example manifest:

```json
{
  "id": "layer-a-avoidable-prompts-1780000256462",
  "version": "0.1.0",
  "layer": "a",
  "state": "proposed",
  "description": "Layer A cached procedure generated from `avoidable-prompts` friction over 11 sessions",
  "friction_source": {
    "since": "all",
    "session_count": 11,
    "eval_run_count": 5,
    "dominant_signal": "avoidable-prompts",
    "turns": 33,
    "tool_calls": 19,
    "tokens": 54073,
    "repeated_errors": 0,
    "retracements": 0,
    "avoidable_prompts": 1,
    "missing_tool_failures": 0
  },
  "payload": {
    "kind": "cached_procedure",
    "title": "Reduce avoidable prompts",
    "body": "Prefer reading the smallest relevant file region before asking.",
    "prompt_budget_chars": 1200
  },
  "validation": null,
  "lineage": {
    "parent_id": null,
    "reason": "generated by greco propose from aggregate trace friction"
  },
  "rollback": null,
  "created_at_unix_ms": 1780000256462,
  "applied_at_unix_ms": null,
  "reverted_at_unix_ms": null
}
```

## Friction Signals

Computed deterministically from traces, not from model judgment:

- `turns_per_task`
- `tokens_per_task`
- `repeated_errors`
- `retracements`
- `avoidable_prompts`
- `missing_tool_failures`
- `harness_artifacts_available`
- `harness_artifacts_loaded`
- `harness_activation_failures`
- `harness_adherence_checks`
- `harness_adherence_misses`
- `objective_success`

A modification must improve one or more without regressing any beyond a declared tolerance. The system retains a Pareto frontier when modifications trade off across metrics.

## Operator Cadence

Three frequencies:

- *During sessions*: regular agent use. Optional friction tagging.
- *On cadence (audit)*: review the audit report. Decide on Layer E proposals if any. Adjust suite or budgets if drift appears.
- *Rare configuration*: edit the suite, adjust budgets and thresholds, define a new subagent baseline.

No per-proposal approval at any phase from Phase 3 onward except for Layer E.

## Design Decisions

- **Responses API first.** Current OpenAI guidance recommends Responses for new agentic projects.
- **OpenAI only in v0.** Greco has a narrow provider trait, but one implementation.
- **Direct HTTP.** No `async-openai`, `rig`, `swiftide`, or `llm-chain` in v0.
- **Filesystem archive first.** SQLite can come later if scale demands.
- **Typed diffs only.** Free-form patches are not admissible as modifications at v0.
- **Plain TUI first.** Snapshot output is useful to humans and agents before widget frameworks are justified.

See:

- [`docs/architecture/design.md`](docs/architecture/design.md) — v0 design under the new axis
- [`docs/architecture/recalibration.md`](docs/architecture/recalibration.md) — why the axis changed and how harness-benefit research corrects it
- [`docs/architecture/critical-analysis.md`](docs/architecture/critical-analysis.md) — answers to the RFC critical questions under the new axis
- [`docs/research/2605-30621-implications.md`](docs/research/2605-30621-implications.md) — paper-specific implications for Greco
- [`docs/operations/local-model-lab.md`](docs/operations/local-model-lab.md) — isolated local solver / MLX evaluation strategy
- [`docs/operations/roadmap.md`](docs/operations/roadmap.md) — versioned roadmap
- [`docs/operations/secret-handling.md`](docs/operations/secret-handling.md) — credential handling rules
- [`THREAT_MODEL.md`](THREAT_MODEL.md) — assets, boundaries, controls, new threats from self-modification

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -- status --json
```

Secret check:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```

## Repository Map (transition state)

```text
src/
  agent.rs             Responses tool loop and trajectory handoff
  main.rs              CLI dispatch
  cli.rs               manual argument parser
  config.rs            env and local config loading
  provider/            model-provider trait and OpenAI adapter
  modification.rs      typed harness modification registry and lifecycle
  eval.rs              operator-owned eval suite runner (criteria, budgets)
  audit.rs             deterministic friction aggregation over traces
  loop_control.rs      bounded autonomous loop: validate, compare, gate, freeze
  proposal.rs          alpha skill proposal (historical skill-axis scaffold)
  tools.rs             primitive tool schemas and local execution
  trajectory.rs        JSONL session traces with friction instrumentation
  catalog.rs           alpha skill archive (historical skill-axis scaffold)
  validation.rs        alpha skill validation (historical skill-axis scaffold)
  tui.rs               plain-text operator snapshots
docs/
  architecture/        design, critical analysis
  operations/          roadmap, secret handling
examples/skills/       alpha skill fixtures (retained as historical reference)
```

Implemented recalibrated modules: `eval`, `audit`, `modification`. Later modules remain gated by the roadmap rather than phase-number versioning.

## Not In Scope

- subagent framework as a general platform;
- MCP;
- web UI;
- hosted marketplace;
- modification of primitive implementations (Layer F);
- modification of the agent loop or harness code itself (Layer G);
- prompt evolution outside the explicit Layer D mechanism with audit diff;
- context/playbook evolution as a separate axis;
- model fine-tuning.

## Honest Closure

The roadmap declares explicit decision gates at the end of Phase 1, Phase 2, and Phase 3. Each gate has a structural failure mode (noisy friction signals, junk proposals, no measurable aggregate improvement). Failing any gate triggers an honest closure with a final `What I learned` document. The point of the recalibration is not to extend the project at any cost. It is to give the project a thesis that can actually be tested.

**Open structural gap (the load-bearing one).** Validation currently runs only the operator's deterministic eval *criteria*; it does not yet run the *solver* (the model) on each task with and without the candidate procedure. A Layer A modification only affects behavior through the model's runtime prompt, which the criteria never exercise, so `primary_improvement` is structurally pinned at `0ppm` and the Phase 3 gate reports `NeedsMoreData` by construction — not by accident. The governance machinery around the experiment (typed layers, budgets, freeze, rollback, comparison, gate) is real and exercised; the measurement at its center is not yet wired to the quantity the thesis is about. Closing this — running `run_agent` inside validation against an isolated workspace copy — is the next decision, and it is what makes a Phase 3 `Pass` reachable at all.

## Status Summary

The alpha skill cycle is closed and documented. The recalibrated v0 begins at `0.4.0-alpha.1` with instrumentation and baseline (Phase 1). `0.5.0-alpha.1` is the first release-gated Phase 2 alpha because it adds new CLI commands and a persistent modification-manifest format. `0.6.0-alpha.1` is the first release-gated Phase 3 alpha because it adds autonomous loop commands and persistent budget/freeze/checkpoint state. `0.6.1-alpha.1` is a Phase 3 patch because loop decisions now carry baseline-vs-candidate comparison deltas and apply only when the threshold policy admits them. `0.7.0-alpha.1` is a minor alpha because it adds `greco loop gate`, the public acceptance verdict surface for Phase 3 windows. The tracked documentation set in `docs/` is the public product contract.
