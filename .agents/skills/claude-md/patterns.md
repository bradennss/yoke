# CLAUDE.md Patterns

Example CLAUDE.md files for common project types. Adapt to your project — do not copy verbatim.

## Contents

- [Minimal project](#minimal-project)
- [Standard web application](#standard-web-application)
- [Monorepo](#monorepo)
- [Progressive disclosure](#progressive-disclosure)
- [Personal global config](#personal-global-config)
- [CLAUDE.local.md](#claudelocalmd)
- [Strong emphasis pattern](#strong-emphasis-pattern)
- [Anti-pattern: over-specified](#anti-pattern-over-specified)

## Minimal project

For small projects where conventions are clear from the code. Under 30 lines.

```markdown
# my-cli

TypeScript CLI tool for processing CSV data.

## Commands
- Build: `npm run build`
- Test: `npm test`
- Lint: `npm run lint`

## Verification
Run `npm test && npm run lint` after changes.
```

Sufficient when project structure is self-evident and conventions follow language defaults.

## Standard web application

Covers the essential categories for a typical project.

```markdown
# my-web-app

Full-stack TypeScript app: Next.js frontend, Express API, PostgreSQL.

## Commands
- Dev server: `pnpm dev` (starts both frontend and API)
- Test: `pnpm test` (unit) | `pnpm test:e2e` (end-to-end)
- Typecheck: `pnpm typecheck`
- Lint + format: `pnpm lint`
- DB migrations: `pnpm prisma migrate dev`

## Verification
After code changes: `pnpm typecheck && pnpm test`
After schema changes: `pnpm prisma migrate dev && pnpm test`

## Code style
- Use ES modules (import/export), not CommonJS (require)
- Destructure imports: `import { foo } from 'bar'`
- Zod for all input validation at API boundaries

## Workflow
- Branch from `main`, PR back to `main`
- Squash merge PRs
- Conventional commits: `feat:`, `fix:`, `chore:`

## Gotchas
- `users` table uses soft deletes — always filter by `deleted_at IS NULL`
- Auth tokens are in HttpOnly cookies, not localStorage
- `STRIPE_WEBHOOK_SECRET` env var required for payment tests
```

## Monorepo

Root-level file with repo-wide instructions. Package-specific instructions go in child CLAUDE.md files.

```markdown
# my-platform

Monorepo: Turborepo + pnpm workspaces.

## Structure
- `apps/web` — Next.js customer dashboard
- `apps/admin` — Internal admin panel
- `apps/api` — Express REST API
- `packages/shared` — Shared types and utilities
- `packages/ui` — React component library
- `packages/db` — Prisma client and migrations

## Commands
- Install: `pnpm install` (from root)
- Build all: `pnpm build`
- Build one: `pnpm --filter=@platform/web build`
- Test one: `pnpm --filter=@platform/api test`
- Typecheck all: `pnpm typecheck`

## Conventions
- Import shared types: `import { User } from '@platform/shared'`
- Never import directly between apps — use shared packages
- Changes to `packages/shared` affect all apps — test broadly
- Each package has its own CLAUDE.md with package-specific instructions
```

## Progressive disclosure

CLAUDE.md kept short with pointers to detailed reference docs.

```markdown
# my-saas

B2B SaaS platform: React frontend, Python FastAPI backend, PostgreSQL.

## Commands
- Frontend: `cd frontend && npm run dev`
- Backend: `cd backend && uvicorn main:app --reload`
- Tests: `cd backend && pytest` | `cd frontend && npm test`
- Lint: `make lint`

## Verification
After backend changes: `pytest && mypy .`
After frontend changes: `npm test && npm run typecheck`

## Reference docs
Read these when working on related areas:
- `agent-docs/architecture.md` — service boundaries, data flow, auth system
- `agent-docs/database.md` — schema conventions, migration process, seed data
- `agent-docs/api.md` — endpoint patterns, error handling, pagination
- `agent-docs/testing.md` — fixture patterns, mocking conventions, CI setup
```

Under 30 lines while making detailed context available on demand.

## Personal global config

For `~/.claude/CLAUDE.md` — preferences across all projects.

```markdown
# Personal preferences

## Style
- Concise responses, skip summaries of what you just did
- Don't add comments unless the logic is non-obvious
- Single-line imports when under 100 chars

## Workflow
- Conventional commits: type(scope): description
- Run tests before suggesting a commit
- Prefer single focused PRs over large bundled changes

## Tools
- Use `gh` CLI for all GitHub operations
- Use `jq` for JSON processing
```

## CLAUDE.local.md

Personal project-specific overrides. Add to `.gitignore`.

```markdown
# Local overrides

## Environment
- My local Postgres runs on port 5433 (not default 5432)
- Use `DATABASE_URL=postgres://localhost:5433/mydb`

## Preferences
- I'm focused on the payments module — prioritize that context
- I know the API well, skip explanations of endpoint patterns
```

## Strong emphasis pattern

When a rule is critical and Claude tends to miss it:

```markdown
## Database

IMPORTANT: Never modify the production database schema directly. All changes MUST go through Prisma migrations.

IMPORTANT: The `orders` table uses event sourcing. Never UPDATE rows — only INSERT new events.
```

If more than 2-3 rules need this treatment, the file is too long and emphasis is being used as a crutch. Prune first.

## Anti-pattern: over-specified

What NOT to do — this file tries to cover everything and ends up being ignored:

```markdown
# my-app (DO NOT WRITE FILES LIKE THIS)

## About
This is a web application built with React and Node.js. It serves as a
platform for managing customer relationships. The application was started
in 2022 and has been through several refactors...

## File structure
- src/components/ - React components
- src/utils/ - Utility functions
- src/hooks/ - Custom React hooks
- src/api/ - API layer
[...20 more directories]

## Code style
- Use camelCase for variables
- Use PascalCase for components
- Use UPPER_CASE for constants
- Always use semicolons
- Use single quotes
- Max line length 100
[...15 more linter-enforceable rules]

## Error handling
- Always wrap async operations in try/catch
- Log errors to console
- Show user-friendly error messages
[...10 more obvious instructions]

## Testing
- Write tests for all new features
- Use descriptive test names
- Follow AAA pattern (Arrange, Act, Assert)
[...10 more generic testing advice]
```

**Problems:**
- "About" section: prose adds no actionable information
- File structure: Claude can read the filesystem
- Code style: linter's job, not Claude's
- Error handling: Claude already knows this
- Testing: generic advice Claude already follows
- Result: 100+ lines providing zero unique value, making every instruction less likely to be followed
