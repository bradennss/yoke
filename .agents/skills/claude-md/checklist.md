# CLAUDE.md Quality Checklist

Run through every item before finalizing a CLAUDE.md file. Items are grouped by category.

## Content quality

- [ ] Every line passes the "would removing this cause mistakes?" test
- [ ] No instructions Claude can infer from reading the code
- [ ] No standard language conventions Claude already knows
- [ ] No style rules a linter could enforce
- [ ] No self-evident instructions ("write clean code", "handle errors")
- [ ] No duplicated content from README or other docs
- [ ] No code snippets that will become stale (use file:line references instead)
- [ ] Short, direct sentences — not prose paragraphs
- [ ] Markdown headers, bullet points, and code blocks used for structure

## Length and focus

- [ ] Under 150 lines (under 60 is ideal)
- [ ] Does not exceed 300 lines
- [ ] All content is universally applicable across sessions
- [ ] No conditional instructions ("when working on X, do Y") — move to child CLAUDE.md or skills
- [ ] `IMPORTANT:` / `YOU MUST` used sparingly (2-3 maximum)

## Completeness (WHAT / WHY / HOW)

- [ ] Project purpose stated (one line, not a paragraph)
- [ ] Build/test/lint/typecheck commands included
- [ ] Verification steps specified (what to run after changes)
- [ ] Non-obvious workflow conventions documented (branches, PRs, commits)
- [ ] Critical architectural decisions noted with rationale
- [ ] Environment requirements listed (env vars, services, tools)
- [ ] Common gotchas and non-obvious behaviors captured

## Structure

- [ ] Critical instructions near the top of the file
- [ ] Related instructions grouped under clear headers
- [ ] Code blocks used for commands (not inline descriptions)
- [ ] No file-by-file codebase descriptions
- [ ] Verification section is present and specific

## Progressive disclosure

- [ ] Detailed domain knowledge in separate files, not in CLAUDE.md
- [ ] Reference files indexed with brief one-line descriptions
- [ ] @imports used judiciously (each increases context size)
- [ ] Pointers preferred over copies for code examples

## Hierarchy (when applicable)

- [ ] Root CLAUDE.md covers only repo-wide instructions
- [ ] Package/module CLAUDE.md files cover package-specific instructions
- [ ] CLAUDE.local.md used for personal overrides (added to .gitignore)
- [ ] No duplication between parent and child CLAUDE.md files

## Freshness

- [ ] All referenced files and commands still exist
- [ ] No stale information (outdated versions, deprecated patterns)
- [ ] No time-sensitive content (dates, version timelines)
- [ ] Instructions match what the codebase actually does, not aspirational conventions
