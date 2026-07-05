# Conventions

- Feature implementations live in `src/<feature>.rs`.
- `src/lib.rs` declares each feature module and re-exports the public API.
- Public fallible functions return `Result<T, CalcError>`.
- Reuse the existing `CalcError` type; do not create a second error enum.
- Source code must not call `unwrap`, `expect`, or `panic!`.
- Behavior tests use names shaped as `<feature>_when_<condition>`.
