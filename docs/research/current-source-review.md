# Current Source Review

Date: 2026-05-27

This document records the research pass that gates Greco's v0 design. The source RFC used "Kappa" as the working name; this repository uses **Greco** as the product name and keeps Kappa as historical project context only.

## OpenAI API State

Primary sources:

- OpenAI Responses migration guide: https://developers.openai.com/api/docs/guides/migrate-to-responses
- OpenAI function calling guide: https://developers.openai.com/api/docs/guides/function-calling
- OpenAI tool search guide: https://developers.openai.com/api/docs/guides/tools-tool-search
- OpenAI latest model guide: https://developers.openai.com/api/docs/guides/latest-model
- OpenAPI spec endpoint: `POST /v1/responses`

Findings:

- Responses is the recommended API for new OpenAI agentic work. Chat Completions remains supported, but the current guidance points new projects toward Responses for tools, structured outputs, reasoning, and multi-turn state.
- `gpt-5.4` is a valid OpenAI model target in the current OpenAPI examples and model docs. It supports `tool_search`, and OpenAI docs state that only `gpt-5.4` and later models support that feature.
- Streaming Responses use server-sent events. Important event types for v0 are `response.output_text.delta`, `response.function_call_arguments.delta`, `response.function_call_arguments.done`, and `response.completed`.
- Responses function tools are strict by default. Greco should generate function schemas with explicit required fields and `additionalProperties: false`.
- For many tools, OpenAI recommends keeping the initial tool set small and using tool search or namespaces when the tool surface grows.

Decision impact:

- Greco should implement Responses first, not Chat Completions.
- Greco should set `store: false` by default for local coding-agent privacy unless the operator explicitly opts into hosted state.
- Greco should expose only the four primitive tools initially (`read`, `write`, `edit`, `bash`) and defer skill catalog surfacing until skill selection is useful.
- Greco should keep a provider trait, but the only v0 implementation should be OpenAI. The trait exists to prevent OpenAI-specific types from leaking across the harness, not to pretend every provider has identical semantics.

## Coding Agent Harnesses

Primary sources:

- Pi: https://github.com/earendil-works/pi
- Pi mono / author experiments: https://github.com/badlogic/pi-mono
- Mario Zechner Pi writeup: https://mariozechner.at/posts/2025-10-18-pi/
- OpenAI Codex CLI: https://github.com/openai/codex
- Darwin Godel Machine paper: https://arxiv.org/abs/2505.22954
- DGM repository: https://github.com/jennyzzt/dgm
- Aider: https://github.com/Aider-AI/aider
- OpenHands: https://github.com/All-Hands-AI/OpenHands
- SWE-agent: https://github.com/SWE-agent/SWE-agent

| System | Language / core | Extensibility | Validation | Persistence | Greco lesson |
| --- | --- | --- | --- | --- | --- |
| Pi | Minimal coding agent, intentionally small surface | Skills/extensions built by the agent | Mostly operator/task outcome driven | Local skill accumulation | Inherit the small primitive surface and local skill ownership; add empirical admission gates. |
| Codex CLI | Rust-first CLI with sandboxing, approvals, UI/core separation | Tool and protocol surfaces around a core agent runtime | Strong local verification through commands and sandbox outcomes | Session/config surfaces, not an evolving skill archive | Inherit Rust, terminal-first shape, core/UI separation, and explicit safety boundaries. Reject heavyweight host integration in v0. |
| DGM | Research system around self-improving coding agents | Evolves agent code variants | Benchmark fitness, archive comparison, sandboxed execution | Open-ended archive of variants | Inherit archive and empirical selection. Reject evolving the whole harness in v0. |
| Aider | Mature coding assistant with repository editing loop | Model/tool/config integration | Tests and user command loops | Repo context and chat history | Use as evidence that test-command feedback is the practical coding-agent fitness signal. |
| OpenHands | Full software-agent platform | Runtime, sandbox, browser, integrations | Task and benchmark oriented | Workspace/runtime state | Reject platform scope for v0; study sandbox ergonomics later. |
| SWE-agent | Benchmark-oriented SWE task agent | Agent config and command hooks | SWE-bench style task evaluation | Trajectory/log artifacts | Borrow task trace discipline and benchmark harness ideas. |

Design conclusion:

