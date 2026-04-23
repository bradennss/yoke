# CLAUDE.md Reference

Complete reference for file locations, syntax, content guidelines, and interaction with other Claude Code features.

## Contents

- [File locations and loading behavior](#file-locations-and-loading-behavior)
- [Import syntax](#import-syntax)
- [Content guidelines](#content-guidelines)
- [Emphasis and attention techniques](#emphasis-and-attention-techniques)
- [Progressive disclosure strategies](#progressive-disclosure-strategies)
- [Monorepo patterns](#monorepo-patterns)
- [Interaction with skills and hooks](#interaction-with-skills-and-hooks)
- [Context budget considerations](#context-budget-considerations)
- [Common anti-patterns](#common-anti-patterns)

## File locations and loading behavior

| Location | Path | Scope | Version controlled |
|---|---|---|---|
| Home (global) | `~/.claude/CLAUDE.md` | All projects, all sessions | No |
| Project root | `./CLAUDE.md` | Current project | Yes (recommended) |
| Project local | `./CLAUDE.local.md` | Current project, personal only | No (gitignore) |
| Parent directory | `../CLAUDE.md` | Inherited by child projects | Yes |
| Child directory | `./src/CLAUDE.md` | When working in that directory | Yes |

**Loading behavior:**
- Home and project root CLAUDE.md files load at session start — always in context
- Parent directory CLAUDE.md files load automatically (useful for monorepo roots)
- Child directory CLAUDE.md files load on demand when Claude works with files in that subdirectory
- CLAUDE.local.md loads alongside CLAUDE.md for personal/non-shared instructions
- More specific locations take precedence when instructions conflict

**System reminder context:** Claude Code wraps CLAUDE.md contents in a system reminder noting the context "may or may not be relevant" to the current task. This means Claude may deprioritize instructions it deems irrelevant to the immediate request. Universally applicable instructions are therefore more reliably followed than conditional ones.

## Import syntax

CLAUDE.md files can reference other files using `@path/to/file` syntax:

```markdown
See @README.md for project overview and @package.json for available commands.

# Git workflow
@docs/git-instructions.md

# Personal overrides
@~/.claude/my-project-instructions.md
```

**Rules:**
- `@` references pull in the target file's content
- Paths are relative to the CLAUDE.md file's location
- `@~/` references resolve to the user's home directory
- Use imports to avoid duplication across CLAUDE.md files
- Each import increases context size — use judiciously

## Content guidelines

### Include (high value per token)

| Category | Examples |
|---|---|
| Non-obvious CLI commands | `bun test --watch`, `pnpm --filter=@app/core build` |
| Verification steps | "Run typecheck after modifying TypeScript", "Run `make lint` before committing" |
| Non-default code style | "Use ES modules (import/export), not CommonJS (require)" |
| Workflow conventions | "Branch from `develop`, not `main`", "Squash merge PRs" |
| Architectural decisions | "All API routes go through the gateway service" |
| Environment requirements | "Requires `OPENAI_API_KEY` env var", "Run `docker compose up` for local Postgres" |
| Common gotchas | "The `users` table uses soft deletes — always filter by `deleted_at IS NULL`" |

### Exclude (wastes tokens, hurts compliance)

| Category | Why |
|---|---|
| Standard language conventions | Claude already knows them |
| Code patterns visible in codebase | Claude learns from code it reads |
| Detailed API documentation | Link to external docs instead |
| Frequently changing information | Goes stale, causes confusion |
| File-by-file codebase descriptions | Claude can read the file tree |
| Linter-enforceable style rules | Use linters + hooks instead |
| Self-evident instructions | "Write clean code", "Handle errors" |
| Generic best practices | "Follow SOLID principles", "Write tests" |
| Long code examples | Become stale; use file:line references |

### Conditionally include

| Category | Include when... |
|---|---|
| Project structure map | Monorepo or non-standard directory layout |
| Service descriptions | Microservices or multi-app repository |
| Database conventions | Non-obvious schema patterns (soft deletes, JSONB usage) |
| Third-party integrations | Non-standard auth flows or API usage patterns |

## Emphasis and attention techniques

When Claude consistently misses a rule, the file may be too long. Pruning is the first fix. If the rule is genuinely important and the file is already concise:

- **`IMPORTANT:`** prefix for critical rules
- **`YOU MUST`** for absolute requirements
- **ALL CAPS** for single key words (sparingly)
- **Bold text** for visual hierarchy
- Place critical rules near the top of the file (primacy bias)
- Give critical rules their own section with a clear header

**Use emphasis sparingly.** If everything is important, nothing is. Reserve for rules that cause real problems when missed. More than 2-3 emphasized rules in a file signals the file is too long.

## Progressive disclosure strategies

### Strategy 1: Agent docs directory

Create reference documents indexed in CLAUDE.md:

```markdown
## Reference docs
Read these when working on related areas:
- `agent-docs/database.md` — schema conventions, migration patterns
- `agent-docs/api.md` — endpoint conventions, auth patterns
- `agent-docs/deploy.md` — deployment process, environment configs
```

### Strategy 2: @import references

Pull in existing documentation:

```markdown
@docs/CONTRIBUTING.md
@docs/ARCHITECTURE.md
```

### Strategy 3: Pointers without import

Reference docs without loading them into context:

```markdown
## Reference
- API docs: see `docs/api/` directory
- Database schema: see `prisma/schema.prisma`
- Component library: see Storybook at localhost:6006
```

### Strategy 4: Skills for domain knowledge

Move specialized knowledge to skills that load on demand:

```markdown
## Skills
- Use `/deploy` for deployment workflows
- Use `/db-migrate` for database migrations
```

**Choosing the right strategy:**
- **Agent docs**: Project knowledge Claude needs occasionally
- **@imports**: Existing docs you don't want to duplicate
- **Pointers**: External resources or self-explanatory files
- **Skills**: Repeatable workflows or specialized domain knowledge

## Monorepo patterns

**Root CLAUDE.md** — repo-wide instructions only:

```markdown
# my-platform

Monorepo managed by Turborepo and pnpm workspaces.

## Structure
- `apps/web` — Next.js customer dashboard
- `apps/api` — Express REST API
- `packages/shared` — Shared TypeScript types
- `packages/ui` — React component library

## Commands
- Build all: `pnpm build`
- Build one: `pnpm --filter=@platform/web build`
- Typecheck all: `pnpm typecheck`

## Conventions
- Import shared types: `import { User } from '@platform/shared'`
- Never import directly between apps — use shared packages
- Changes to `packages/shared` affect all apps — test broadly
```

**Package-level CLAUDE.md** — package-specific instructions:

```markdown
# apps/api/CLAUDE.md

## Testing
Run tests: `pnpm test`
Run single test: `pnpm test -- path/to/test`

## Database
Migrations: `pnpm prisma migrate dev`
Schema changes require a migration — never modify the database directly.
```

Package-level files load on demand when Claude works with files in that directory. Never duplicate root instructions in child files.

## Interaction with skills and hooks

### CLAUDE.md vs skills

| Use CLAUDE.md for... | Use skills for... |
|---|---|
| Rules that apply to every session | Domain knowledge needed occasionally |
| Build/test/lint commands | Repeatable workflows (deploy, review, migrate) |
| Project-wide conventions | Specialized reference material |
| Verification steps | Step-by-step guides for complex tasks |

**Rule of thumb:** If it applies to >80% of sessions, CLAUDE.md. Otherwise, a skill.

### CLAUDE.md vs hooks

| Use CLAUDE.md for... | Use hooks for... |
|---|---|
| Advisory instructions | Mandatory actions (must happen every time) |
| Context and rationale | Deterministic checks (lint, format, validate) |
| Flexible guidelines | Blocking rules (prevent writes to certain dirs) |

**Rule of thumb:** If you'd be upset when Claude skips it, make it a hook. CLAUDE.md instructions are advisory.

## Context budget considerations

CLAUDE.md content loads into every conversation. Larger files reduce context available for file contents, command outputs, conversation history, and active skills.

**Approximate token costs:**
- 60-line CLAUDE.md: ~500-800 tokens (ideal)
- 150-line CLAUDE.md: ~1,500-2,500 tokens (acceptable)
- 300-line CLAUDE.md: ~3,000-5,000 tokens (maximum)

Claude Code's system prompt uses ~50 instructions. LLMs reliably follow ~150-200 total instructions. Every CLAUDE.md instruction counts against that budget.

## Common anti-patterns

1. **The kitchen sink.** Every possible instruction crammed in. Causes uniform compliance degradation across ALL instructions.

2. **The linter replacement.** Style rules a linter could enforce. Wastes tokens, inconsistent results. Use a formatter hook instead.

3. **The README duplicate.** Copying README content. Claude can read the README itself.

4. **The changelog.** Version history, migration notes, time-sensitive information that goes stale.

5. **The code snippet museum.** Pasted code that will become outdated. Use file:line references to the authoritative source.

6. **The over-emphasizer.** Everything marked IMPORTANT/MUST/CRITICAL. When everything is emphasized, nothing is.

7. **The conditional block.** "When working on the API, do X. When working on the frontend, do Y." These belong in child CLAUDE.md files or skills.

8. **The unreviewed auto-generation.** Using `/init` output verbatim. Auto-generated files include obvious information and miss non-obvious gotchas. Always hand-craft and curate.

9. **The aspirational rules.** Conventions the team wants to follow but the codebase doesn't actually follow. Claude will match what the code does, not what you wish it did.

10. **The stale reference.** Mentions of files, commands, or patterns that no longer exist. Causes Claude to attempt impossible operations.
