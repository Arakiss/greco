# Roadmap

## 0.1.0-alpha.1

- Secret-safe repo shell.
- Research and design docs.
- Rust CLI scaffold.
- OpenAI Responses request/response/SSE parser.
- Primitive tool schemas and local execution commands.
- Skill manifest parsing and validation fixtures.
- Plain-text TUI snapshot.

## 0.2.0-alpha.1

- Full Responses tool loop.
- Function-call output submission.
- Session trajectory JSONL.
- Stateless reasoning preservation via `reasoning.encrypted_content` include.
- Live smoke against OpenAI `gpt-5.4` using only ignored `.greco` artifacts.

## 0.3.0-alpha.1

- Candidate skill archive layout.
- Promotion/rejection file moves.
- Skill proposal structured output schema.
- Multi-task validation fixtures.
- Score file and catalog lint.
- `greco report bundle --redact`.

## Beta Gate

Greco should not be called beta until:

- a simple coding task can complete through read/edit/bash;
- at least one generated skill can be proposed, validated, promoted, and reused;
- rejected candidates leave useful traces;
- secret scans and CI are green;
- the README matches actual behavior.
