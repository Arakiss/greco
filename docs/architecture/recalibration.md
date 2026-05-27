# Recalibration: From Skill Catalog to Self-Improving Harness

Date: 2026-05-27
Supersedes the v0 axis declared in the Kappa RFC and the design assumption of the v0.1-v0.3-alpha cycle.

## Why this document exists

Greco v0.1 through v0.3-alpha implemented the original Kappa RFC thesis: the evolutionary unit is the local skill catalog. The harness can now propose script skills through Structured Outputs, persist candidates, validate them empirically, promote passing candidates, reject failures, update scores, and retain JSONL traces. That loop closes.

After the loop closed, a critical review forced a question the RFC anticipated but did not fully answer:

> *Improving in what sense?*

A skill catalog can grow, version, and prune. None of those is improvement unless tied to a measurable objective on tasks the operator actually performs. The implemented loop did not have that anchor. Validation was self-referential: each proposal carried its own validation command, and admission only confirmed that the script-and-check pair was internally consistent. No task fitness, no reuse signal, no measurable delta between session 1 and session 100.

The skill axis was not technically broken. It was thematically misaligned with the deeper aspiration of the project: **a harness that becomes a better harness through use**.

This document records the recalibration of the project's evolutionary axis and the reasoning that produced it. It is meant as load-bearing context for any agent or operator who will implement the next phases.

## What stays from the Kappa RFC

The decisions of identity blindados in Section 4 of the RFC remain in force:

- Minimal Rust harness, readable in a single sitting.
- Direct OpenAI HTTP integration, no broad LLM wrappers.
- Tokio, reqwest with rustls, serde, serde_json as the only base dependencies.
- One narrow `ModelProvider` trait, one OpenAI implementation.
- Empirical admission as the inviolable filter.
- Persistent archive between sessions; rejected and retired variants survive.
- Terminal-first operator surface, snapshot-style TUI before widgets.
- Architectural transparency: the agent can read the code that surrounds it.
- Anti-objectives intact: no platform, no IDE integration, no marketplace, no MCP, no sub-agent runtime as a general framework.

The three-level disambiguation in RFC Section 2.1 also stays:

- Level 1 (intra-session reasoning) is not Greco's territory.
- Level 2 (the environment around a frozen model) is Greco's territory.
- Level 3 (model weights) is out of scope.

## What is replaced

The RFC chose **skills** as the Level 2 evolutionary vector (RFC Section 2.3), arguing three points:

1. Pi already implements a good skill primitive and only lacks evolution.
2. Skills have a naturally bounded validation criterion.
3. Skills are modular: a bad skill is discarded without destabilizing the loop.

After implementing the loop, each argument is either weak or non-exclusive:

1. *Proximity to Pi is not intrinsic merit.* Pi accumulates skills because its author chose minimalism, not because skills are the optimal unit.
2. *Bounded validation is not exclusive to skills.* Tools, prompt fragments, playbooks, hooks — all admit bounded validation when paired with a task and a criterion.
3. *Modularity descards only the "evolve the whole agent" vector* (DGM scale). It does not differentiate among skills, prompts, and playbook.

The original argument descards the whole-agent vector well but does not justify skills over the alternatives in coding work. Subsequent evidence reinforces the doubt:

- Skills shine where procedures are closed and repeatable (Voyager in Minecraft, RPA, deploy rituals). Coding tasks are mostly contextual rather than procedural.
- The base model improves faster than any locally curated catalog. Gains from cached procedures get eroded each release.
- Hosted ecosystems already provide skill-like primitives (Anthropic Skills, Cursor rules, CLAUDE.md). The differentiator a local harness can credibly own is not "another skill catalog".

## The new axis

Greco's evolutionary unit is the **harness modification**.

The harness is everything around the frozen model: the system prompt, the tool surface, the subagent definitions, the settings, the hooks, the cached procedures, the operator surfaces. A *harness modification* is a concrete, reversible change to one of those parts, proposed because session traces show friction the harness could reduce.

Two dimensions structure the axis:

- **Layers.** Modifications live at different depths, with different risk, frequency, and gate strictness. See `design.md` for the table.
- **Subjects.** Modifications can target the harness as a whole (system prompt, tool schemas, settings) or a specific subagent (its prompt, its toolset, its scope). Subagents are typically the cheapest and fastest place to iterate because their scope is declared and their metrics are clear.

## The human stops being a notary

The Kappa RFC implicitly assumed the operator would gate every proposal. After mapping out the loop in practice, that role is a bottleneck and an insult: humans should not spend their time rubber-stamping micro-decisions.

