# Critical Analysis

This document answers the RFC's critical questions under the recalibrated axis. The skill-axis answers from the v0.3-alpha review are kept at the bottom as historical record.

## Do any blind decisions fail?

The original RFC blindados (Section 4) remain in force with one substantive amendment.

1. **"Skills are the evolutionary unit" no longer holds.** Implementation closed the loop, then revealed that skill admission was self-referential and the catalog had no measurable reuse channel. The recalibration replaces the evolutionary unit with the *harness modification* spanning layered, typed, reversible changes to the control plane around the model. See `recalibration.md`.

2. **"No provider abstraction" is unchanged.** The narrow `ModelProvider` trait with one OpenAI implementation survives.

3. **Crate naming** is settled: product name **Greco**, binary `greco`, future package `greco-cli`.

The rest of the RFC blindados persist:

- Rust as the base.
- Direct OpenAI HTTP.
- Empirical admission as inviolable filter (now applied to modifications, not skills).
- Persistent archive between sessions; rejected and retired variants survive.
- Terminal-first, snapshot TUI before widgets.
- Architectural transparency.

## Is there exact precedent?

Not exactly, but the references are different now.

Under the skill axis the closest analogs were Pi (skill accumulation without empirical filter) and DGM (whole-agent evolution with archive discipline). Under the recalibrated axis the closer reference points are:

- **ACE (Agentic Context Engineering)** for the idea that the surrounding context-shape, not the model, is the evolution target.
- **GEPA** for Pareto retention when multiple metrics trade off and for reflective proposal from execution traces.
- **MemSkill / Mem2Evolve** for the loop of accumulated experience producing curated assets, with downstream task feedback.
- **DGM** still for archive lineage and empirical selection, but at modification scale, not whole-agent scale.

What is distinctive in Greco:

- The unit is the harness modification, typed and layered. Most prior work evolves a single dimension (prompts, context, agent code).
- The operator is in the loop as designer and aggregate auditor, not as per-proposal notary.
- The system is local, single-operator, terminal-first; the governance model differs structurally from hosted assistants that cannot afford autonomous self-modification.

## Most robust validation criterion

The recalibrated criterion is layered as before, but anchored on the suite, not on the proposal:

1. **Static validity.** Manifest parses, diff is typed, target layer permits the change, dependencies declared.
2. **Sandboxed pre-application check.** The modification is applied in a temporary harness state, not the live state. Workspace mutations during validation occur in temp dirs.
3. **Task fitness against the suite.** The same suite is run with and without the modification. Friction signals are computed deterministically from traces. Objective criteria (command exit codes, file matches) are the only ground truth.
4. **Regression guard.** The modification cannot regress any metric beyond the declared `regression_tolerance`. Independent validation runs (default 2) must agree.
5. **Trace persistence.** Every pass/fail decision stores enough evidence to reproduce or audit the outcome.

Human review exists, but only at audit cadence on aggregate behavior. Model judgment of "this looks good" is not admissible at any stage.

## Persistence model

Filesystem-first, same as the alpha cycle, with a richer layout:

```text
.greco/
  catalog/
    proposed/<modification-id>/
    validated/<modification-id>/
    active/<modification-id>/
    rejected/<modification-id>/
    retired/<modification-id>/
  traces/
    sessions/
    proposals/
    validations/
    audits/
  eval/                  read-only for the system
    <task-id>/
  subagents/
    <name>/
  state/
    harness-checkpoints/
    budgets.json
    thresholds.json
  audit/
    <window-end>.md
    <window-end>.json
```

Why filesystem first remains correct:

- Inspectable by the operator and by the model.
- Audit reports are plain markdown; machine-readable JSON sits next to them.
- Commits and selective diffs work without a schema layer.
- SQLite can come later if catalog scale demands.

## Modification invocation model

A *modification* is not invoked. It is *applied*. Once applied, the modification is part of the live harness and the agent uses it like any other harness element.

Invocation models considered and rejected:

- *Runtime hot-swap* of harness components without checkpoint: too much surface for rollback discipline.
- *Plugin loading at session start*: acceptable for v0; this is essentially how the harness loads its current state every session.
- *FFI extension points*: too much unsafe surface for v0.
- *Dynamic Rust loading*: requires recompilation or unsafe linking; rejected.

The chosen model is *load current harness state at session start*. Modifications that change the system prompt, tool descriptions, settings, subagent definitions, or cached procedures take effect on the next session. Validation runs apply the candidate in a temp harness state, run the suite, and discard the temp state.

## Tool calling versus structured outputs

Same split as the alpha cycle.

- Tool calling for primitives (`read`, `write`, `edit`, `bash`) and any Layer C composite tool the catalog activates.
- Structured outputs for modification proposals, audit reports, and validation summaries.

The operator never sees free-form model claims about "what improved". They see deterministic deltas computed from traces, rendered in audit markdown.

## Operator surface

The operator interacts with Greco at three frequencies:

- *During sessions*: regular agent use via `greco ask`. Optional in-session signaling (`greco friction <tag>`) to mark events the deterministic instrumentation might miss. Out of v0 scope unless trivial.
- *On cadence (audit)*: review the latest audit report. Decide on Layer E proposals if any. Adjust suite or budgets if needed.
- *Rare configuration*: edit the suite, adjust budgets and thresholds, define a new subagent baseline.

No per-proposal approval at any phase from Phase 3 onward except for Layer E.

## Honest disposition to close

The RFC Section 11 and Appendix B clause persists: if the hypothesis does not stand, the project closes. The recalibration adds explicit decision gates at the end of Phase 1, Phase 2, and Phase 3. Each gate has a structural failure mode (noisy signals, junk proposals, no measurable aggregate improvement). Reaching any gate without passing it triggers an honest closure with a final `What I learned` document.

## Historical: alpha skill cycle answers (for the record)

- "No provider abstraction" was already amended in the alpha review to permit a narrow trait with one implementation. That amendment carries forward.
- "Cargo install kappa" was already replaced by `greco-cli` with binary `greco`. Unchanged.
- "Pi-style local skill accumulation with empirical admission" was the alpha thesis. Closed by the recalibration; documented above.
- "Filesystem-first archive" was correct at alpha and remains correct at v0.4.
- "Subprocess invocation for skills" no longer load-bearing; modifications are not invoked, they are applied.
