# yoke

> Orchestrate Claude Code through structured development workflows.

Yoke drives Claude Code sub-agents through a repeatable pipeline: spec authoring, phase decomposition, and per-phase execution with automatic context injection, review cycles, and state tracking. It replaces ad-hoc prompting with a deterministic workflow that produces consistent, high-quality results across complex projects.

## Highlights

- **Automatic context injection** into every sub-agent prompt, saving tokens and turns
- **Review cycles** at every stage with configurable iteration limits
- **Resumable execution** with step-level state tracking; restart from any step on failure
- **Configurable models** per task type (spec, research, planning, review, execution, code review)
- **Prompt overrides** for full control over agent behavior without touching source code

## Installation

```bash
cargo install --path .
```

Requires the [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code) installed and authenticated.

## Quick Start

```bash
yoke init
yoke spec "Build a REST API that manages a task queue with priorities and deadlines"
yoke plan
yoke run
```

## Usage

### Initialize a project

```bash
$ yoke init
initialized .yoke in /Users/you/my-project
```

Creates `.yoke/config.toml`, `.yoke/state.json`, and `.yoke/prompts/` for overrides.

### Generate specs

Pass a description as an argument, from a file, or piped via stdin:

```bash
yoke spec "A CLI tool that converts markdown to HTML with plugin support"
yoke spec --file description.md
cat description.md | yoke spec
```

Produces `docs/product-spec.md` and `docs/technical-spec.md`, each automatically reviewed and revised.

### Generate the implementation plan

```bash
yoke plan
```

Reads both specs and produces `docs/plan.md` with individual phase files in `docs/phases/` (e.g., `001-setup.md`, `002-core-logic.md`).

### Execute phases

```bash
yoke run          # run the next pending phase
yoke run 3        # run phase 3 specifically
yoke run 3 --from execution   # restart phase 3 from the execution step
```

Each phase runs through seven steps: research, planning, plan review, execution, code review, handoff, and git commit. State is saved after each step; re-running resumes from the point of failure.

### Check progress

```bash
$ yoke status
Spec: complete
Plan: complete

  Phase 1: setup [completed]
    cost: $0.1542
  Phase 2: core logic [in progress]
    current step: execution
    cost: $0.0830
  Phase 3: api endpoints [pending]

Total cost: $0.2372
```

### Preview without invoking Claude

```bash
yoke run --dry-run
```

Shows the prompt, model, and tools that would be sent without making any API calls.

## Configuration

Edit `.yoke/config.toml` after running `yoke init`:

```toml
# Project identity
[project]
name = "my-project"

# Claude model to use for each phase. Valid values: opus, sonnet, haiku.
[models]
spec = "opus"
research = "sonnet"
planning = "opus"
review = "sonnet"
execution = "opus"
code_review = "sonnet"
spec_extract = "haiku"

# Thinking effort for each phase. Valid values: low, medium, high, max.
[effort]
spec = "high"
research = "high"
planning = "high"
review = "high"
execution = "high"
code_review = "high"
spec_extract = "high"

# Automatically commit changes at the end of each phase.
[git]
auto_commit = true

# Maximum review iterations before moving on (plan review and code review).
# If the review does not converge within this limit, behavior depends on
# interaction mode: autonomous continues anyway, milestones pauses for input.
[review]
max_iterations = 5

# Retry configuration for transient failures (rate limits, server errors).
# Uses exponential backoff: base_delay_secs * 2^attempt, with ±25% jitter.
[retry]
max_retries = 3
base_delay_secs = 10

# "autonomous" runs all phases without pausing.
# "milestones" pauses after specs, plans, and unconverged reviews for human review.
interaction = "autonomous"

# Shell commands that must pass before work is considered complete.
# These are provided to Claude as constraints, not run directly by yoke.
# Example: ["cargo fmt", "cargo check", "cargo test"]
gate_commands = []

# Skip spec extraction for small projects. If combined product + technical spec
# tokens are below this threshold, full specs are injected directly.
# Set to 0 to always extract.
[context]
spec_extract_threshold = 4000
```

Override any prompt template by placing a file in `.yoke/prompts/` (e.g., `.yoke/prompts/execution.md`). Templates use `{{variable}}` placeholders for context injection.

## License

[MIT](LICENSE)