The recalibrated model splits the operator's role into three:

1. **Designer of the experiment** (manual, rare). Defines the evaluation suite (5-15 representative tasks), the metrics (turns, tokens, retracements, error repetition, objective success), the budgets (tokens/time per experiment, frequency caps), and the thresholds (minimum improvement to apply, hard cap before mandatory freeze).
2. **Auditor in aggregate** (periodic, scheduled). Reviews whether aggregate metrics correlate with real utility on tasks the operator cares about. Adjusts suite or budgets if drift is detected.
3. **Notary per proposal** (frequent). **Eliminated.** The system applies modifications autonomously within the operator-defined rules.

This is the operating model of any mature ML pipeline: nobody approves each gradient step. You define val set, metrics, budgets, and you audit the system in aggregate.

## The objective function

A harness modification is "an improvement" if and only if it reduces measurable friction on the evaluation suite without regressing other dimensions. Friction signals come directly from session traces Greco already writes:

- **Turns per task type.** Fewer turns to reach the same objective state.
- **Tokens per task type.** Fewer tokens consumed.
- **Repeated errors.** Same error class fewer times across sessions.
- **Retracements.** Actions undone after the fact.
- **Avoidable permission prompts.** Operator approvals on patterns that recur.
- **Missing-tool failures.** Sequences where the agent reaches for a primitive it does not have.
- **Objective success.** Did the task pass its declared criterion.

The signal is multidimensional. The system retains a Pareto frontier of modifications when they trade off against each other (GEPA-style), instead of forcing a single global score.

## Where Greco fits now

The narrow intersection Greco occupies is no longer "Pi minimalism + DGM archive at skill scale". It is:

- A **local, single-operator, terminal-first** harness.
- That **observes its own use** through traces.
- And **proposes modifications to itself** (harness + subagents).
- **Validates them empirically** against an operator-defined suite within strict budgets.
- **Applies them autonomously** when thresholds are met.
- **Archives them with lineage** for reversibility.
- And **lets the operator audit in aggregate**, not per-proposal.

Hosted assistants (Cursor, Claude Code, Codex CLI) do not do this and likely should not: their harness is a shared control plane for millions of users, where autonomous modification would be a governance disaster. A *local* harness owned by a single operator can do it under explicit budget and audit discipline.

## Implications for the v0.1-v0.3-alpha code

The implemented skill catalog code (`catalog.rs`, `validation.rs`, `proposal.rs`) is not deleted. It is retained as evidence of the alpha exploration and as scaffolding that the new model will refactor. The structural pieces that do carry forward are:

- The `ModelProvider` trait and the OpenAI Responses adapter.
- The primitive tool surface (`read`, `write`, `edit`, `bash`).
- The agent tool loop in `agent.rs`.
- The JSONL trajectory under `.greco/traces/sessions/`.
- The filesystem-first archive discipline.
- The TUI snapshot pattern.

What changes substantively:

- `catalog.rs` becomes the registry of harness modifications, not scripts.
- `validation.rs` validates modifications by running the evaluation suite with and without them.
- `proposal.rs` becomes the friction-detection and modification-proposal pass that runs over session traces.
- A new module (working name: `eval.rs`) owns the suite, metrics, budgets, and thresholds.
- A new module (working name: `audit.rs`) owns operator-facing aggregate reports.

The exact module boundaries are an implementation question for Codex. The design document specifies the contracts; the file layout can adjust.

## What the operator gets in v0 of the new model

By the end of the phased plan (`implementation-plan.md`):

- A harness whose primitive tools, subagent definitions, system prompt, and cached procedures evolve autonomously within explicit budgets.
- A suite the operator defines once and rarely changes.
- A weekly (or per-cadence) audit report showing aggregate friction trends, applied modifications, reverted modifications, and pending proposals frozen for review.
- A rollback that returns the harness to any prior checkpoint.

If the audit cadence ever shows that aggregate metrics improve on the suite but do not correlate with real utility on tasks outside the suite, the operator updates the suite. That is the only intervention loop required at scale.

## Honest disposition to close the project

The Kappa RFC included in Section 11 and Appendix B an explicit clause: if the hypothesis does not stand, the project reformulates or closes honestly. That clause now applies to the new axis too.

If, after Phase 1 of the new plan, baseline friction signals are too noisy to support measurement, the project closes. If, after Phase 3, autonomous modifications on layers A and S1 do not produce a measurable aggregate improvement on the suite, the project closes. The point of the recalibration is not to extend the project at any cost. It is to give the project a thesis that can actually be tested.
