# Greco v0 Design

## Product Statement

Greco is a terminal-first Rust coding-agent harness whose evolutionary unit is the harness itself. The model is not retrained. The agent observes its own use through session traces, proposes targeted modifications to the harness or its subagents, validates them empirically against an operator-defined evaluation suite within strict budgets, and applies or discards them autonomously. The operator does not approve per-proposal changes; the operator designs the experiment and audits aggregate behavior on a cadence.

The original v0.1-v0.3-alpha cycle implemented a skill catalog and proved its loop closes. The current axis supersedes that work.

## Architecture

```text
greco
  cli              command surface, snapshot TUI
  agent            tool loop, friction instrumentation
  provider         narrow ModelProvider trait + OpenAI Responses adapter
  tools            primitive read/write/edit/bash
  trajectory       JSONL session traces
  eval             suite, metrics, budgets, thresholds
  proposal         friction detection and modification proposals
  catalog          registry of harness modifications (active/proposed/rejected/retired)
  validation       runs the suite with/without a candidate modification
  audit            aggregate reports for the operator
  subagent         declared subagent definitions (prompt + toolset + scope)
  harness          loadable harness state: system prompt, tools, settings, hooks

.greco/
  catalog/         modification archive with lineage
  traces/          session, proposal, validation, audit JSONL
  eval/            suite definitions (read-only for the agent)
  state/           applied modifications, current harness checkpoint
  audit/           periodic reports
```

The current implementation can keep modules in a single crate. The boundaries above must be reflected in code so the eventual split into crates is mechanical.

## Levels of Modification

A *harness modification* is a concrete, reversible change to one piece of the surrounding control plane. Modifications are stratified by risk, frequency, and the strictness of the gate they require. Layers F and G are explicitly out of scope for v0.

| Layer | What changes | Frequency | Risk | Gate model |
|-------|---|---|---|---|
| **A** | Cached procedure (template for a recurring task) | High | Low | Autonomous within budgets |
| **B** | Tool description or schema (existing primitive) | Medium-high | Low | Autonomous within budgets |
| **C** | Composite tool (combination of primitives) | Medium | Medium | Autonomous, stricter thresholds |
| **D** | System prompt edit | Low-medium | Medium | Mandatory pre-application diff in audit report |
| **E** | Settings, hooks, permissions | Low | High | Frozen until next scheduled audit; operator approves explicitly |
| **S1** | Subagent prompt | Medium-high | Low-medium | Autonomous within budgets |
| **S2** | Subagent toolset | Medium | Medium | Autonomous, stricter thresholds |
| **S3** | New subagent definition | Medium | Medium | Autonomous, stricter thresholds |
| F | Implementation of primitives (`read`, `write`, `edit`, `bash`) | — | Very high | Out of scope v0 |
| G | Agent loop, harness code itself | — | DGM scale | Out of scope v0 |

The layered approach permits an honest phased rollout: Phase 3 of the implementation plan targets A and S1 only. Higher layers are unlocked when lower layers have demonstrated stable behavior.

## Subjects of Modification

Two subject types coexist:

- **Harness-wide modifications** touch the global control plane. Layers A through E.
- **Subagent-scoped modifications** touch a single declared subagent (its prompt, its toolset, its allowed operations). Layers S1 through S3.

Subagents are the cheapest terrain to iterate. Their scope is declared, their objective metrics are clear, and a regressing subagent can be deactivated without breaking the rest of the system. Variants can coexist (subagent_v1, subagent_v2) and be compared on the suite.

## The Co-Loop

The operator and the model both contribute to the improvement loop. Neither is the gate; they are co-designers of the system.

**Operator contributions:**

- Defines and maintains the evaluation suite (read-only for the system).
- Declares metrics, budgets, thresholds, and freeze caps.
- Schedules and performs the periodic audit.
- Tags friction events in real time during sessions when the system did not detect them (signals to the proposal pass).
- Reverts harmful modifications when the audit shows they regress real utility outside the suite.

**Model contributions:**

- Resolves the operator's tasks using current primitives and active subagents.
- Emits friction signals during sessions (declared as instrumentation points, not as model judgment).
- Runs the offline proposal pass over recent traces.
- Drafts candidate modifications scoped to a layer.
- Reports aggregate trends and pending proposals at audit cadence.

Per-proposal approval is not a step. The operator audits in aggregate.

