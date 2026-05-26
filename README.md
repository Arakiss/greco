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
  <a href="docs/architecture/design.md"><img src="https://img.shields.io/badge/status-alpha--scaffold-blue.svg" alt="Alpha scaffold"></a>
</p>

# Greco

> A Rust coding-agent harness whose evolutionary unit is the local skill catalog.

**Development status: alpha scaffold.** Greco is private, early, and intentionally narrow. The current build proves the project shape: direct OpenAI Responses integration, a small provider seam, primitive tool definitions, local tool execution, skill manifests, empirical validation fixtures, and a plain-text TUI snapshot. It is not ready for unattended real-project use.

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
greco ask --input "Explain this repository in one paragraph"
greco ask --input "Hello" --stream
greco tool read README.md
greco tool write scratch.txt "hello"
greco tool edit scratch.txt hello goodbye
greco tool bash "cargo test" --timeout 120
greco catalog list --json
greco validate-skill examples/skills/pass --json
greco tui --snapshot
```

`greco ask` uses OpenAI and requires `OPENAI_API_KEY`. Local smoke tests and skill validation do not require network access.

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
candidate -> static checks -> bounded validation -> active | rejected
```

Active skills require:

- a valid `manifest.json`;
- an existing entrypoint;
- a passing validation command or task check;
- a retained validation trace.

Rejected skills are not deleted in v0. They remain archive material for future lineage, mutation, and diagnosis.

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
```

Secret check:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```

## Repository Map

```text
src/
  main.rs              CLI dispatch
  cli.rs               manual argument parser
  config.rs            env and local config loading
  provider/            model-provider trait and OpenAI adapter
  tools.rs             primitive tool schemas and local execution
  catalog.rs           skill manifest and active catalog loading
  validation.rs        empirical skill validation scaffold
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

Greco currently proves the skeleton. The next serious milestone is closing the full tool loop: model emits primitive tool calls, Greco executes them, returns function-call outputs, and records a trajectory trace. Only after that should skill generation become autonomous.
