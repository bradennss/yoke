Design and implement v1 of yoke based on the current harness setup I have for Asa.

Reference files:

- @~/projects/asa/plan-phase.sh - runs claude code in TUI mode with model settings and templated plan prompt
- @~/projects/asa/plan-prompt.md - core of the harness; describes all of the steps in detail to ensure claude runs each step in a sub-agent for efficiency
- @~/projects/asa/docs/plans - plans written by the harness
- @~/projects/asa/docs/research - research documents written by the harness
- @~/projects/asa/docs/deviations - deviation documents written by the harness
- @~/projects/asa/docs/plan.md and @docs/phases - full implementation plan decomposed from product and technical specs
- @~/projects/asa/docs/product-spec.md - original product spec
- @~/projects/asa/docs/technical-spec.md - original technical spec authored alongside product spec

This CLI program will drive claude through the same process that the current plan prompt does, but in a more controlled manner.

Key features:

- Automatically injects relevant context into each task - claude doesn't need to read the same context files manually. Saves on tokens and turns
- Writing handoff documents after each phase
- Using appropriate models for different tasks

New feature: Spec authoring and phase/task decomposition

Yoke should start with a rough-draft or explanation prompt (like the one you're reading) from the user and design a spec with a smart model. It will use a similar self-review and revise cycle that development uses. Once the spec is accepted by the user, it decomposes phases and tasks. The implementation also goes through a self-review and revise cycle.

The CLI should stream human-readable claude output using the --stream-json and --verbose flags and parsing the output. Research claude's stream json output format to come up with a concrete plan on using it.

Ask me key questions about your design and implementation using the question tool so that there are no unanswered questions in the final plan.

Key design decisions that need to be made:

- Prompt engineering
- Formats for documents written by the agent
