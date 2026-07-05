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

### 0.5.0-alpha.1 — Proposal pass with manual application (Phase 2)

- Filesystem-first `modification` registry with proposed, validated, active, rejected, and retired states.
- Proposal pass over aggregate trace friction.
- Operator commands for the full modification lifecycle, manual application only.
- Layer A (cached procedures) active and reversible; Layer S1 manifests inspectable but not runtime-critical.
- Active Layer A procedures loaded into the runtime prompt under size limits.
- Audit and TUI snapshots show lifecycle state and current active procedures.

Decision gate: proposal precision and at least one applied modification moves the suite.

### 0.6.0-alpha.1 — Autonomous loop on A and S1 (Phase 3, v0 acceptance gate)

- Budget enforcement.
- Threshold logic.
- Scheduler-driven proposal-validation-application loop.
- Freeze caps and rollback.
- Extended audit reports.
- Operator commands: `greco loop run`, `greco loop status`, `greco loop freeze`, `greco loop unfreeze`.

Decision gate: one audit window of autonomous operation shows measurable aggregate friction reduction. This is the v0 acceptance gate. If it fails, the project closes per RFC Appendix B.

### 0.6.1-alpha.1 — Comparative admission patch (Phase 3)

- Loop decisions persist baseline-vs-candidate comparison artifacts under `.greco/state/comparisons/`.
- Threshold logic gates apply mode through objective-success deltas and protected regression tolerance instead of pass/fail validation alone.
- Duplicate equivalent proposals are rejected or reused so active/pending procedure payloads do not multiply silently.
- Audit and TUI snapshots expose comparison outcomes, primary improvement, maximum regression, and decision reasons.

### 0.7.0-alpha.1 — Phase 3 acceptance gate

- Public command: `greco loop gate --since <window> [--json]`.
- Deterministic verdicts: `pass`, `fail`, or `needs_more_data`.
- Gate evidence summarizes decisions by kind, comparison outcomes, budget consumption, protected regression signals, active duplicate health, and the best primary metric delta.
- The gate does not pass from wall-time-only Pareto movement; it requires applied comparative evidence with primary-metric improvement.

### Current unversioned — Harness-benefit correction

- Document the arXiv:2605.30621 correction: separate cheap evolver work from solver harness-benefit.
- Add activation and adherence signal fields to traces, audit reports, modification friction sources, and the Phase 3 gate.
- Restore the missing recalibration document cited by the agent contract.
- Run candidate validation in sandbox homes with the proposed modification activated before admission.
- Keep MLX and local open-weight solver work isolated in a local model lab until candidates pass activation, adherence, objective-success, and operator-cost checks.

### Done — A suite a modification can move (WS1 + WS2, 2026-07-05)

- WS1 shipped: five fixtures under `fixtures/eval-suite/` with deterministic
  criteria, versioned-suite loading in `greco eval` (commit `86df911`;
  implemented by a sandboxed executor run, committed by the operator session).
- WS2 shipped: three-run baseline per task under a clean `GRECO_HOME`
  (`docs/operations/ws2-baseline.md`, commit `9f17a7e`). Result: **1/5 tasks
  admitted** (`t3-edit-discipline`, deterministic retracements). The other
  four showed zero target friction: the frontier solver already exhibits the
  desired behavior at direct-fixture scale. WS2's own >=4 admission target
  was missed and is recorded as such — the admission rule worked; the fixture
  calibration is the finding.

### Next — The WS3 fork (decision point, operator's call)

Two non-exclusive paths, with one honesty rule binding both:

- **Path A — narrow but complete:** run WS3/WS4 end to end on `t3` alone
  (proposal pass over its baseline traces → `loop run --with-solver` →
  `loop gate`). Cheap, exercises the entire machine on real data, and gives
  the gate its first non-empty verdict — over a one-cluster base, stated as
  such.
- **Path B — widen the base:** harden fixtures (indirection, distractors,
  longer horizons, realistic project shapes) so target frictions can appear.
  **Re-registration rule:** every hardened fixture gets its movability
  hypothesis re-written and committed BEFORE re-baselining — the experiment
  must not tune tasks until answers look good.
- Recommendation on record: A first (days), B in parallel where cheap; if A
  fails on `t3` AND hardened fixtures stay friction-dry, the honest-closure
  clause applies with the `What I learned` document — and that document now
  has a real finding to carry: where frontier solvers no longer need harness
  help at small scale.

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