## Friction Signals

Friction signals are extracted from traces by deterministic instrumentation, not by model judgment. The base set:

| Signal | Source | Direction |
|--------|--------|-----------|
| `turns_per_task` | Trajectory length per declared task | Lower is better |
| `tokens_per_task` | Token usage rolled up per task | Lower is better |
| `repeated_errors` | Same error class > N sessions in window | Lower is better |
| `retracements` | Edits or actions undone within same session | Lower is better |
| `avoidable_prompts` | Permission prompts on recurring patterns | Lower is better |
| `missing_tool_failures` | Sequences of primitives that suggest a missing composite | Lower is better |
| `objective_success` | Pass/fail of the task's declared criterion | Higher is better |

A modification must reduce one or more of these without regressing any of them beyond a declared tolerance. The system maintains a Pareto frontier when modifications trade off against each other, after GEPA's pattern.

## The Evaluation Suite

The suite is a read-only directory under `.greco/eval/`. The agent and proposal pass can read it; the system cannot modify it. Only the operator can edit suite files.

Each suite entry is:

```text
.greco/eval/<task-id>/
  task.json          declared input, declared criterion, declared task type
  artifacts/         reference outputs or test files used by the criterion
```

`task.json` schema (minimum):

```json
{
  "id": "rewrite_test_to_table_driven",
  "task_type": "refactor",
  "input": "Convert the assertions in tests/foo_test.go to a table-driven form.",
  "criterion": {
    "kind": "command",
    "command": "go test ./tests/foo_test.go",
    "expect_exit_code": 0,
    "timeout_seconds": 60
  },
  "budget": {
    "max_turns": 20,
    "max_tokens": 50000
  }
}
```

Criteria support at least `command` (run a command, expect exit code), `file_match` (file exists and matches a pattern), and `composite` (all of a list pass). The criterion is the only ground truth; the model's claim of success is irrelevant.

Suite size target: 5 to 15 tasks. Smaller is fine if representative. Larger inflates validation cost without proportional signal.

## Budgets

Budgets are non-negotiable and enforced by the harness. They protect against unbounded cost during autonomous validation.

Global budgets (per audit window, configured in `.greco/state/budgets.json`):

- `max_tokens_per_window` — hard cap of tokens spent on validation in a window.
- `max_modifications_per_window` — hard cap of applied modifications before mandatory freeze.
- `max_chained_modifications` — hard cap of modifications stacked without an intermediate audit.

Per-experiment budgets (declared per validation run):

- `max_tokens_per_validation` — tokens consumed in one validation pass.
- `max_wall_seconds_per_validation` — wall time bound.
- `early_stop_on_first_regression` — abort validation when the first suite task regresses beyond tolerance.

When a budget exhausts, the system marks the modification as `validation_inconclusive` and archives it without applying.

## Thresholds

Thresholds decide whether a candidate modification is applied, archived, or escalated to the operator at audit.

- `min_relative_improvement` — minimum delta on the primary metric to apply. Default 5%.
- `regression_tolerance` — maximum allowed regression on any other metric. Default 1%.
- `validation_runs_required` — number of independent validation passes that must agree. Default 2.
- `pareto_keep_when_uncomparable` — when a candidate improves one axis and regresses another within tolerance, keep on the frontier instead of discarding.

## The Modification Lifecycle

```text
session
  -> trajectory + friction instrumentation
  -> proposal pass over recent traces (offline, scheduled or on-demand)
  -> candidate modification archived as `proposed` with layer, target, lineage
  -> validation against suite within budgets
  -> apply (autonomous) | archive as `rejected` | escalate to audit
  -> if applied: monitor for N subsequent sessions
  -> aggregate audit report at cadence
  -> operator can rollback to any prior checkpoint
```

States in `.greco/catalog/`:

- `proposed/` — candidate, not yet validated.
- `validated/` — passed validation, awaiting application or escalation.
- `active/` — applied, currently in force.
- `rejected/` — failed validation, archived with traces and reason.
- `retired/` — was active, rolled back. Retained with lineage.

No artifact is ever deleted in v0. Lineage and reason fields are mandatory.

## Manifests

Each modification carries a manifest:

