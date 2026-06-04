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
  <a href="docs/architecture/design.md"><img src="https://img.shields.io/badge/status-experimental%20alpha-orange.svg" alt="Experimental alpha"></a>
</p>

# Greco

> An experiment in whether a coding-agent harness can measurably improve itself — built in the open, and candid about what is proven and what is not.

## Status: an early experiment, not a product

Greco is embryonic: a single-operator research project in alpha, not a tool to adopt. The thesis below is under test, and the project is built to be **declared false** if the evidence does not hold (see [Honest Closure](#honest-closure)). What works today runs locally and is covered by tests; what does not work yet is named plainly throughout this README instead of implied.

If you came looking for a self-improving agent to use, this is not that yet. If you came to follow — or pull apart — an attempt to turn "the harness improves itself" into a claim that can actually be tested, that is what this is.

It has already corrected course once. The first alpha cycle (`0.1.0`–`0.3.0`) evolved a skill catalog; a critical review found that axis self-referential and it was replaced at `0.4.0`. The unit of evolution now is the *harness modification*: a typed, layered, reversible change to the control plane around a frozen model. Session traces reveal friction, the agent proposes a modification, it is validated against an operator-defined evaluation suite within strict budgets, and modifications that clear the thresholds apply autonomously. The operator does not approve each proposal; the operator designs the experiment and audits aggregate behavior on a cadence.

## What's Real, What's Not Yet

Working and tested today:

- A coding-agent loop over OpenAI `gpt-5.4` (Responses API) with read/write/edit/bash tools, confined to the workspace and run with a credential-scrubbed environment.
- Deterministic friction signals computed from session traces and aggregated by `greco audit` — counters, not model-judged scores.
- A typed, layered, reversible modification lifecycle (`proposed → validated → active → rejected → retired`) in which no artifact is ever deleted.
- A bounded autonomous loop with budgets, freeze, one-command rollback, a baseline-vs-candidate comparison delta, and a deterministic Phase 3 acceptance gate.
- `greco eval solve`: the model attempts a task inside an isolated workspace snapshot and is graded by the operator's criteria, with the live workspace left untouched.

Not yet — and called out wherever it matters below:

- The *autonomous loop's* validation does not yet run the solver, so its `primary_improvement` is pinned at `0ppm` and the Phase 3 gate cannot return `Pass`. Wiring `eval solve` into the loop as a baseline-vs-candidate run is the next step, and the one that makes the thesis testable end to end.
- Only modification layers `A` (cached procedure) and `S1` (subagent prompt) are implemented; `B`–`E` and `S2`–`S3` are roadmap. The type for a layer cannot even represent a Layer E change, which is what makes one impossible to apply autonomously.
- No MCP, no web UI, no hosted service, no model fine-tuning.

The governance machinery is built and exercised; the experiment's central measurement is half-wired. Closing that gap is the next phase, not a footnote.

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
greco eval solve <task-id>
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

**Open structural gap (the load-bearing one).** The autonomous loop's validation still runs only the operator's deterministic eval *criteria*. A Layer A modification only affects behavior through the model's runtime prompt, which those criteria never exercise, so `primary_improvement` inside `greco loop run` is structurally pinned at `0ppm` and the Phase 3 gate reports `NeedsMoreData` by construction — not by accident.

`greco eval solve <task>` closes the first half of this gap: it snapshots the workspace into an isolated copy, runs the model (`run_agent`) on the task with the harness state active, and grades the copy with the task criteria — so the solver is finally in the loop and its work is measured. What remains is to run `eval solve` twice inside autonomous validation (baseline with no candidate vs candidate active), feed the real delta into the existing comparison, and make a Phase 3 `Pass` gate autonomous `--apply`. Until that wiring lands, the governance machinery (typed layers, budgets, freeze, rollback, comparison, gate) is real and exercised, but the autonomous loop's central measurement is not yet the quantity the thesis is about.

## Status Summary

The alpha skill cycle is closed and documented. The recalibrated v0 begins at `0.4.0-alpha.1` with instrumentation and baseline (Phase 1). `0.5.0-alpha.1` is the first release-gated Phase 2 alpha because it adds new CLI commands and a persistent modification-manifest format. `0.6.0-alpha.1` is the first release-gated Phase 3 alpha because it adds autonomous loop commands and persistent budget/freeze/checkpoint state. `0.6.1-alpha.1` is a Phase 3 patch because loop decisions now carry baseline-vs-candidate comparison deltas and apply only when the threshold policy admits them. `0.7.0-alpha.1` is a minor alpha because it adds `greco loop gate`, the public acceptance verdict surface for Phase 3 windows. `0.8.0-alpha.1` is a minor alpha because it adds `greco eval solve`, which runs the model inside an isolated workspace snapshot and grades it against the operator's criteria — the first half of putting the solver inside validation. The tracked documentation set in `docs/` is the public product contract.
