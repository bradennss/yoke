## Output format

Write the result to `{{target_file}}`. Structure it exactly as follows:

```
## Task to spec mapping

| Task | Product spec sections | Technical spec sections |
|------|----------------------|------------------------|
| 1. [task title] | [requirement IDs or section headings] | [relevant technical spec sections] |

## Extracted product specification

### Overview
[verbatim from product spec]

### Glossary
[verbatim from product spec]

### Requirements
#### [requirement ID or section heading]: [title]
[verbatim]

### User flows
#### Flow N: [title]
[verbatim, only if referenced]

### Hard rules
[verbatim from product spec]

### Failure modes
[verbatim, only sections referenced by tasks]

### Non-requirements
[verbatim from product spec]

## Extracted technical specification

### System overview
[verbatim from technical spec]

### Module layout
[verbatim from technical spec]

### Data model
[verbatim, only entities referenced by tasks]

### Interfaces
[verbatim, only interfaces referenced by tasks]

### Dependencies
[verbatim from technical spec]

### Error handling
[verbatim from technical spec]
```

Omit any top-level section header if neither the product spec nor the technical spec contains it. Omit filtered sections (User flows, Failure modes, Data model, Interfaces) entirely if no entries match.
