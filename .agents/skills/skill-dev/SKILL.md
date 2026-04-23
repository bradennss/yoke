---
name: skill-dev
description: Authors, reviews, and maintains Claude Code skills. Guides the full lifecycle from initial design through quality verification, ensuring skills follow best practices for structure, discoverability, and token efficiency.
when_to_use: When creating a new skill, writing a SKILL.md file, improving or reviewing an existing skill, or discussing skill architecture. Also when the user mentions authoring skills, skill best practices, or asks how to structure a skill.
argument-hint: "[skill-name or path-to-existing-skill]"
effort: max
---

# Skill Authoring Guide

Follow this guide when creating new skills or reviewing existing ones. For complete field documentation see [reference.md](reference.md). For structural examples see [patterns.md](patterns.md).

## Creating a skill

### Step 1: Define scope and intent

Before writing anything, determine:

**What type of content?**

- **Reference content**: Knowledge Claude applies to ongoing work (conventions, patterns, style guides). Runs inline alongside conversation context.
- **Task content**: Step-by-step instructions for a specific action (deploy, generate, analyze). Often invoked directly via `/skill-name`.

**Who invokes it?**

- **Both user and Claude** (default): Good for most skills. Claude auto-loads when relevant; user can also invoke directly.
- **User only** (`disable-model-invocation: true`): For side-effect actions like deploy, send-message, commit. Prevents Claude from triggering it unprompted.
- **Claude only** (`user-invocable: false`): For background knowledge. Not a meaningful action for users to take directly.

**How much freedom should Claude have?**

- **High freedom** (text guidance): Multiple valid approaches, context determines best path. Example: code review guidelines.
- **Medium freedom** (templates/pseudocode): A preferred pattern exists but some variation is acceptable. Example: report generation with configurable sections.
- **Low freedom** (exact scripts/commands): Fragile or critical operations where consistency matters. Example: database migrations.

### Step 2: Choose location

| Location   | Path                               | Scope                   |
| ---------- | ---------------------------------- | ----------------------- |
| Personal   | `~/.claude/skills/<name>/SKILL.md` | All your projects       |
| Project    | `.claude/skills/<name>/SKILL.md`   | This project only       |
| Plugin     | `<plugin>/skills/<name>/SKILL.md`  | Where plugin is enabled |
| Enterprise | Via managed settings               | All org users           |

Higher-priority locations win when names collide: enterprise > personal > project. Plugin skills use `plugin-name:skill-name` namespacing and cannot conflict.

If working within a skills collection repository, create skills in the repo's `skills/<name>/SKILL.md` directory.

### Step 3: Write the frontmatter

The `description` is the most critical field. Claude uses it to select the right skill from potentially 100+ available skills. Get this wrong and the skill won't activate when needed.

```yaml
---
name: your-skill-name
description: What the skill does and when to use it. Written in third person.
---
```

**Name rules:**

- Lowercase letters, numbers, and hyphens only
- Max 64 characters
- Cannot contain "anthropic" or "claude"
- Cannot contain XML tags
- Prefer gerund form: `processing-pdfs`, `analyzing-data`, `writing-tests`
- Be specific: `pdf-form-filling` not `helper`
- Stay consistent across a skill collection

**Description rules:**

- Third person always ("Processes files" not "I process files" or "You can use this")
- Include WHAT it does AND WHEN to use it
- Front-load the primary use case
- Include trigger keywords users would naturally say
- Max 1024 characters, no XML tags
- Combined `description` + `when_to_use` is capped at 1,536 characters in the skill listing

**Good description example:**

```yaml
description: Extracts text and tables from PDF files, fills forms, merges documents. Use when working with PDF files or when the user mentions PDFs, forms, or document extraction.
```

**Bad descriptions:** "Helps with documents", "Processes data", "Does stuff with files"

Only add optional frontmatter fields when they serve a purpose. See [reference.md](reference.md) for the complete field list.

### Step 4: Write the body

**The default assumption: Claude is already smart.** Only include information Claude doesn't already have. Challenge every paragraph: "Does Claude really need this? Does this justify its token cost?"

**Concise example** (~50 tokens):

````markdown
Extract text with pdfplumber:

```python
import pdfplumber
with pdfplumber.open("file.pdf") as pdf:
    text = pdf.pages[0].extract_text()
```
````

**Verbose anti-example** (~150 tokens): Explaining what PDFs are, that libraries exist, and how pip works before showing the same code.

**Body structure guidelines:**

