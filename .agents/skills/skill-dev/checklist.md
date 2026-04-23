# Skill Quality Checklist

Run through every item before finalizing a skill. Items are grouped by category.

## Frontmatter

- [ ] `name` uses only lowercase letters, numbers, and hyphens
- [ ] `name` is 64 characters or fewer
- [ ] `name` does not contain "anthropic" or "claude"
- [ ] `name` is specific, not vague (`pdf-form-filling` not `helper`)
- [ ] `description` is non-empty and under 1,024 characters
- [ ] `description` is written in third person ("Processes files" not "I process files")
- [ ] `description` states WHAT the skill does AND WHEN to use it
- [ ] `description` includes keywords users would naturally say
- [ ] `description` front-loads the primary use case
- [ ] No XML tags in `name` or `description`
- [ ] Optional fields are included only when they serve a purpose
- [ ] `disable-model-invocation: true` is set for side-effect actions (deploy, send, commit)

## Content quality

- [ ] SKILL.md body is under 500 lines
- [ ] Every paragraph earns its token cost (no filler, no Claude-obvious explanations)
- [ ] Quick start / common case comes first, edge cases later
- [ ] Concrete examples used instead of abstract descriptions
- [ ] One term per concept used consistently throughout
- [ ] No time-sensitive information (dates, version timelines)
- [ ] Degrees of freedom match the task's fragility
- [ ] Instructions are actionable, not just informational
- [ ] Provides one recommended default, not a menu of options

## Structure and progressive disclosure

- [ ] Supporting files are referenced from SKILL.md with brief descriptions
- [ ] All references are one level deep (no chains: A -> B -> C)
- [ ] Files over 100 lines have a table of contents
- [ ] File names are descriptive (`form-validation-rules.md` not `doc2.md`)
- [ ] Forward slashes in all file paths
- [ ] Directory organized by domain or feature, not file type

## Workflows (when applicable)

- [ ] Complex tasks have clear sequential steps
- [ ] Multi-step workflows include a copy-paste checklist
- [ ] Validation/feedback loops for quality-critical operations (validate -> fix -> repeat)
- [ ] Conditional workflows have explicit decision points
- [ ] Large workflows are in separate files, not crowding SKILL.md

## Scripts (when applicable)

- [ ] Scripts handle errors explicitly with helpful messages (no punting to Claude)
- [ ] All constants are documented with reasoning (no magic numbers)
- [ ] Required packages are listed in instructions with install commands
- [ ] Execution intent is explicit: "Run `script.py`" (execute) vs "See `script.py`" (read)
- [ ] Validation scripts provide specific error messages naming the problem and available alternatives
- [ ] Scripts are self-contained and don't assume package availability

## Testing

- [ ] Tested with direct invocation (`/skill-name`) — behavior matches intent
- [ ] Tested with automatic discovery — asking matching questions triggers the skill
- [ ] Observed Claude navigating supporting files correctly
- [ ] Tested with target models if cross-model use is planned (Haiku needs more guidance than Opus)
- [ ] Iterated based on observed behavior, not assumptions
