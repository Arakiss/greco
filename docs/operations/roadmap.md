# Roadmap

The original alpha cycle explored the skill catalog axis. After the loop closed, the axis was reviewed and replaced. The roadmap below preserves the alpha cycle as historical context and starts the recalibrated work at `0.4.0-alpha.1`.

## Alpha skill cycle (closed)

### 0.1.0-alpha.1

- Secret-safe repo shell.
- Research and design docs.
- Rust CLI scaffold.
- OpenAI Responses request/response/SSE parser.
- Primitive tool schemas and local execution commands.
- Skill manifest parsing and validation fixtures.
- Plain-text TUI snapshot.

### 0.2.0-alpha.1

- Full Responses tool loop.
- Function-call output submission.
- Session trajectory JSONL.
- Stateless reasoning preservation via `reasoning.encrypted_content` include.
- Live smoke against OpenAI `gpt-5.4` using only ignored `.greco` artifacts.

### 0.3.0-alpha.1

- Candidate skill archive layout.
- Promotion/rejection file moves.
- Skill proposal structured output schema.
- Proposal, validation, promotion, and rejection JSONL traces.
- Score file with attempts, passes, failures, and lifecycle timestamps.
- CLI lifecycle commands for create, validate, promote, reject, and list.
- Live OpenAI proposal smoke using Responses `text.format`.

Outcome: loop closes. Critical review concludes the skill axis does not test the deeper aspiration. Recalibration follows.

## Recalibrated cycle (active)

### 0.4.0-alpha.1 — Instrumentation and baseline (Phase 1)

- `eval` module: suite loader, criterion runner, budget skeleton.
- Five baseline suite tasks on a real local project.
- Friction instrumentation in the trajectory.
- `audit` module with markdown + JSON report.
- Operator commands: `greco eval list`, `greco eval run`, `greco audit --since`.

Decision gate: baseline friction signals stable enough for 5% delta detection.

### Next alpha — Proposal pass with manual application (Phase 2)

- Rewritten `proposal`, `catalog`, `validation` modules.
- Subagent loader.
- Operator commands for the full modification lifecycle, manual application only.
- Layer A (cached procedures) and Layer S1 (subagent prompt edits) only.

Decision gate: proposal precision and at least one applied modification moves the suite.

### Later alpha — Autonomous loop on A and S1 (Phase 3, v0 acceptance gate)

- Budget enforcement.
- Threshold logic.
- Scheduler-driven proposal-validation-application loop.
- Freeze caps and rollback.
- Extended audit reports.

Decision gate: one audit window of autonomous operation shows measurable aggregate friction reduction. This is the v0 acceptance gate. If it fails, the project closes per RFC Appendix B.

### Later alpha — Higher layers under audit (Phase 4)

- Layers B, C, S2, S3 autonomous within stricter thresholds.
- Layer D (system prompt) with mandatory pre-application diff in audit.
- Layer E (settings, hooks, permissions) explicit operator approval per modification.

### Later alpha — Second-project validation and bundle (Phase 5)

- Replication on a second operator project with its own suite.
- Catalog lint.
- `greco report bundle --redact`.

## Beta Gate

`0.9.0-beta.1` is conditional on:

- Phase 3 acceptance gate passed.
- A second project shows non-zero friction reduction.
- README reflects measured behavior.
- Secret scans and CI green.

## Versioning rules

Pre-1.0 semantic versioning:

- A numbered phase is not automatically a minor version.
- Version bumps happen only at a release gate, after the diff has a defensible public contract.
- Breaking or materially new CLI, trace schema, manifest schema, suite schema, provider trait, or archive lifecycle behavior can justify a minor pre-release.
- Compatible fixes use patch pre-releases.
- Experimental checkpoints that are not ready to be a release stay unversioned in `main`.
- Future roadmap entries name phases, not promised version numbers. The release commit chooses the exact SemVer and records why in the commit trailers.

## Honest closure clause

At every decision gate (Phase 1 end, Phase 2 end, Phase 3 end), the project closes if the gate fails. Closure is recorded as a final `What I learned` document in `docs/` and the repository is archived with the gate report.
