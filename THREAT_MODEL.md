# Threat Model

Greco is not a sandbox. It is a local harness that asks a model to select tools and may execute local subprocesses during tool use or skill validation.

## Assets

- Source code in the current workspace.
- Local credentials such as `OPENAI_API_KEY`.
- Skill catalog entries under `.greco/catalog`.
- Validation traces under `.greco/traces`.
- Git history and release artifacts.

## Trust Boundaries

- OpenAI model output is untrusted.
- Proposed skills are untrusted until validation passes.
- Skill validation commands are untrusted subprocesses.
- `.env.local` and `~/.config/greco/env` are local secret stores, not repository artifacts.

## Current Controls

- `.env.local` is ignored by git.
- Local credential files are expected to be `0600`.
- Primitive file tools reject absolute paths and `..` traversal.
- Validation commands run with a cleared environment and bounded timeout.
- Skill manifests must parse and point to an existing entrypoint before validation.

## Known Gaps

- v0 does not provide OS-level sandboxing.
- v0 does not yet enforce a write allowlist beyond path traversal checks.
- v0 does not yet persist full trajectory traces.
- v0 does not yet promote/reject by moving candidate directories.
- v0 does not yet verify generated Rust skill packages in isolated containers.

## Operator Guidance

Use Greco first on non-critical repositories. Keep normal Codex/Claude/Cursor sandboxing enabled when using those hosts around Greco. Rotate any API key that has been pasted into chat before making the repository public.