```json
{
  "id": "tool_grep_extract",
  "version": "0.1.0",
  "layer": "C",
  "subject": "harness",
  "title": "Composite tool for find + grep + extract field",
  "rationale": "Detected pattern of 3+ bash invocations per session matching find/grep/cut chain.",
  "diff": {
    "kind": "add_tool_definition",
    "schema_path": "tools/grep_extract.json",
    "implementation_path": "tools/grep_extract.sh"
  },
  "lineage": {
    "parent_id": null,
    "source_traces": [".greco/traces/sessions/2026-05-27-...jsonl"],
    "proposal_trace": ".greco/traces/proposals/2026-05-27-...jsonl",
    "validation_trace": null
  },
  "metrics_target": {
    "primary": "turns_per_task",
    "expected_delta": -0.15
  }
}
```

Diffs are typed by layer. For Layer D (system prompt edit) the diff is a textual patch. For Layer S3 (new subagent) the diff is a subagent definition file. For Layer E (settings) the diff is a structured settings patch with explicit pre/post values.

## Subagent Definitions

A subagent definition is a self-contained directory:

```text
.greco/subagents/<name>/
  manifest.json     id, version, scope, owned tools, prompt path
  prompt.md         system prompt for the subagent
  tools.json        the subset of primitives + composite tools available
```

The main agent invokes subagents through a dedicated primitive (planned `subagent` tool, out of v0 scope or part of Phase 3). Subagent traces feed back into the same trajectory format with a `subagent_id` discriminator.

## Audit Reports

Audit reports live under `.greco/audit/<window-end>.md` and are also rendered through `greco audit --since <window>`. A report contains:

- Aggregate friction trends across the window (per metric, per task type).
- Modifications applied in the window, with their pre/post deltas.
- Modifications rolled back automatically with reasons.
- Pending proposals frozen for explicit operator approval (Layer E or escalated).
- Suite coverage notes: tasks that failed structurally vs. tasks that simply did not improve.
- Budget consumption against caps.

The report is the operator's main interface to the system. It is plain markdown for human reading and a parallel JSON file for machine consumption.

## Provider Boundary

The narrow `ModelProvider` trait from v0.3-alpha persists unchanged in shape:

```rust
trait ModelProvider {
    async fn respond(&self, request: ModelRequest) -> Result<ModelResponse>;
    async fn stream(&self, request: ModelRequest) -> Result<ModelStream>;
}
```

- `OpenAiProvider` is the only implementation.
- Default model is `gpt-5.4`.
- Validation runs use the same provider and respect declared budgets.
- The core harness sees normalized text, tool calls, usage, and raw output items for stateless reasoning replay.

## Primitive Tools

The four primitives stay:

- `read` — UTF-8 read inside the workspace.
- `write` — create or replace inside the workspace.
- `edit` — bounded textual edit inside the workspace.
- `bash` — shell command with cwd, timeout, captured stdout/stderr.

The path guard (no absolute paths, no `..`), bounded env, and timeout discipline persist. New composite tools introduced through Layer C must compose these primitives; they cannot reach outside the workspace nor escape the path guard.

## TUI Direction

The plain-text snapshot pattern persists. Operator commands the system mostly through:

- `greco status [--json]`
- `greco audit --since <window>`
- `greco catalog list --state <proposed|validated|active|rejected|retired>`
- `greco modification show <id> [--diff]`
- `greco modification revert <id>`
- `greco eval list`
- `greco propose --task <text>` (manual trigger of the proposal pass)
- `greco ask --input <text>` (regular agent use)
- `greco tui --snapshot`

JSON output is always available alongside human-readable output.

## Non-Goals

- No subagent framework as a general platform (subagents are declared definitions, not extension points for third parties).
- No MCP integration.
- No web dashboard.
- No auto-modification of primitives or the harness code itself (Layers F and G).
- No prompt evolution outside the explicit Layer D mechanism with mandatory diff in audit.
- No context/playbook evolution as a separate axis (the proposal pass over traces is the substitute).
- No fine-tuning.
- No marketplace.

## Versioning

Semantic Versioning. Pre-1.0:

- Breaking CLI, trace schema, manifest schema, suite schema, or provider trait changes require a minor bump.
- Compatible fixes use patch bumps.

The alpha cycle so far (`0.1.0-alpha.1` through `0.3.0-alpha.1`) covered the skill-axis exploration. The next minor version begins the recalibrated work: `0.4.0-alpha.1` and onward correspond to Phase 1 of the implementation plan.
