# Critical Analysis

This document answers the RFC's required critical questions after the current-source review.

## Do Any Blind Decisions Fail?

Mostly no, but two decisions need refinement.

1. **"No provider abstraction" is superseded by operator instruction.** The RFC wanted OpenAI only and no cross-provider abstraction. The operator explicitly requested an abstraction that allows future providers. Greco should satisfy that with a small `Provider` trait and one OpenAI implementation. It should not build a broad compatibility layer or normalize provider-specific semantics prematurely.

2. **"cargo install kappa" is not viable.** Both `kappa` and `greco` are already occupied on crates.io. Greco should reserve the binary name `greco` but use `greco-cli` as the package name when publishing.

The rest of the RFC survives:

- Rust remains the right base language.
- Direct OpenAI HTTP remains defensible.
- Skills are the right v0 evolution unit.
- Empirical validation must gate active catalog admission.
- Failed candidates should remain archived.

## Is There Exact Precedent?

Not exactly.

Pi provides minimal local skill accumulation but not empirical evolution. DGM provides empirical archive evolution but at whole-agent/code-variant scale. DSPy, GEPA, TextGrad, Reflexion, ACE, APIGen, and TOUCAN operate on prompts, context, trajectories, datasets, or model-facing programs rather than a local coding-agent skill catalog with active/passive admission.

Greco's novelty is the scope restriction: evolve skills, not the harness, prompt stack, model weights, or full agent loop.

## Most Robust Skill Validation Criterion

The v0 validator should be layered:

1. **Static validity:** manifest parses, command exists, script is executable or Rust package builds, declared inputs are sane.
2. **Sandboxed dry run:** candidate runs in an isolated temp workspace with bounded time, env, stdout/stderr, and exit status capture.
3. **Task fitness:** candidate is exercised against one or more validation tasks with explicit expected artifacts, command outputs, or tests.
4. **Regression guard:** candidate cannot mutate the Greco catalog or project outside its assigned temp root during validation.
5. **Trace persistence:** every pass/fail decision stores enough evidence to reproduce or audit the outcome.

Human review is useful but opt-in. Another model can critique a candidate, but model judgment cannot promote a skill without executable evidence.

## Persistence Model

Use a filesystem-first archive for v0:

```text
.greco/
  catalog/
    active/<skill-id>/manifest.json
    active/<skill-id>/bin-or-script
    rejected/<candidate-id>/
    retired/<skill-id>/
  traces/
    validation/<trace-id>.jsonl
    sessions/<session-id>.jsonl
  scores.json
```

Why filesystem first:

- It matches Pi-style local ownership.
- It is transparent to the model and human operator.
- It can be committed or inspected selectively.
- It avoids SQLite schema churn while design is still fluid.

SQLite is likely useful after the catalog reaches hundreds of skills, but v0 should keep the archive inspectable and text-native.

## Skill Invocation Model

Use subprocess invocation for v0.

Rejected alternatives:

- Dynamic plugin loading: raises ABI and safety complexity.
- FFI: too much unsafe surface for v0.
- In-process Rust trait plugins: requires recompiling or linking the harness for every skill.
- MCP: explicitly outside the RFC v0 scope and too broad for the thesis.

Subprocess skills keep the harness small, allow scripts or compiled binaries, and create a natural boundary for timeout, stdin/stdout, env, and working directory controls.

## Tool Calling Versus Structured Outputs

Use both, with different responsibilities.

- Tool calling is for primitive actions: `read`, `write`, `edit`, `bash`, and eventually `invoke_skill`.
- Structured outputs are for internal proposals: skill manifests, validation plans, mutation reports, and final machine-readable decisions.

This follows OpenAI's current Responses guidance and keeps action execution separate from artifact generation.
