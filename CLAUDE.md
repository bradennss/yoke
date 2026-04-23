# yoke

Harness for speccing, building and maintaining complex applications with Claude Code.

This project is a VERY EARLY WIP. Proposing sweeping changes that improve long-term maintainability is encouraged.

## Core Priorities

1. Performance first.
2. Reliability first.
3. Keep behavior predictable under load and during failures (session restarts, reconnects, partial streams).

If a tradeoff is required, choose correctness and robustness over short-term convenience.

## Maintainability

Long term maintainability is a core priority. If you add new functionality, first check if there is shared logic that can be extracted to a separate module. Duplicate logic across multiple files is a code smell and should be avoided. Don't be afraid to change existing code. Don't take shortcuts by just adding local logic to solve a problem.

## Workflow

- Do not make assumptions; always ask the user questions using the question tool to clarify uncertainty. Plans must not have unresolved questions.

## Hard rules

- Never write unnecessary inline comments in code; comments earn their keep.
- Always Use CLIs for updating dependencies or scaffolding new packages where possible.
- Always use the latest stable versions of dependencies.
- Never add Co-Authored-By or any attribution lines to commits.
- Never use hypens, double hypens, or em dashes as punctuation. Restructure with periods, commas, or semicolons instead.
- Always use `/rust-dev` when planning or implementing Rust code.
- Always use `/claude-md` when updating CLAUDE.md or supporting documentation.

## Commands

No code is complete until it passes all four commands below:

```sh
cargo fmt
cargo check --all-targets
cargo clippy --all-targets --all-features --fix --allow-dirty -- -D warnings
cargo test --all-features
```
