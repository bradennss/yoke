### Fixed decisions

Choices that apply to all phases and are not revisited: language edition, database engine, model assignments, key dependency choices, error handling conventions, naming conventions. Decided once; if a phase needs to change a fixed decision, it must document why.

### Working agreement

A per-phase checklist that every phase follows: gate commands to run, skills to invoke (e.g., /rust-dev), convention reminders. This section prevents drift across phases.

### Target layout

The directory and module tree showing the final state after all phases complete. One-line purpose per module. This is the north star that code reviews compare against.

### Phase summary table

A table with columns: phase number, title, primary deliverable (one line each). This is the at-a-glance view.
