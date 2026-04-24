## Sections

### 1. System overview

A textual component diagram showing data flow between all major subsystems. Name every component and state its responsibility in one sentence. Show how data flows between components (what produces, what consumes, what the payload is).

### 2. Module layout

The exact directory, crate, or package tree for the project. One-line purpose per module. This becomes the target layout for the implementation plan.

<examples>
<example>
```
my-project/
  my-core/          # shared types, errors, config, database
  my-api/           # HTTP server, routes, middleware
  my-worker/        # background job processing
  my-cli/           # binary entry point, CLI parsing
```
</example>
</examples>

### 3. Data model

Every table, struct, or storage entity. For each entity, specify:

- **Field names and types** using the language's actual type system (e.g., `id: Ulid`, `name: String`, `status: SearchStatus`, `created_at: DateTime<Utc>`).
- **Constraints**: NOT NULL, UNIQUE, CHECK constraints, foreign keys.
- **Indexes** with rationale (e.g., "index on `(search_id, status)` for filtered listing queries").
- **Enum columns** with all variants listed (e.g., `SearchStatus: active | paused | completed`).
- **Primary key strategy** with rationale (UUIDs, ULIDs, auto-increment).
- **Timestamp format and timezone** convention (e.g., RFC 3339, UTC).
- **Relationships** with cardinality and join strategy.

<examples>
<example>
### searches

| Field | Type | Constraints |
|-------|------|-------------|
| id | TEXT (ULID) | PRIMARY KEY |
| status | TEXT | NOT NULL, CHECK(status IN ('active','paused','completed')) |
| metro | TEXT | NOT NULL |
| min_beds | INTEGER | |
| max_price | INTEGER | |
| created_at | TEXT | NOT NULL, RFC 3339 UTC |

**Indexes:** `(status)` for active search queries.
**Relationships:** many-to-many with clients via `search_clients`; one-to-many with `search_listings`.
</example>
</examples>

### 4. Interfaces

Every public API surface. For each interface:

- **Function signatures** with argument types, return types, and error types.
- **CLI commands** with flags, arguments, defaults, and exit codes.
- **Configuration file format** with all fields, types, defaults, and validation rules.
- **Wire protocols or IPC schemas** if applicable (message types, serialization format).
- **For each operation**: success behavior, error responses (all variants), and edge case behavior (empty input, concurrent access, rate limiting).

### 5. Dependencies

A table of external crates, services, or tools:

| Name | Version | Purpose | License |
|------|---------|---------|---------|

Note MSRV or compatibility constraints. State the rationale for each non-obvious dependency choice.

### 6. Error handling

- Error enum with all variants.
- Propagation strategy (e.g., `thiserror` in library crates, `anyhow` at binary boundaries).
- What the user or operator sees for each error class.
- Recovery strategy for each recoverable error.

### 7. Concurrency and state

If the system has scheduling, locking, retries, or concurrent access:

- Task scheduling model (polling loop, event-driven, cron).
- Locking strategy with conflict resolution.
- Retry policy with **concrete values** (not "exponential backoff" but "delays of 10s, 60s, 300s with ±25% jitter; max 3 attempts").
- Debounce windows with durations.
- Priority levels with numeric values and ordering rationale.

If the system has no concurrency concerns, omit this section.

### 8. Observability

- Logging framework and output format (structured JSON, plaintext).
- Log levels with what triggers each (ERROR for unrecoverable, WARN for retries, INFO for state changes, DEBUG for details).
- Key structured fields per log event.

### 9. Testing strategy

- **Unit tests**: pure logic, no I/O, no network.
- **Integration tests**: real database per test, fakes for external services. How test isolation is achieved.
- **Scenario or E2E tests**: format and structure if applicable.
- **Determinism controls**: pinned model versions, temperature settings, seed values for any non-deterministic components.

### 10. Configuration and secrets

- Environment variable names with descriptions and defaults.
- Configuration file sections with all fields, types, and defaults.
- What is never checked into version control.
- Example configuration file content.

## Principles

- Be specific about types and function signatures. Prefer concrete examples over abstract descriptions.
- Include alternatives considered for major architectural decisions, with trade-off analysis for why the proposed approach was chosen.
- Every entity in the product spec glossary must appear in the data model. Every action in the product spec flows must be covered by an interface.
