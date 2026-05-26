# Greco Agent Contract

Greco is a Rust coding-agent harness whose v0 evolutionary surface is the skill catalog.

## Operating Rules

- Keep the harness small enough to read in one sitting.
- Prefer direct Rust, `tokio`, `reqwest`, `rustls`, `serde`, and `serde_json` over broad agent frameworks.
- Keep provider abstractions narrow: one trait, one OpenAI implementation, no speculative cross-provider semantics.
- Do not add a skill to the active catalog without empirical validation evidence.
- Preserve failed skill candidates and validation traces; archival memory is part of the product.
- Keep local secrets in `.env.local` or user-level secure storage only. Never commit credentials.

## Verification

Before claiming completion, run:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For secret checks, run:

```sh
git status --ignored --short
git grep -n -E "sk-(proj|svcacct)-" -- . ':!docs/**'
```

The RFC may contain historical text but must not contain live credentials.
