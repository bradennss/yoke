### Context

What exists before this phase starts (produced by prior phases or already present in the codebase). What this phase adds to the project.

### Research topics

Specific external unknowns to investigate before implementation. Each topic: what needs to be learned and why it matters. If the phase uses only well-understood internal patterns, state that no research is needed.

### Deliverables

Exact files and modules this phase produces. Use file paths, not abstract descriptions.

### Tasks

A numbered list of implementation steps. Each task must include:

- **Spec anchor**: which product spec requirement IDs (e.g., `REQ-SEARCH-01`) or technical spec sections this task implements. If the specs do not use formal requirement IDs, reference the relevant spec section or heading instead.
- **Files**: exact paths created or modified.
- **Types/functions**: key signatures the task introduces or modifies (not full implementations).
- **Dependencies**: which prior tasks (by number) must complete first.
- **Acceptance criteria**: concrete, verifiable conditions that prove the task is done. Include the specific tests or checks required. Avoid subjective criteria; every criterion must be objectively testable.

### Acceptance criteria (phase-level)

Conditions for the phase to be considered complete. Must include: all gate commands pass, all task-level acceptance criteria met.

### Handoff

What the next phase inherits from this one: stable types, defined schemas, trait boundaries, new conventions, test utilities.
