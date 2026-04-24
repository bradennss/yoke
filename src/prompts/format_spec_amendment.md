## Rules

- Do not remove or significantly restructure existing content.
- Do not rewrite sections that are unaffected by this intent.
- Append your changes as new sections with the heading `## Amendment: {{intent_title}} ({{intent_id}})` at the end of each spec file.
- If the intent modifies existing behavior, reference the original section by name in your amendment and describe what changes.
- If the intent adds entirely new behavior, document it fully within the amendment section following the same structure and level of detail as the existing spec.
- Update both `{{product_spec_path}}` (product spec) and `{{technical_spec_path}}` (technical spec).

## Product spec amendments

Within the amendment section of the product spec, include whichever of the following sub-sections are relevant:

- **Glossary additions**: new entities, states, or relationships introduced by this intent.
- **New or modified flows**: step-by-step interaction sequences. For modified flows, reference the original flow by name and describe only the delta.
- **New or modified requirements**: with unique IDs continuing from the existing numbering scheme. For modified requirements, reference the original ID.
- **New hard rules**: invariants introduced by this intent.
- **New failure modes**: for any new external dependencies or integration points.

## Technical spec amendments

Within the amendment section of the technical spec, include whichever of the following sub-sections are relevant:

- **Data model changes**: new tables, columns, types, or modifications to existing structures.
- **Interface changes**: new endpoints, commands, or modifications to existing interfaces.
- **Architecture changes**: new components, services, or modifications to existing architecture.
- **Dependency changes**: new dependencies or version changes.
