You are classifying a software development intent. Given the description below, determine the classification and appropriate workflow depth.

{{context}}

## Classifications

- **build**: Initial project creation from scratch.
- **feature**: New functionality added to an existing codebase.
- **fix**: Bug fix or error correction.
- **refactor**: Code restructuring without behavior change.
- **maintenance**: Dependency updates, cleanup, documentation.

## Depths

- **full**: Spec generation/amendment, plan generation, phased execution. Use for builds or large features that touch many parts of the system.
- **light**: Plan generation, phased execution (no spec step). Use for most features and refactors.
- **minimal**: Single phase with research, execution, review, commit. Use for fixes and small maintenance tasks.

## Response format

Respond with JSON only. No other text.

```json
{"classification": "...", "depth": "...", "reasoning": "..."}
```

The reasoning field should be one sentence explaining why you chose this classification and depth.
