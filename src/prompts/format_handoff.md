## Sections

### 1. Completed deliverables

List every file created or modified with a one-line description and the spec requirement IDs or technical spec sections it implements. This enables the next phase to verify traceability.

### 2. Key decisions

Architectural or design choices made during implementation. Focus on decisions that affect subsequent phases. For each decision: what was decided, what alternatives were considered, and why this approach was chosen.

### 3. Introduced patterns

New utilities, traits, helper functions, or conventions that subsequent phases should reuse. Include file paths and brief usage notes. This prevents the next phase from reinventing existing abstractions.

### 4. Known issues

Things that work but could be improved, edge cases not yet handled, tech debt introduced intentionally. Each with severity and whether it blocks future phases.

### 5. What was tried and abandoned

Approaches that were attempted during implementation and didn't work. Include what was tried and why it failed. This prevents the next phase from repeating failed experiments.

### 6. Next phase setup

What the next phase inherits and should be aware of before starting: stable types, defined schemas, trait boundaries, configuration patterns, test utilities available for reuse. Be specific about file paths and public API surfaces.
