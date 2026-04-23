---
name: claude-md
description: Authors, reviews, and maintains CLAUDE.md files for Claude Code projects. Guides the full process from project assessment through content organization, ensuring files are concise, universally applicable, and follow progressive disclosure. Use when creating, reviewing, or improving a CLAUDE.md file, or when discussing CLAUDE.md best practices.
when_to_use: When the user asks to create, write, improve, review, or maintain a CLAUDE.md file, set up CLAUDE.md hierarchy, or asks about CLAUDE.md best practices, content guidelines, or file organization.
argument-hint: "[path-to-project or path-to-existing-claude-md]"
effort: max
---

# CLAUDE.md Authoring Guide

Follow this guide when creating new CLAUDE.md files or reviewing existing ones. For detailed reference see [reference.md](reference.md). For example patterns see [patterns.md](patterns.md).

## The core principle

CLAUDE.md is loaded into every conversation. It is the highest leverage point in Claude Code — a bad line affects every task, every session, every artifact. Every line must earn its place.

**Three tests for every line:**
1. Is this universally applicable across sessions? If not, it belongs in a skill, agent doc, or separate reference file.
2. Can Claude figure this out by reading the code? If yes, delete it.
3. Would removing this cause Claude to make mistakes? If not, cut it.

## Creating a CLAUDE.md

### Step 1: Assess the project

Before writing anything, read the codebase to understand:

1. **Stack**: Languages, frameworks, package managers, build tools — check `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, Dockerfiles, etc.
2. **Non-obvious workflows**: Build/test/lint/format commands, deployment scripts, CI config
3. **Gotchas**: Required environment variables, services that must be running, non-standard configurations, common pitfalls
4. **Conventions**: Branch naming, PR templates, commit formats, code organization patterns
5. **Existing docs**: README, CONTRIBUTING, docs/ — avoid duplicating what already exists

Read the code first. Ask the user targeted questions only about things you cannot determine from the codebase (architectural rationale, future plans, team preferences).

### Step 2: Plan content using the WHAT / WHY / HOW framework

**WHAT** — Project identity and structure
- Tech stack and key dependencies (only if non-obvious)
- Project structure map (essential for monorepos, skip for simple projects)
- What the apps/services are and what they do

**WHY** — Purpose and architectural context
- What the project does (one sentence)
- Key architectural decisions and their rationale
- Non-obvious design choices that would otherwise confuse Claude

**HOW** — Working on the project
- Build, test, lint, typecheck commands
- Verification steps Claude should run after changes
- Workflow conventions (branches, PRs, commits)
- Environment requirements (env vars, services)

**Filter ruthlessly.** For each item, ask: "Would Claude get this wrong without being told?" Keep only items where the answer is yes.

### Step 3: Write the CLAUDE.md

**Format rules:**
- Short, direct sentences. Not prose paragraphs.
- Markdown headers to group related instructions
- Bullet points and code blocks over paragraphs
- `IMPORTANT:` or `YOU MUST` sparingly for critical rules Claude tends to miss
- Target under 150 lines. Under 60 is ideal. Never exceed 300.

**Content priority order:**
1. Verification steps Claude should run after changes (highest leverage single instruction)
2. Build/test/lint commands Claude cannot guess
3. Code style rules that differ from language defaults
4. Repository workflow conventions (branches, PRs, commits)
5. Architectural decisions specific to this project
6. Developer environment quirks (required env vars, services)
7. Common gotchas and non-obvious behaviors

**Exclude:**
- Standard language conventions Claude already knows
- Anything derivable from code or config files
- Detailed API documentation (link to docs instead)
- Frequently changing information
- File-by-file codebase descriptions
- Style rules enforceable by linters (use hooks instead)
- Self-evident practices ("write clean code", "handle errors")

### Step 4: Set up progressive disclosure

If the project needs more context than fits concisely, create separate reference files:

```
agent-docs/
├── architecture.md
├── database.md
├── testing.md
└── api-conventions.md
```

In CLAUDE.md, add a brief index:

```markdown
## Reference docs
- `agent-docs/architecture.md` — service boundaries, data flow, auth system
- `agent-docs/database.md` — schema conventions, migrations, seed data
- `agent-docs/testing.md` — test framework, fixtures, CI setup
```

Claude reads these only when relevant — zero context cost otherwise. Prefer pointers over copies: use `file:line` references to the authoritative source instead of pasting code snippets that will become stale.

### Step 5: Set up verification

Every CLAUDE.md should tell Claude how to verify its work. This is the single highest-leverage instruction you can include.

```markdown
## Verification
- Run `npm test` after code changes
- Run `npm run typecheck` after modifying TypeScript files
- Run `npm run lint` after any file changes
```

For checks that must happen every time with zero exceptions, use hooks (`.claude/settings.json`) instead — hooks are deterministic, CLAUDE.md instructions are advisory.

### Step 6: Verify quality

Run through every item in [checklist.md](checklist.md) before finalizing.

## Improving an existing CLAUDE.md

When asked to review or improve an existing CLAUDE.md:

1. **Read it completely** alongside the actual codebase
2. **Apply the three tests** to every line: universally applicable? not code-inferrable? removal would cause mistakes?
3. **Run the checklist**: Go through [checklist.md](checklist.md) item by item
4. **Check for staleness**: Do referenced files, commands, and patterns still exist?
5. **Check for bloat**: Instructions Claude follows by default, duplicated info, verbose explanations, linter-enforceable rules
6. **Check for gaps**: Missing verification steps? Missing non-obvious commands? Missing gotchas?
7. **Check progressive disclosure**: Should any content move to separate files or skills?
8. **Propose specific changes** organized as: add, remove, move to separate file, rewrite

## Setting up CLAUDE.md hierarchy

For monorepos or multi-layer configurations, see the location reference in [reference.md](reference.md).

| Location | Scope | Check in? |
|---|---|---|
| `~/.claude/CLAUDE.md` | All projects, all sessions | No (personal) |
| `./CLAUDE.md` | This project | Yes (team-shared) |
| `./CLAUDE.local.md` | This project, personal | No (gitignored) |
| `./packages/foo/CLAUDE.md` | When working in foo/ | Yes |

Parent CLAUDE.md files load automatically. Child CLAUDE.md files load on demand when Claude works with files in that directory. Keep root files focused on repo-wide instructions; push package-specific instructions into child CLAUDE.md files.

## Key principles

**CLAUDE.md is not a knowledge base.** It's a concise set of standing instructions. Paragraphs of explanation belong in reference docs, skills, or codebase documentation.

**Claude is an in-context learner.** If the codebase consistently follows a pattern, Claude will follow it. Don't repeat what the code already demonstrates.

**Less instructions, more compliance.** LLMs reliably follow ~150-200 instructions. Claude Code's system prompt already uses ~50. Every CLAUDE.md line competes for attention. Fewer, sharper instructions get followed more consistently than many diluted ones.

**Instructions degrade uniformly.** As instruction count increases, compliance drops across ALL instructions — not just the newer ones. One low-value instruction makes every other instruction less reliable.

**Linters are not Claude's job.** Use deterministic tools for style enforcement. Use hooks to run them automatically. Reserve CLAUDE.md for what only natural language instructions can express.

**Treat CLAUDE.md like code.** Review when things go wrong. Prune regularly. Test changes by observing Claude's behavior. Check into git so your team can contribute and iterate.
