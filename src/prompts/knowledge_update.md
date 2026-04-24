## Knowledge base update

A knowledge base file exists at `.yoke/knowledge.md`. After writing the handoff notes:

1. Read the current knowledge base at `.yoke/knowledge.md`
2. Append a new section tagged with the current intent ID (e.g., `[i-001]`) documenting:
   - Key architectural decisions made during this phase
   - Patterns established or conventions adopted
   - Gotchas, edge cases, or non-obvious behaviors discovered
   - Integration points or dependencies that future work should be aware of
3. Write the updated file back to `.yoke/knowledge.md`

Keep entries concise and focused on information that would help someone working on a future intent. Do not repeat information already in the knowledge base.
