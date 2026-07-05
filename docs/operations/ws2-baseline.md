# WS2 Baseline and Movability Admission

Status: completed on 2026-07-05. Scope: Workstream WS2 only from
`docs/operations/phase3-movable-suite-plan.md`.

## Method

Baseline runs used a clean `GRECO_HOME` at
`.greco/ws2-baseline-20260705T160508Z` so no active Layer A procedures were
loaded. The committed fixture suite under `fixtures/eval-suite/` was used. Each
candidate task was solved three times with `greco eval solve <task-id> --json`.

Admission rule used for this report:

- A task needs objective solve evidence from all three runs.
- A profiled friction counter must have a non-zero baseline mean.
- The non-zero profiled counter must be stable enough for a 5% delta check; in
  this report that means observed sample coefficient of variation at or below
  5%, or zero variance on the profiled counter.
- Generic turn/token spend is reported, but it does not admit a task when the
  task's written movability hypothesis targets a different friction that stayed
  at zero.

Evidence artifacts:

- Aggregate WS2 validation trace:
  `.greco/ws2-baseline-20260705T160508Z/traces/validations/ws2-baseline-20260705T160508Z.jsonl`
- Per-run solve reports:
  `.greco/ws2-baseline-20260705T160508Z/solve-reports/*.json`
- Per-run solver traces:
  `.greco/ws2-baseline-20260705T160508Z/traces/sessions/*.jsonl`

No fixture, criterion, gate, threshold, or comparison logic was changed.

## Admission Table

| Task | Runs | Objective success | Profiled friction values | Mean | Sample variance | 5% detectable | Admitted | Diagnosis |
|---|---:|---:|---|---:|---:|---|---|---|
| `t1-conventions` | 3 | 3/3 | `criterion_adherence_misses=[0,0,0]`, `avoidable_prompts=[0,0,0]`, `retracements=[0,0,0]` | 0.00 | 0.00 | No: zero profiled baseline friction | No | The solver read `CONVENTIONS.md` first and produced 4 adherence hits/run with 0 misses. This is not a mechanical criterion bug; changing the prompt to induce misses would redefine the fixture. |
| `t2-read-economy` | 3 | 3/3 | `generated_file_reads=[0,0,0]`, `turns=[5,5,5]`, `tool_calls=[4,4,4]` | generated reads 0.00; turns 5.00 | generated reads 0.00; turns 0.00 | No for the written hypothesis: generated-file reads were zero | No | The solver already used `INDEX.md` and never opened the generated dump. Stable turn/token counts exist, but the index-first hypothesis has no observed target friction to move. |
| `t3-edit-discipline` | 3 | 3/3 | `retracements=[1,1,1]` | 1.00 | 0.00 | Yes | Yes | The same-file re-read pattern produces one deterministic retracement per run, matching the edit-discipline friction profile. |
| `t4-staging` | 3 | 3/3 | `objective_failures=[0,0,0]`, `broad_staging_commands=[0,0,0]`, `retracements=[0,0,0]` | 0.00 | 0.00 | No: zero profiled baseline friction | No | The solver staged `src/invoice.txt` explicitly in all runs and preserved unrelated dirty files. This is not a criterion defect. |
| `t5-recovery` | 3 | 3/3 | `repeated_errors=[0,0,0]`, `missing_tool_failures=[0,0,0]`, `retracements=[0,1,1]` | retracements 0.67 | retracements 0.33 | No: target counters zero; retracements too noisy | No | The solver read `TROUBLESHOOTING.md` every run and did not flail on tools or repeat identical errors. The incidental retracement signal is not stable enough for 5% detection. |

Admitted tasks: 1/5. This misses the WS2 target of at least four admitted
tasks. The failure mode is useful: the current prompts are too direct for four
of the five movability hypotheses, so the baseline already demonstrates the
desired behavior instead of exposing moveable friction.

## Friction Summary

| Task | Turns mean | Turns variance | Tool calls mean | Tokens mean | Tokens variance | Other useful observations |
|---|---:|---:|---:|---:|---:|---|
| `t1-conventions` | 9.00 | 0.00 | 8.00 | 9409.00 | 1981.00 | First cargo probe failed in each run, then final criterion passed; adherence misses stayed zero. |
| `t2-read-economy` | 5.00 | 0.00 | 4.00 | 2996.00 | 0.00 | The generated file was never read. |
| `t3-edit-discipline` | 7.67 | 0.33 | 6.67 | 7862.67 | 293477.33 | `retracements` is the admitting signal; generic token variance is above the 5% rule. |
| `t4-staging` | 5.00 | 0.00 | 4.00 | 2999.33 | 14.33 | All runs used explicit path staging. |
| `t5-recovery` | 7.00 | 0.00 | 6.00 | 6011.33 | 524.33 | One expected build failure occurred before the documented fix in every run. |

## Spend Summary

The API response traces exposed token usage but not billed cost. Estimated cost
uses the 2026-07-05 OpenAI API standard short-context `gpt-5.4` rates:
$2.50/M uncached input tokens, $0.25/M cached input tokens, and $15.00/M output
tokens.

| Runs | Uncached input tokens | Cached input tokens | Output tokens | Estimated total | Max estimated run |
|---:|---:|---:|---:|---:|---:|
| 15 | 81,303 | 1,280 | 5,252 | $0.282359 | $0.030433 |

No run approached the stop threshold of approximately $2/run.

## What WS3 Needs

WS3 should not proceed as if all five candidates are admitted. There is only
one admitted friction cluster:

- `t3-edit-discipline`: propose a Layer A edit-discipline procedure targeting
  repeated same-file reads/edits and `retracements`.

Before a full WS3 pass, the suite needs a new WS1/WS2 repair pass or replacement
tasks for at least three dropped candidates. The repair should remove
over-leading task prompts or add criteria that expose the intended friction,
but that is outside this WS2 execution and should be treated as a new semantic
workstream, not as a baseline-report edit.

## Model Deltas

- The candidate suite is not yet a four-task movable suite. Under the current
  prompts and model, only `t3-edit-discipline` exposes stable non-zero
  profiled friction.
- The written movability hypotheses for `t1`, `t2`, `t4`, and `t5` are already
  satisfied by baseline solver behavior often enough that their target
  counters do not provide admission evidence.
