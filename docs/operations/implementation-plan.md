# Implementation Plan

## Phase 0: Repository Foundation

Exit criteria:

- Git repository exists under `indie-hackers/greco`.
- `.env.local` is ignored.
- Local OpenAI key is stored outside git.
- Research, critical analysis, design, and risk docs exist.
- First Lore commits are present.

## Phase 1: Rust CLI Skeleton

Build:

- `Cargo.toml`
- `rust-toolchain.toml`
- `src/main.rs`
- `src/config.rs`
- `src/provider/mod.rs`
- `src/provider/openai.rs`
- `src/tools.rs`
- `src/catalog.rs`
- `src/validation.rs`
- `src/tui.rs`

Commands:

- `greco --version`
- `greco status --json`
- `greco ask --input <text> [--max-turns <n>] [--stream]`
- `greco catalog list --json`
- `greco validate-skill <path> --json`
- `greco tui --snapshot`

Exit criteria:

- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

## Phase 2: OpenAI Responses Adapter

Build:

- Direct `reqwest` client.
- JSON request/response structs.
- SSE parser for text deltas and completed events.
- Minimal function-tool schema generation for primitive tools.
- `store: false` default.
- `gpt-5.4` default from env/config.
- Raw output item retention for stateless tool loops.
- `reasoning.encrypted_content` include for reasoning item replay when `store: false`.

Exit criteria:

- Unit tests cover request construction, response text extraction, function-call extraction, and SSE event parsing.
- One opt-in live smoke command can run against `.env.local` without printing the key.

## Phase 3: Primitive Tool Execution

Build:

- Responses function-call loop.
- Function-call output submission.
- Session trajectory JSONL under `.greco/traces/sessions`.
- Workspace path guard.
- UTF-8 read/write.
- Simple edit by exact find/replace.
- Bash command runner with cwd, timeout, stdout/stderr capture.

Exit criteria:

- Tests prove path traversal is rejected.
- Tests prove model output items are preserved and tool outputs are returned.
- Tests prove bash timeout handling.
- Tests prove edit failure is explicit when the target text is absent.
- One live OpenAI smoke task completes through write, read, and bash.

## Phase 4: Skill Catalog and Validation

Build:

- Manifest schema.
- Candidate/active/rejected archive layout.
- Validation trace JSONL.
- Promotion gate.
- Basic score file.

Exit criteria:

- A fixture skill passes validation and promotes.
- A failing skill is rejected and archived with trace.
- Re-running validation is deterministic enough for local tests.

## Phase 5: README and Private Remote

Build:

- Gommage-quality README front door.
- Roadmap, threat model, and publishing notes.
- GitHub metadata: private repo, description, topics.
- Atomic Lore commits.

Exit criteria:

- Secret scan is clean.
- Remote is private.
- `main` is pushed.
- Optional `v0.2.0-alpha.1` tag is created only after the repo is verified.
