# Risks and Mitigations

## False Skill Positives

Risk:

A skill passes a narrow validation task but introduces subtle regressions on real tasks.

Mitigation:

- Keep validation traces.
- Require multiple validation tasks before high score.
- Track post-promotion failures and demote automatically.
- Never delete rejected or failed variants during v0.

## Catalog Debt

Risk:

The archive becomes a junk drawer of near-duplicate scripts.

Mitigation:

- Use manifest IDs, versions, lineage, and status.
- Add `retired` before deletion.
- Keep scoring simple and visible.
- Add catalog lint before adding optimizer logic.

## Scaling to Hundreds of Skills

Risk:

Prompt stuffing and naive skill selection degrade quality and cost.

Mitigation:

- Only expose summaries initially.
- Use local search/ranking before model exposure.
- Consider OpenAI `tool_search` once skill count makes context cost measurable.
- Move from flat score file to SQLite only after filesystem archive limits are observed.

## Validation Cost

Risk:

Autonomous validation consumes model and compute budget.

Mitigation:

- Prefer executable tests over model judgment.
- Cache validation fixtures.
- Run live OpenAI calls only when necessary.
- Make validation budget visible in traces.

## Secret Exposure

Risk:

The initial API key was pasted into an assistant transcript and could be considered exposed.

Mitigation:

- Store it only in ignored local files with `600` permissions.
- Never commit it.
- Rotate before any public release.
- Add secret scan checks to the release gate.

## Provider Abstraction Drift

Risk:

The provider trait becomes a speculative framework before the OpenAI path works.

Mitigation:

- Keep only one provider implementation in v0.
- Keep OpenAI-specific capabilities in the adapter.
- Add a second provider only after real integration pressure.

## Harness Self-Modification

Risk:

The agent starts editing the harness instead of evolving skills, invalidating the v0 thesis.

Mitigation:

- Treat harness changes as normal human/agent development commits.
- Treat skill generation as the only autonomous evolutionary path.
- Keep candidate validation workspaces separate from the repo.
