# Frontmatter & Feature Reference

Complete reference for SKILL.md frontmatter fields, string substitutions, shell injection, and file organization rules.

## Contents

- [Frontmatter \& Feature Reference](#frontmatter--feature-reference)
  - [Contents](#contents)
  - [Frontmatter fields](#frontmatter-fields)
    - [Core fields](#core-fields)
    - [Optional fields](#optional-fields)
  - [Invocation behavior matrix](#invocation-behavior-matrix)
  - [String substitutions](#string-substitutions)
  - [Shell injection](#shell-injection)
  - [Naming conventions](#naming-conventions)
  - [Description writing guide](#description-writing-guide)
  - [File organization rules](#file-organization-rules)
  - [Skill lifecycle](#skill-lifecycle)
  - [Anti-patterns](#anti-patterns)

## Frontmatter fields

### Core fields

| Field         | Required                        | Validation                                                                                                   | Description                                                                                                              |
| ------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `name`        | No (defaults to directory name) | Max 64 chars. Lowercase letters, numbers, hyphens only. No XML tags. Cannot contain "anthropic" or "claude". | The skill name. Becomes the `/slash-command`.                                                                            |
| `description` | Recommended                     | Max 1024 chars. Non-empty. No XML tags.                                                                      | What the skill does AND when to use it. Must be third person. Claude uses this for skill selection from 100+ candidates. |

### Optional fields

| Field                      | Default                | Description                                                                                                                                                                               |
| -------------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `when_to_use`              | None                   | Additional trigger context (phrases, scenarios). Appended to `description` in skill listing. Combined text capped at 1,536 chars.                                                         |
| `argument-hint`            | None                   | Autocomplete hint shown in `/` menu. Examples: `[issue-number]`, `[filename] [format]`.                                                                                                   |
| `disable-model-invocation` | `false`                | When `true`, Claude cannot auto-load this skill. User-only via `/name`. Use for side-effect actions (deploy, send, commit).                                                               |
| `user-invocable`           | `true`                 | When `false`, hidden from `/` menu. Claude-only background knowledge.                                                                                                                     |
| `allowed-tools`            | None                   | Tools Claude can use without permission prompts while skill is active. Space-separated string or YAML list. Does not restrict other tools. Example: `Bash(git add *) Bash(git commit *)`. |
| `model`                    | Inherited from session | Override model for this skill.                                                                                                                                                            |
| `effort`                   | Inherited from session | Override effort level. Options: `low`, `medium`, `high`, `xhigh`, `max`. Available levels depend on the model.                                                                            |
| `context`                  | None                   | Set to `fork` to run in an isolated subagent. Skill content becomes the subagent's task. No access to conversation history.                                                               |
| `agent`                    | `general-purpose`      | Subagent type when `context: fork`. Built-in: `Explore`, `Plan`, `general-purpose`. Also accepts custom agent names from `.claude/agents/`.                                               |
| `hooks`                    | None                   | Lifecycle hooks scoped to this skill. See Hooks documentation for format.                                                                                                                 |
| `paths`                    | None                   | Glob patterns that limit when skill auto-activates. Comma-separated string or YAML list. Claude loads the skill only when working with files matching these patterns.                     |
| `shell`                    | `bash`                 | Shell for `` !`command` `` and ` ```! ` blocks. Options: `bash`, `powershell`. PowerShell requires `CLAUDE_CODE_USE_POWERSHELL_TOOL=1`.                                                   |

## Invocation behavior matrix

| Frontmatter                      | User can invoke | Claude can invoke | Description in context |
| -------------------------------- | --------------- | ----------------- | ---------------------- |
| (defaults)                       | Yes             | Yes               | Yes                    |
| `disable-model-invocation: true` | Yes             | No                | No                     |
| `user-invocable: false`          | No              | Yes               | Yes                    |

## String substitutions

Available in skill body content. Replaced before content reaches Claude.

| Variable                | Description                                                                                                                      |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `$ARGUMENTS`            | All arguments passed when invoking the skill. If not present in content, arguments are appended as `ARGUMENTS: <value>`.         |
| `$ARGUMENTS[N]` or `$N` | Access a specific argument by 0-based index. `$ARGUMENTS[0]` and `$0` are equivalent.                                            |
| `${CLAUDE_SKILL_DIR}`   | Absolute path to the directory containing this SKILL.md. Use to reference bundled scripts/files regardless of working directory. |
| `${CLAUDE_SESSION_ID}`  | Current session ID. Useful for session-specific logs or files.                                                                   |

Indexed arguments use shell-style quoting: `/my-skill "hello world" second` makes `$0` = `hello world`, `$1` = `second`. `$ARGUMENTS` always expands to the full string as typed.

**Example:**

```yaml
---
name: fix-issue
description: Fixes a GitHub issue by number
disable-model-invocation: true
---
Fix GitHub issue $ARGUMENTS following our coding standards.
```

`/fix-issue 123` becomes "Fix GitHub issue 123 following our coding standards."

## Shell injection

Commands that run before skill content reaches Claude. Output replaces the placeholder.

**Inline form:**

```markdown
Current branch: !`git branch --show-current`
Repo status: !`git status --short`
```

**Multi-line form:**

````markdown
```!
node --version
npm --version
git status --short
```
````

**Important:** This is preprocessing, not something Claude executes. Claude only sees the command output in the rendered content.

Disable with `"disableSkillShellExecution": true` in settings. Each command is replaced with `[shell command execution disabled by policy]`.

## Naming conventions

**Preferred: gerund form** (clearly describes the activity):

- `processing-pdfs`
- `analyzing-spreadsheets`
- `managing-databases`
- `testing-code`
- `writing-documentation`

**Acceptable alternatives:**

- Noun phrases: `pdf-processing`, `spreadsheet-analysis`
- Action-oriented: `process-pdfs`, `analyze-spreadsheets`

**Avoid:**

- Vague: `helper`, `utils`, `tools`, `misc`
- Generic: `documents`, `data`, `files`
- Reserved words: `anthropic-helper`, `claude-tools`
- Inconsistent patterns within a skill collection

## Description writing guide

The description is THE most critical field. Claude uses it to decide which skill to load from potentially 100+ skills. A bad description means the skill never activates when needed or activates when it shouldn't.

**Rules:**

1. Write in third person always. "Processes files" not "I can process files" or "Use this to process files."
2. Include WHAT the skill does in the first clause.
3. Include WHEN to use it, with trigger keywords users would naturally say.
4. Front-load the primary use case before edge cases.
5. Be specific. "Extracts text and tables from PDF files" not "Helps with documents."

**Strong examples:**

```yaml
# Specific, includes triggers
description: Extracts text and tables from PDF files, fills forms, merges documents. Use when working with PDF files or when the user mentions PDFs, forms, or document extraction.

# Clear scope, action keywords
description: Generates descriptive commit messages by analyzing git diffs. Use when the user asks for help writing commit messages or reviewing staged changes.

# Domain-specific, with context signals
description: Analyzes BigQuery datasets for sales metrics, pipeline health, and revenue trends. Use when the user asks about sales data, pipeline, win rates, or revenue analysis.
```

**Weak examples and why:**

```yaml
description: Helps with documents       # Too vague, no triggers
description: Processes data              # No specificity, matches everything
description: Does stuff with files       # Meaningless
description: A tool for working with PDFs # "A tool for" is noise
```

## File organization rules

```
my-skill/
├── SKILL.md              # Required. Main entry point. Under 500 lines.
├── reference.md          # Optional. Detailed API/field docs.
├── examples.md           # Optional. Usage examples.
├── templates/            # Optional. Output templates.
│   └── report.md
└── scripts/              # Optional. Utility scripts.
    └── validate.py
```

1. **SKILL.md is the only required file.** Everything else is optional.
2. **Supporting files load on demand.** Zero context cost until Claude reads them. Bundle as much reference material as needed.
3. **Reference every supporting file from SKILL.md.** Claude needs to know files exist and what they contain. Brief, one-line descriptions.
4. **Keep references one level deep.** SKILL.md links to files. Files do NOT link to further files. Nested references cause partial reads.
5. **Name files descriptively.** `form-validation-rules.md` not `doc2.md`. Names should indicate content at a glance.
6. **Forward slashes in all paths.** `scripts/helper.py` not `scripts\helper.py`. Unix paths work everywhere; Windows paths break on Unix.
7. **Table of contents for files over 100 lines.** Ensures Claude sees the full scope even when previewing.
8. **Organize by domain or feature.** `reference/finance.md`, `reference/sales.md` — not `docs/file1.md`, `docs/file2.md`.
9. **Scripts execute, not load.** When Claude runs `validate.py`, only the script's output enters context. The script code itself never loads. This makes scripts far more efficient than generated code.
10. **Make execution intent explicit.** "Run `analyze.py` to extract fields" (execute) vs "See `analyze.py` for the extraction algorithm" (read as reference).

## Skill lifecycle

**Loading stages:**

| Stage     | What loads                             | Token cost                       | When                              |
| --------- | -------------------------------------- | -------------------------------- | --------------------------------- |
| Startup   | `name` + `description` from all skills | ~100 tokens per skill            | Always                            |
| Triggered | SKILL.md body                          | Varies (aim for under 5K tokens) | When skill matches user's request |
| On demand | Supporting files                       | Unlimited                        | When Claude follows references    |
| Execution | Script output only                     | Varies                           | When Claude runs a bundled script |

**After invocation:** Rendered SKILL.md enters the conversation as a single message for the rest of the session. Claude does not re-read the file on later turns. Write guidance as standing instructions, not one-time steps.

**Auto-compaction:** Claude Code re-attaches the most recent invocation of each skill after compaction, keeping the first 5,000 tokens each. Combined budget: 25,000 tokens across all active skills, filled from most recently invoked. Older skills may be dropped entirely if many are active.

**Live changes:** Edits to existing skills take effect within the current session without restart. A new top-level skills directory requires a restart.

## Anti-patterns

**Avoid these common mistakes:**

1. **Over-explaining.** Don't tell Claude what PDFs are or how pip works. Only include knowledge Claude doesn't already have.

2. **Too many options.** Don't present five libraries and let Claude choose. Provide one recommended default with an escape hatch for edge cases.

3. **Deeply nested references.** SKILL.md → advanced.md → details.md causes partial reads. Keep everything one level deep.

4. **Vague descriptions.** "Helps with files" matches everything and nothing. Be specific about capability and trigger conditions.

5. **Time-sensitive content.** "If before August 2025, use the old API" will become wrong. Use an "old patterns" section with `<details>` tags.

6. **Inconsistent terminology.** Alternating between "field", "box", "element", and "control" for the same concept confuses Claude.

7. **Magic numbers in scripts.** `TIMEOUT = 47` — why 47? Document the reasoning for every constant.

8. **Punting errors to Claude.** Scripts should handle errors explicitly with helpful messages, not fail with raw stack traces.

9. **Windows-style paths.** `scripts\helper.py` breaks on Unix. Always use forward slashes.

10. **Assuming package availability.** Always list required packages and include install instructions. Don't assume anything is pre-installed.

11. **Writing descriptions in first or second person.** "I can help you process PDFs" causes discovery problems. Always use third person: "Processes PDF files."

12. **Including content Claude already knows.** Every token in a skill competes with conversation history. Trim aggressively.
