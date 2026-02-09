# Project rules

## Rust quality gates

Before returning control to the user after any Rust code change, always run these checks in order:

1. `cargo check` — must pass with zero errors
2. `cargo clippy` — must pass with zero warnings (`cargo clippy -- -D warnings`)
3. `cargo test` — all tests must pass

Fix any issues found before presenting results. Do not skip these steps.

## Version control

- Use **jj** (Jujutsu), not git. Never run raw git commands.
- Workflow per task: `jj commit -m "message"` then `jj squash` to fold into the parent.
- Do not push or create branches yet. `jj tug` and `jj git push` will be used later when we have branches.

## Style

- Follow idiomatic Rust and existing patterns in the codebase.
- Do not add unnecessary dependencies, abstractions, or boilerplate.
- Keep code simple and direct.
