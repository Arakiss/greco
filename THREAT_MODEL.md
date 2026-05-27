# Threat Model

Greco is not a sandbox. It is a local harness that asks a model to select tools and may execute local subprocesses during tool use, validation, and now during the autonomous application of harness modifications. The current axis (harness self-improvement) introduces additional trust boundaries beyond the alpha skill cycle. See [`docs/architecture/recalibration.md`](docs/architecture/recalibration.md).

## Assets

- Source code in the current workspace.
- Local credentials such as `OPENAI_API_KEY`.
- The active harness state under `.greco/state/` (system prompt, tools, settings, hooks, subagent definitions, cached procedures, current checkpoint).
- The modification archive under `.greco/catalog/`.
- The evaluation suite under `.greco/eval/`.
- Session, proposal, validation, and audit traces under `.greco/traces/` and `.greco/audit/`.
- Git history and release artifacts.

## Trust Boundaries

- OpenAI model output is untrusted.
- Proposed harness modifications are untrusted until validation passes.
- Validation runs of candidate modifications execute against a temp harness state but on the real workspace's evaluation suite criteria; the criterion commands themselves run as local subprocesses and are untrusted.
- The evaluation suite is operator-trusted. The system must not modify suite files.
- Modification diffs are typed; free-form patches are not admissible at v0.
- `.env.local` and `~/.config/greco/env` are local secret stores, not repository artifacts.

## Current Controls

- `.env.local` is ignored by git.
- Local credential files are expected to be `0600`.
- Primitive file tools reject absolute paths and `..` traversal.
- Validation commands (criterion commands from the suite) run with a cleared environment and bounded timeout.
- Manifests must parse and target a permitted layer before any validation runs.
- The suite directory is mounted read-only for the proposal pass and the agent runtime; only the operator may edit suite files.
- Every applied modification creates a harness checkpoint before application. Rollback returns to any prior checkpoint.
- Budgets cap tokens, wall time, modification frequency, and chained application.

## New Risks Introduced by Self-Modification

### Privilege escalation via Layer E

A proposal at Layer E (settings, hooks, permissions) could relax a guardrail that other layers depend on (path guard relaxation, expanded shell scope, permission auto-approval).

Mitigation:

- Layer E is the only layer that requires per-modification operator approval. It is never applied autonomously.
- Layer E diffs must declare explicit pre/post values for every field they touch.
- The path guard, env-clear policy, and validation timeout are declared invariants of the primitive layer (F) and cannot be modified through Layer E at all.

### Tool surface widening via Layer C

A composite tool at Layer C could chain primitives to produce capability the operator did not anticipate (network egress through bash, file access outside the workspace through `..` injection, env exfiltration).

Mitigation:

- Composite tools must be expressible as a typed combination of existing primitives. The combination is validated to stay inside the path guard and within the env-clear policy of the primitive layer.
- Composite tools that introduce new outbound network surface or absolute path access are rejected at static validation.
- Validation traces include the composite tool invocations for audit.

### Prompt smuggling via Layer D

A system-prompt edit at Layer D could embed instructions that disable safety reminders or bias the agent toward unsafe operations.

Mitigation:

- Layer D diffs are shown verbatim in the audit report and applied only after the next cadence passes without operator veto.
- A small set of prompt invariants are checked at static validation (presence of the workspace guard statement, presence of the empirical-admission reminder, etc.). Diffs that remove invariants are rejected.

### Gaming the suite

The autonomous loop could discover modifications that satisfy suite criteria through shortcuts (mutating the criterion script, exploiting brittle file-match patterns, side-stepping the actual task).

Mitigation:

- The suite is read-only for the system. The criterion command runs with `env_clear` and is not writable by the agent.
- Anomaly heuristics in audit flag suspicious patterns: very large improvements on a single task, modifications affecting only one task, modifications that change exit codes without changing semantics.
- Off-suite probes (`greco eval probe <task>`) test against tasks not in the suite to catch suite-specific shortcuts.

### Rollback residue

A reverted modification leaves side effects outside the checkpoint (artifacts written to disk during validation, environment mutations).

Mitigation:

- Modifications must be expressible as typed diffs (add tool, edit prompt, edit setting, add subagent). Free-form patches forbidden.
- Validation occurs in temp directories, never in the live workspace.
- Checkpoint restore validates post-state against the recorded baseline and aborts if there is residue.

### Cascade attack

A sequence of small, individually safe modifications composes into a state the operator would have rejected as a single step.

Mitigation:

- `max_chained_modifications` caps the depth of stacked applications before mandatory audit.
- Lineage fields make cascade chains visible in audit reports.
- Cumulative diff against a chosen baseline checkpoint is part of the audit report.

### Cost explosion

Autonomous validation against the suite is the largest cost driver of the system. A misconfigured threshold or runaway proposal loop could consume budget without producing useful modifications.

Mitigation:

- `max_tokens_per_window`, `max_tokens_per_validation`, and `max_wall_seconds_per_validation` are hard, enforced at runtime.
- `early_stop_on_first_regression` aborts validation when the first suite task regresses beyond tolerance.
- Budget exhaustion freezes the autonomous loop until the next operator audit.

## Known Gaps

- v0 does not provide OS-level sandboxing.
- v0 does not yet enforce a write allowlist beyond path traversal checks.
- v0 does not yet verify generated composite tools through container or sandbox isolation; validation isolation is limited to temp directories and env-clear.
- v0 trusts the operator's evaluation suite. A malicious suite could be a vector for arbitrary code execution since criterion commands run locally. Suites should be reviewed before use.
- v0 does not signature-verify or cryptographically hash modification archives.

## Operator Guidance

- Use Greco first on non-critical repositories.
- Keep normal Codex/Claude/Cursor sandboxing enabled when using those hosts around Greco.
- Rotate any API key that has been pasted into chat before making the repository public.
- Review the audit report on cadence. Do not skip cadences.
- Audit the suite before adding new tasks; criterion commands run locally with bounded but real privileges.
- When in doubt about a modification, revert. The cost of rollback is one command. The cost of a quiet drift discovered late is much higher.