1. Lead with the common case. Put quick-start instructions first, edge cases later.
2. Use concrete examples over abstract descriptions. Input/output pairs beat prose.
3. Pick one term per concept and use it consistently. Don't alternate between "API endpoint", "URL", "route", and "path".
4. No time-sensitive content. Don't reference specific dates or version timelines. Use an "old patterns" section with `<details>` for deprecated approaches.
5. Provide a default approach, not a menu of options. One recommended tool with an escape hatch beats listing five alternatives.
6. For workflows, break complex tasks into clear sequential steps. Include a copy-paste checklist for multi-step operations.
7. For feedback-sensitive tasks, include validation loops: run validator, fix errors, repeat until clean.

**Structure for simple skills:**

```markdown
# Skill Title

Brief purpose statement.

## Instructions

[Concise, actionable guidance]

## Examples

[Concrete input/output pairs if the output format matters]
```

**Structure for complex skills:**

```markdown
# Skill Title

Brief purpose statement.

## Quick start

[Essential instructions for the common case]

## Advanced features

**Topic A**: See [topic-a.md](topic-a.md)
**Topic B**: See [topic-b.md](topic-b.md)

## Workflows

[Step-by-step instructions with checklists for complex operations]
```

See [patterns.md](patterns.md) for detailed examples of each structural pattern.

### Step 5: Organize supporting files

Only needed when SKILL.md would exceed ~300 lines or when content is domain-specific enough that not all of it is relevant to every invocation.

```
my-skill/
├── SKILL.md              # Required. Main entry point.
├── reference.md          # Detailed docs (loaded on demand)
├── examples.md           # Usage examples (loaded on demand)
└── scripts/
    └── helper.py         # Utility scripts (executed, not loaded)
```

**Rules:**

- Reference every supporting file from SKILL.md so Claude knows it exists and what it contains
- Keep references one level deep: SKILL.md links to files, files do NOT link to other files
- Name files descriptively: `form-validation-rules.md` not `doc2.md`
- Add a table of contents to files over 100 lines
- Forward slashes in all paths, even on Windows
- Organize by domain or feature, not by file type

**Scripts vs instructions:**

- Scripts are executed via bash. Their code never enters context; only output does. Prefer scripts for deterministic operations.
- Make execution intent explicit: "Run `analyze.py`" (execute) vs "See `analyze.py` for the algorithm" (read as reference)
- Scripts should handle errors explicitly, not punt failures to Claude
- Document all constants (no "magic numbers")
- List required packages in SKILL.md and verify availability

### Step 6: Verify quality

Run through every item in [checklist.md](checklist.md) before finalizing. The checklist covers frontmatter, content quality, structure, workflows, scripts, and testing.

### Step 7: Test and iterate

1. **Direct invocation**: Run `/skill-name` and verify behavior matches intent
2. **Automatic discovery**: Ask questions using natural language that should trigger the skill. Verify Claude loads it
3. **Navigation**: Observe how Claude navigates supporting files. Does it find the right file? Does it read unnecessary files?
4. **Model coverage**: If the skill will be used across models, test with each. Haiku needs more guidance than Opus
5. **Iterate from observation, not assumptions**: If Claude misses a rule, make it more prominent. If Claude ignores a file, improve its signal in SKILL.md. If Claude over-reads, restructure for better progressive disclosure

## Reviewing an existing skill

When asked to review or improve a skill:

1. **Read everything**: SKILL.md and all supporting files
2. **Check frontmatter**: Validate against the rules in [reference.md](reference.md)
3. **Run the checklist**: Go through [checklist.md](checklist.md) item by item
4. **Assess discoverability**: Is the description specific enough? Does it include trigger keywords?
5. **Assess conciseness**: Is every paragraph earning its token cost? Is Claude being told things it already knows?
6. **Assess structure**: Is progressive disclosure used appropriately? Are references one level deep?
7. **Report issues by category** (frontmatter, content, structure, testing) with concrete fixes

## Key concepts to remember

**Context window economics**: At startup, only skill metadata (name + description, ~100 tokens each) is loaded. The full SKILL.md loads when triggered. Supporting files load on demand. Scripts execute without loading their code. This means skills can bundle extensive resources with zero context cost until needed.

**Skill lifecycle in Claude Code**: When invoked, rendered SKILL.md enters the conversation for the remainder of the session. Claude does not re-read the file on later turns. Auto-compaction retains the first 5,000 tokens of each skill, with a combined budget of 25,000 tokens across all active skills, filled from most recently invoked.

**Live change detection**: Claude Code watches skill directories. Edits to existing skills take effect within the current session. Creating a new top-level skills directory requires restarting Claude Code.
