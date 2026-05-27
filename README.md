<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/banner.svg">
    <img src="assets/banner.svg" alt="Greco - a Rust harness for evolving coding-agent skills" width="100%" />
  </picture>
</p>

<p align="center">
  <a href="https://github.com/Arakiss/greco/actions/workflows/ci.yml"><img src="https://github.com/Arakiss/greco/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/rust-1.90%2B-orange.svg" alt="Rust 1.90+"></a>
  <a href="docs/architecture/design.md"><img src="https://img.shields.io/badge/status-alpha--evolution--loop-blue.svg" alt="Alpha evolution loop"></a>
</p>

# Greco

> A Rust coding-agent harness whose evolutionary unit is the local skill catalog.

**Development status: alpha evolution loop.** Greco is private, early, and intentionally narrow. The current build can run a Responses API function-calling loop against OpenAI `gpt-5.4`, propose script skills through Structured Outputs, persist candidates, validate them empirically, promote passing candidates, reject failures, update scores, and retain JSONL traces. It is not ready for unattended real-project use.

Greco starts from the Kappa RFC thesis: the model is not retrained and the harness does not rewrite itself. Instead, the agent proposes skills, Greco validates them empirically, and only passing candidates enter the active catalog. Failed candidates stay archived because the archive is the memory substrate.

## Where It Fits

A serious coding-agent harness has several layers:

1. **Model provider**: initially OpenAI `gpt-5.4` through the Responses API.
2. **Primitive tools**: read, write, edit, and bash.
3. **Skill catalog**: local executable patterns proposed by the agent.
4. **Validation gate**: static checks, bounded execution, and explicit task fitness.
5. **Archive and traces**: active, rejected, retired, and validation evidence.
6. **Operator surface**: CLI and plain-text TUI snapshots before richer interaction.

Greco owns layers 2-6. It does not try to be an IDE, a platform, a subagent runtime, or a hosted service.

## Why

Most "self-improving agent" talk collapses several different ideas: reasoning better within one session, changing prompts/tools around a frozen model, or changing model weights. Greco targets the middle layer.

The core bet is practical:

- **Small harness.** If the agent is going to improve its environment, that environment must be readable.
- **Skills as the unit.** A bad skill can be rejected without destabilizing the whole loop.
- **Empirical admission.** A skill is not active because it sounds good. It is active because it passed validation.
- **Persistent archive.** Improvement across sessions requires remembered successes and failures.
- **Terminal first.** The operator should be able to inspect everything with ordinary shell tools.

## Current Surface

```sh
greco --version
greco status --json
greco ask --input "Read README.md and summarize the project" --max-turns 6
greco tool read README.md
greco tool write scratch.txt "hello"
greco tool edit scratch.txt hello goodbye
greco tool bash "cargo test" --timeout 120
greco propose-skill --task "Create a tiny reusable shell skill that prints GRECO_OK" --json
greco catalog create-candidate --id demo --description "Prints demo" --script '#!/bin/sh
printf "%s\n" demo' --validation-command "sh run.sh | grep -x demo"
greco catalog validate demo --json
greco catalog promote demo --json
greco catalog reject demo --reason "not useful"
greco catalog list --state all --json
greco validate-skill examples/skills/pass --json
greco tui --snapshot
```

`greco ask` uses OpenAI and requires `OPENAI_API_KEY`. It runs buffered even when `--stream` is passed, because streaming function-call orchestration needs a separate assembler for partial argument events. Each run prints the local trace path on stderr, for example `.greco/traces/sessions/<id>.jsonl`. Local tool commands and skill validation do not require network access.

## Install From Source

```sh
cargo install --path . --force
```

The future crates.io package should be `greco-cli` because `greco` is already occupied on crates.io. The installed binary remains `greco`.

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

The initial API key used during project creation was provided in chat and should be rotated before any public release.

## Skill Lifecycle

```text
task -> proposed candidate -> validation trace -> active | rejected
```

There are two candidate entry paths:

- `greco propose-skill --task ...` asks OpenAI for a structured JSON skill proposal and writes a proposal trace under `.greco/traces/proposals/`.
- `greco catalog create-candidate ...` creates a deterministic local candidate without network access.

Active skills require:

- a valid `manifest.json`;
- an existing entrypoint;
- a passing validation command or task check;
- a retained validation trace.

Rejected skills are not deleted in v0. They move to `.greco/catalog/rejected/` with rejection metadata and remain archive material for future lineage, mutation, and diagnosis. Scores live in `.greco/catalog/scores.json`.

Example manifest:

```json
{
  "id": "example_pass",
  "version": "0.1.0",
  "kind": "script",
  "entrypoint": "run.sh",
  "description": "Minimal skill fixture that passes validation.",
  "validation": {
    "command": "sh run.sh",
    "timeout_seconds": 5
  }
}
```

## Design Decisions

- **Responses API first.** Current OpenAI guidance recommends Responses for new agentic projects.
- **OpenAI only in v0.** Greco has a narrow provider trait, but one implementation.
- **Direct HTTP.** No `async-openai`, `rig`, `swiftide`, or `llm-chain` in v0.
- **Filesystem archive first.** SQLite can come later if the catalog actually outgrows files.
- **Subprocess skills.** Dynamic plugins and FFI are too much surface for v0.
- **Plain TUI first.** Snapshot output is useful to humans and agents before widget frameworks are justified.

See:

- [`docs/research/current-source-review.md`](docs/research/current-source-review.md)
- [`docs/architecture/critical-analysis.md`](docs/architecture/critical-analysis.md)
- [`docs/architecture/design.md`](docs/architecture/design.md)
- [`docs/operations/implementation-plan.md`](docs/operations/implementation-plan.md)
- [`docs/operations/risks.md`](docs/operations/risks.md)

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -- status --json
cargo run -- validate-skill examples/skills/pass --json
cargo run -- validate-skill examples/skills/fail --json
cargo run -- ask --max-turns 6 --input "Use read on README.md, then answer with one sentence."
```

Secret check:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```

## Repository Map

```text
src/
  agent.rs             Responses tool loop and trajectory handoff
  main.rs              CLI dispatch
  cli.rs               manual argument parser
  config.rs            env and local config loading
  provider/            model-provider trait and OpenAI adapter
  proposal.rs          structured skill proposal via Responses text.format
  tools.rs             primitive tool schemas and local execution
  trajectory.rs        JSONL session traces
  catalog.rs           candidate, active, rejected, and score archive
  validation.rs        empirical skill validation and traces
  tui.rs               plain-text operator snapshots
docs/
  research/            current-source review
  architecture/        design and critical analysis
  operations/          implementation plan, risks, secret handling
examples/skills/       passing and failing validation fixtures
```

## Not In Scope

- subagents;
- MCP;
- web UI;
- hosted marketplace;
- automatic harness self-modification;
- prompt evolution;
- context/playbook evolution;
- model fine-tuning.

Those are future research vectors, not v0.

## Status Summary

Greco now proves the first evolutionary loop: proposal, candidate persistence, empirical validation, promotion/rejection, scores, and traces. The next serious milestone is skill reuse during `greco ask`: active skills should be discoverable and invocable alongside primitive tools, with score-aware selection.
