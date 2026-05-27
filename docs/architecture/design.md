# Greco v0 Design

## Product Statement

Greco is a terminal-first Rust coding-agent harness whose only v0 evolutionary surface is its local skill catalog.

The agent does not rewrite its own harness. It proposes skills, validates them empirically, promotes only successful candidates, and preserves the archive of rejected and retired variants.

## Architecture

```text
greco-cli
  commands, TUI-ready snapshots, operator UX

greco-core
  sessions, primitive tools, skill catalog, validation loop, traces

greco-provider
  narrow model-provider trait and OpenAI Responses implementation

.greco/
  local archive, traces, scores, generated skill candidates
```

The initial implementation can live in one crate if that keeps the first cut smaller. The module boundaries above should still be reflected in code so splitting into crates later is mechanical.

## Provider Boundary

The v0 trait should be intentionally small:

```rust
trait ModelProvider {
    async fn respond(&self, request: ModelRequest) -> Result<ModelResponse>;
    async fn stream(&self, request: ModelRequest) -> Result<ModelStream>;
}
```

Rules:

- `OpenAiProvider` is the only implementation.
- The default model is `gpt-5.4`.
- OpenAI-specific fields stay inside the OpenAI adapter whenever possible.
- The core harness receives normalized assistant text, tool calls, usage, and raw trace references.
- Do not design a universal provider feature matrix in v0.

## Primitive Tools

Greco exposes four primitive tools:

- `read`: read UTF-8 text from files inside the workspace.
- `write`: create or replace files inside the workspace.
- `edit`: apply bounded textual edits inside the workspace.
- `bash`: run shell commands with cwd, timeout, and captured stdout/stderr.

All tool calls produce trace items. Tool outputs are fed back into the model through Responses function-call outputs.

All actions should write trajectory records with at least:

- timestamp;
- request/session id;
- action type;
- normalized input;
- observation/result;
- success flag;
- validation or command evidence when available.

## Skill Catalog

Skill manifests are JSON:

```json
{
  "id": "rust_test_fix",
  "version": "0.1.0",
  "kind": "script",
  "entrypoint": "run.sh",
  "description": "Repairs simple Rust test failures by running cargo test and applying focused edits.",
  "inputs": {
    "task": "string"
  },
  "validation": {
    "command": "cargo test",
    "timeout_seconds": 120
  }
}
```

Active skills require:

- Valid manifest.
- Executable entrypoint or buildable Rust package.
- At least one passing validation trace.
- Score initialized from validation evidence, not model confidence.

Skill variants should keep lineage:

```json
{
  "candidate_id": "cand_...",
  "parent_id": "rust_test_fix@0.1.0",
  "mutation_reason": "Captured repeated cargo-nextest retry pattern",
  "source_trace": "traces/sessions/..."
}
```

This keeps Greco closer to DGM/GEPA archival discipline without evolving the harness itself.

Rejected candidates remain archived with failure traces.

## Validation Loop

1. Candidate is written under `.greco/catalog/candidates/<candidate-id>/`.
2. Greco parses and statically validates the manifest.
3. Greco creates a temp validation workspace.
4. Greco invokes the candidate with bounded env, cwd, and timeout.
5. Greco runs validation commands or checks declared expected outputs.
6. Greco writes a JSONL trace.
7. Greco promotes, rejects, or leaves the candidate pending.

Promotion is a file move plus score update. No candidate is deleted during v0.

## Scoring

v0 scoring should be simple:

- `attempts`
- `passes`
- `failures`
- `last_used_at`
- `last_validated_at`
- `score = passes / attempts` with conservative defaults

This is deliberately not a learned ranking model. Hundreds of skills may require a better retrieval/ranking mechanism later.

## TUI Direction

Gommage's terminal dashboard proves that a dependency-free snapshot/watch style can be useful before a full interactive UI. Greco should start with:

- `greco status --json`
- `greco catalog list --json`
- `greco tui --snapshot`

The first TUI should be plain text and agent-readable. Rich terminal interaction can come after the core loop proves useful.

Gommage's concrete pattern to reuse:

- render plain `Vec<String>`-style views before adopting a widget framework;
- support snapshot/watch modes that work in headless terminals;
- test that snapshot output has no ANSI escapes;
- keep JSON command surfaces separate from human-readable output.

## Non-Goals

- No subagents.
- No MCP.
- No web dashboard.
- No auto-modification of the harness.
- No prompt evolution.
- No context/playbook evolution.
- No fine-tuning.
- No provider marketplace.

## Versioning

Greco follows Semantic Versioning. While pre-1.0:

- Breaking CLI, trace schema, manifest schema, or provider trait changes require a minor bump.
- Compatible fixes use patch bumps.
- Release notes should later be generated from Conventional Commits or release-please.

Current alpha version: `0.2.0-alpha.1`.