Greco occupies a narrow intersection: Pi's local minimalism plus DGM's archive and fitness discipline, restricted to the skill catalog. It should not become a platform, IDE, or multi-agent runtime in v0.

## Self-Improvement Frameworks

Primary sources:

- DSPy: https://github.com/stanfordnlp/dspy and https://dspy.ai/
- GEPA: https://github.com/gepa-ai/gepa
- GEPA paper: https://arxiv.org/abs/2507.19457
- TextGrad: https://github.com/zou-group/textgrad
- TextGrad paper: https://www.nature.com/articles/s41586-025-08661-4
- ACE, Agentic Context Engineering: https://arxiv.org/abs/2510.04618
- Reflexion: https://arxiv.org/abs/2303.11366
- APIGen: https://arxiv.org/abs/2406.18518
- APIGen-MT: https://arxiv.org/abs/2501.13145
- TOUCAN: https://arxiv.org/abs/2502.13720
- MemSkill: https://arxiv.org/abs/2602.02474
- Mem2Evolve: https://arxiv.org/abs/2604.10923

| Framework | What evolves | Validation | Persistence | Applicability to Greco |
| --- | --- | --- | --- | --- |
| DSPy / MIPRO | Prompt programs and module instructions | Metric-driven optimization over examples | Compiled prompt/program artifacts | Useful for future prompt evolution, not v0 skill admission. |
| GEPA | Reflective prompt/program candidates | Pareto-aware metric feedback from traces | Candidate pools and optimization history | Strong inspiration for trace-aware skill mutation after v0. |
| TextGrad | Text prompts/solutions via textual feedback gradients | Loss/feedback functions | Optimization trajectory | Useful as a validator feedback mechanism, not as a core dependency. |
| ACE | Context/playbook memory | Downstream task performance | Structured context store | Future vector; v0 should avoid mixing context evolution with skill evolution. |
| Reflexion | Verbal self-reflection memory | Task feedback and repeated attempts | Reflection memory | Useful for validation trace annotations; insufficient alone for active catalog admission. |
| APIGen / APIGen-MT / TOUCAN | Synthetic tool-use trajectories and API-use data | Execution, semantic checks, reviewer committees, and benchmark filters | Generated datasets | Useful later to build synthetic skill validation sets. The transferable idea is blueprint-first validation, not model training. |
| MemSkill / Mem2Evolve | Memory operations, experience memory, and asset/tool memory | Downstream task improvement and hard-case review loops | Evolving memory plus tool/asset stores | Closest recent support for Greco's thesis: skills should be generated from accumulated experience, and promoted skills should create new experience traces. |

Design conclusion:

The strongest transferable pattern is not a specific dependency. It is the loop:

1. Generate or mutate a candidate.
2. Run it against explicit tasks.
3. Keep the trace.
4. Promote only on empirical evidence.
5. Preserve failures as archive material.

Greco should implement that loop with local files and process execution before adopting optimizer frameworks. GEPA-style Pareto retention is a design hint: when two variants pass different task clusters, Greco should not force a single global winner too early.

## Rust LLM Ecosystem

Primary sources:

- async-openai: https://github.com/64bit/async-openai
- rig: https://github.com/0xPlaygrounds/rig
- swiftide: https://github.com/bosun-ai/swiftide
- llm-chain: https://github.com/sobelio/llm-chain

Finding:

These libraries are useful for ordinary application development, but Greco's research question requires the harness to be visible and modifiable by the agent. Direct HTTP+JSON+SSE keeps the critical surface small and inspectable. The exclusion of broad LLM wrappers remains defensible for v0.

Allowed dependency exception:

`reqwest` already owns HTTP, TLS integration, and streaming body access. Reimplementing those from `hyper` in v0 would expand code and risk without improving the evolutionary thesis.

## Name Availability

Checks run locally:

- `crates.io` has an existing `kappa` crate.
- `crates.io` has an existing `greco` crate.
- `greco-cli` did not return from the crates index check.
- `gh repo view Arakiss/greco` returned no visible repository.

Decision:

- Product name: **Greco**
- Binary name: `greco`
- Crate/package name for future publishing: `greco-cli`
- Historical RFC name: Kappa

Rationale:

Greco is more public-friendly than Kappa and avoids the most obvious Greek-letter/statistics/Twitch collisions. Since the exact crate name is already occupied, the future install path should be `cargo install greco-cli`, which can still install a `greco` binary.
