# Phase 3 completion — a suite a modification can actually move

Status: plan, not started. Owner: operator (Petru). Executor: Codex, in ordered
workstreams, under this repository's local agent contract and commit
discipline.

## Why this phase exists

The governance machinery is built and exercised: typed layered modifications,
sandboxed validation, comparative admission (`primary_improvement_ppm`), the
Phase 3 gate, freeze and rollback. But the README states the honest gap: with
the current always-pass eval suite the measured delta is zero, so the loop
correctly applies nothing. The experiment — can the harness improve itself
measurably — is runnable but unanswered.

The recalibration doc already names the consequence: **the evaluation suite is
the real product surface.** This phase builds the missing organ and runs the
experiment. Nothing else. Explicitly out of scope: new governance features,
higher layers (B–E, S2–S3), MLX/local-model work outside the lab, TUI work.

The external correction this phase encodes: the strongest available signal for
suite design is **real operator friction**. The operator's production coding environment already measures, nightly and
deterministically, which frictions actually occur in daily work: concurrent-edit failures ("file modified since read"),
edit-without-read protocol errors, re-reads of hot files, indiscriminate
staging (`git add -A`), repeated error clusters. These are not invented
benchmark tasks; they are the operator's measured pain. The suite tasks below
are designed so each friction has a natural place to appear — and a plausible
Layer A / S1 modification has a real chance to move it.

## Design constraints (inherited, non-negotiable)

- Suite lives under `.greco/eval/`, **read-only for the system** (local agent contract).
- Criteria are deterministic: command exit codes and file matches. No
  model-judged scores anywhere (recalibration doc).
- Criterion scripts run with `env_clear`, bounded timeout, workspace guard.
- Adherence evidence is emitted deterministically by criterion scripts
  (`harness_adherence_check` hits/misses), never inferred by a model.
- A task is only admitted to the suite if its baseline friction is non-zero
  and stable (see WS2) — a task nothing can move is dead weight.

## The five candidate tasks

Each task = fixture project + task prompt + criterion script + friction
profile + a written *movability hypothesis* (the Layer A/S1 modification that
should move it). The hypothesis is written BEFORE baselining, so the
experiment cannot quietly redefine success afterwards.

| # | Task (fixture + ask) | Objective criterion | Friction it exercises | Movability hypothesis |
|---|---|---|---|---|
| T1 | `conventions/`: small Rust crate with a `CONVENTIONS.md` (naming, module layout, error style). Ask: add a small feature. | Tests pass + a lint script greps the diff for convention compliance; each rule check emits an adherence hit/miss. | Avoidable prompts, adherence misses, retracements. | Layer A cached procedure: digest of `CONVENTIONS.md` loaded into the prompt → adherence hits rise, avoidable re-asks fall. |
| T2 | `read-economy/`: repo with one large generated file plus a small accurate `INDEX.md`. Ask: answer/change something whose facts live in the index. | Correct file+line change verified by file match. | Re-reads, wasted read tokens, turns. | Layer A procedure: "consult `INDEX.md` before opening generated files" → reads and turns fall. |
| T3 | `edit-discipline/`: multi-file refactor where two edits touch the same file region (tempts blind sequential edits). | Build + tests pass; trace counter for edit-conflict / edit-without-read failures. | Repeated tool errors, retracements. | Layer A procedure: "re-read a file immediately before editing it after any prior tool activity on it" → error counters fall. |
| T4 | `staging/`: task ends in a commit; fixture contains pre-existing dirty files unrelated to the task. | Criterion checks the commit contains ONLY the task's paths (file match on `git show --stat`). | Objective failure (over-staging), retracements. | Layer A procedure: enumerate-then-stage-explicit-paths → objective success rises. |
| T5 | `recovery/`: fixture with a failing build whose fix is documented in `TROUBLESHOOTING.md`. Ask: make the build green. | Build exits 0 within budget. | Repeated identical errors, missing-tool flailing, turns. | S1 subagent prompt (or Layer A procedure) pointing the recovery flow at the troubleshooting index → repeated-error count falls. |

