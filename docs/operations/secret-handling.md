# Secret Handling

Greco reads OpenAI credentials from the environment. Local development uses `.env.local`, which is ignored by git.

Required local keys:

```sh
OPENAI_API_KEY=...
GRECO_PROVIDER=openai
GRECO_MODEL=gpt-5.4
```

Rules:

- Do not commit `.env.local`.
- Do not write API keys into docs, tests, fixtures, commit messages, or shell transcripts.
- Prefer user-level secure storage for long-lived credentials when packaging catches up.
- Treat any API key pasted into an assistant transcript as exposed and rotate it before public release.

Verification:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```