Naming note for Codex: keep task ids short (`t1-conventions` … `t5-recovery`);
fixtures are self-contained mini-projects committed under a `fixtures/` area
the suite copies from (the live `.greco/eval/` stays out of git per secret and
state discipline — follow the existing layout conventions).

## Workstreams (ordered; each ends in a commit group)

### WS1 — Author fixtures and criteria

Build the five fixtures and their criterion scripts. Acceptance:
- `greco eval list` shows the five tasks; `greco eval run all` executes
  criteria deterministically twice with identical verdicts.
- Every criterion emits its objective verdict AND (T1 at minimum) deterministic
  `harness_adherence_check` evidence.
- No criterion depends on network, wall-clock time of day, or model judgment.

### WS2 — Baseline and movability admission

Run each task N=3 times with no active modifications (`greco eval solve`).
Acceptance:
- Per task: non-zero baseline friction on the profiled counters, and variance
  small enough that a 5% delta is detectable (the Phase 1 gate criterion,
  now applied per task).
- A task failing admission is fixed or dropped — documented either way in the
  baseline report. Target: ≥4 admitted tasks.
- Baseline artifacts persisted under traces/validations per trace discipline.

### WS3 — Proposal pass over real baseline traces

Run `greco propose` over the baseline windows. Acceptance:
- At least one typed Layer A/S1 proposal per admitted friction cluster, each
  with manifest, lineage, and target metric matching the movability
  hypothesis table above.
- Junk or duplicate proposals are rejected and the rejection is archived —
  proposal precision is reported, not hidden.

### WS4 — Run the experiment

`greco loop run --with-solver` across enough windows to give the gate real
evidence, within existing budgets. Acceptance:
- `greco loop gate --since <window> --json` returns a verdict on REAL data:
  `pass` (some modification cleared comparative admission and aggregate
  friction fell) or `fail` — not `needs_more_data` from an empty suite.
- Every applied modification has its comparison artifact under
  `state/comparisons/`, and rollback is demonstrated once on a live applied
  modification (evidence in the audit).

### WS5 — Verdict and disposition

Write the audit-window report and the decision:
- Gate `pass` → Phase 4 (higher layers under audit) unlocks; README's
  "not yet" section updates to measured reality.
- Gate `fail` → the honest-closure clause executes as designed: a
  `What I learned` document, README updated, repository archived with the gate
  report. This outcome is a valid, publishable result — the project was built
  to be falsifiable.

## Guardrails for the executor

- Do not touch the gate, thresholds, or comparison logic to make results
  admissible — the gate can only get stricter (existing invariant).
- Do not add governance features, layers, or provider work in this phase.
- The evolver stays cheap and swappable (arXiv:2605.30621 correction); spend
  capability budget on the solver runs and the suite, not on proposal
  cleverness.
- Every claim in reports must trace to `.greco/` artifacts (the pre-claim
  verification rule of the local agent contract).
- Respect the repository's commit discipline: semantic groups, required
  commit trailers, local checks passing before committing.

## Relationship to the operator's production harness

One-way street, by design. Friction *categories* observed in the operator's
production harness inform fixture design here; nothing from this repo writes
back into the production harness, and no production-harness code or config is
imported. The two systems share ideas (risk layers, evidence-gated admission)
and friction taxonomy — never state.

## Addendum — WS2 outcome (2026-07-05)

WS1 and WS2 are complete; see `ws2-baseline.md` for the full admission table.
Outcome: 1/5 tasks admitted (target was >=4; missed and recorded). The plan's
workstreams WS3-WS5 remain valid but now start from the WS3 fork recorded in
`roadmap.md` (narrow t3 run and/or fixture hardening). Binding rule added to
this plan: hardened fixtures re-register their movability hypotheses in
writing, committed BEFORE any re-baseline run. Pre-registration is one-way;
this addendum section is append-only, like the rest of the plan after first
commit.
