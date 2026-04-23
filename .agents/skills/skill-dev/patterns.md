# Skill Patterns

Common structural patterns for organizing skill content, with examples.

## Contents

- [Skill Patterns](#skill-patterns)
  - [Contents](#contents)
  - [Template pattern](#template-pattern)
  - [Examples pattern](#examples-pattern)
  - [Conditional workflow pattern](#conditional-workflow-pattern)
  - [Workflow with checklist pattern](#workflow-with-checklist-pattern)
  - [Feedback loop pattern](#feedback-loop-pattern)
  - [Progressive disclosure pattern](#progressive-disclosure-pattern)
  - [Dynamic context injection pattern](#dynamic-context-injection-pattern)
  - [Subagent execution pattern](#subagent-execution-pattern)
  - [Default tool pattern](#default-tool-pattern)
  - [Complete skill example](#complete-skill-example)

## Template pattern

Provide output format templates. Match strictness to requirements.

**Strict** (exact format required — use for API responses, data formats, compliance docs):

````markdown
ALWAYS use this exact structure:

```markdown
# [Title]

## Summary

[One-paragraph overview]

## Findings

- Finding 1 with supporting data
- Finding 2 with supporting data

## Recommendations

1. Specific actionable recommendation
2. Specific actionable recommendation
```
````

**Flexible** (adaptation welcome — use for reports, reviews, analyses):

````markdown
Sensible default format — adjust sections as needed:

```markdown
# [Title]

## Summary

[Overview]

## Details

[Adapt based on what you discover]
```
````

## Examples pattern

Input/output pairs when output quality depends on demonstration. More effective than describing the desired style in prose.

````markdown
## Commit message format

**Example 1:**
Input: Added user authentication with JWT tokens
Output:

```
feat(auth): implement JWT-based authentication

Add login endpoint and token validation middleware
```

**Example 2:**
Input: Fixed bug where dates displayed incorrectly in reports
Output:

```
fix(reports): correct date formatting in timezone conversion

Use UTC timestamps consistently across report generation
```

**Example 3:**
Input: Updated dependencies and refactored error handling
Output:

```
chore: update dependencies and refactor error handling

- Upgrade lodash to 4.17.21
- Standardize error response format across endpoints
```

Follow this style: type(scope): brief description, then detailed explanation.
````

## Conditional workflow pattern

Guide Claude through decision points. Each branch should lead to clear, actionable instructions.

```markdown
## Document modification

1. Determine the modification type:

   **Creating new content?** -> Follow "Creation workflow" below
   **Editing existing content?** -> Follow "Editing workflow" below

2. Creation workflow:
   - Use docx-js library
   - Build document from scratch
   - Export to .docx format

3. Editing workflow:
   - Unpack existing document
   - Modify XML directly
   - Validate after each change
   - Repack when complete
```

When workflows become large, push each branch into a separate file and tell Claude to read the appropriate one.

## Workflow with checklist pattern

For complex multi-step tasks. The copy-paste checklist lets Claude (and the user) track progress and prevents steps from being skipped.

````markdown
## Deployment workflow

Copy this checklist and track progress:

```
- [ ] Step 1: Run test suite
- [ ] Step 2: Build application
- [ ] Step 3: Push to staging
- [ ] Step 4: Verify staging deployment
- [ ] Step 5: Promote to production
- [ ] Step 6: Verify production
```

**Step 1: Run test suite**

```bash
npm test -- --ci
```

All tests must pass. If any fail, fix before proceeding.

**Step 2: Build application**

```bash
npm run build
```

Verify the build output in `dist/` contains the expected files.

**Step 3: Push to staging**

```bash
./scripts/deploy.sh staging
```

**Step 4: Verify staging deployment**

Check the staging URL. Verify core flows work end-to-end.
If issues found, fix and return to Step 1.

**Step 5: Promote to production**

```bash
./scripts/deploy.sh production
```

**Step 6: Verify production**

Check the production URL. Verify the same core flows.
If issues found, roll back immediately and investigate.
````

## Feedback loop pattern

Validate, fix, repeat. This pattern dramatically improves output quality for tasks where mistakes are costly.

```markdown
## Editing process

1. Make your edits
2. **Validate immediately**: `python scripts/validate.py output/`
3. If validation fails:
   - Review the error message carefully
   - Fix the specific issue identified
   - Run validation again
4. **Only proceed when validation passes**
5. Finalize output
```

For tasks without scripts, the "validator" can be a reference document:

```markdown
## Content review

1. Draft the content
2. Review against style-guide.md:
   - Terminology consistency
   - Example format compliance
   - Required sections present
3. If issues found:
   - Note each issue with section reference
   - Revise the content
   - Review again
4. Finalize when all requirements are met
```

## Progressive disclosure pattern

Keep SKILL.md as a high-level guide. Detailed content lives in supporting files and loads on demand — zero context cost until needed.

**Single-domain skill:**

````markdown
---
name: pdf-processing
description: Extracts text and tables from PDF files, fills forms, merges documents. Use when working with PDF files or when the user mentions PDFs, forms, or document extraction.
---

# PDF Processing

## Quick start

```python
import pdfplumber
with pdfplumber.open("file.pdf") as pdf:
    text = pdf.pages[0].extract_text()
```

## Advanced features

**Form filling**: See [forms.md](forms.md) for the complete guide
**API reference**: See [reference.md](reference.md) for all methods
**Examples**: See [examples.md](examples.md) for common patterns
````

**Multi-domain skill:**

````markdown
---
name: data-analysis
description: Analyzes datasets using SQL and Python. Use when working with data, queries, reports, or analytics.
---

# Data Analysis

## Available datasets

**Finance**: Revenue, ARR, billing -> See [reference/finance.md](reference/finance.md)
**Sales**: Opportunities, pipeline -> See [reference/sales.md](reference/sales.md)
**Product**: API usage, features -> See [reference/product.md](reference/product.md)

## Quick search

Find specific metrics:

```bash
grep -i "revenue" reference/finance.md
grep -i "pipeline" reference/sales.md
```
````

When a user asks about revenue, Claude reads only `reference/finance.md`. The sales and product files stay on disk at zero cost.

## Dynamic context injection pattern

Use `` !`command` `` to inject live data before Claude sees the content. Commands run during skill loading as preprocessing — Claude only sees the output.

```yaml
---
name: pr-review
description: Reviews the current pull request
context: fork
agent: Explore
allowed-tools: Bash(gh *)
---

## PR context
- Diff: !`gh pr diff`
- Comments: !`gh pr view --comments`
- Changed files: !`gh pr diff --name-only`

## Review instructions
Analyze the changes above and provide feedback on:
1. Correctness and edge cases
2. Code quality and readability
3. Test coverage
```

**Multi-line form:**

````markdown
## Environment

```!
node --version
npm --version
git status --short
```
````

## Subagent execution pattern

Use `context: fork` for tasks that should run in isolation without conversation history. The skill content becomes the subagent's prompt.

```yaml
---
name: deep-research
description: Researches a topic thoroughly across the codebase
context: fork
agent: Explore
---

Research $ARGUMENTS thoroughly:

1. Find relevant files using Glob and Grep
2. Read and analyze the code
3. Summarize findings with specific file references
```

**When to use `context: fork`:**

- Research/exploration tasks that shouldn't pollute conversation context
- Tasks that benefit from a clean context (no prior conversation bias)
- Parallel work that can run independently

**When NOT to use `context: fork`:**

- Reference knowledge (conventions, patterns) that should apply to ongoing work
- Skills that need access to conversation history or current context

## Default tool pattern

Provide one recommended approach with an escape hatch. Don't present a menu of options.

````markdown
Use pdfplumber for text extraction:

```python
import pdfplumber
with pdfplumber.open("file.pdf") as pdf:
    text = pdf.pages[0].extract_text()
```

For scanned PDFs requiring OCR, use pdf2image with pytesseract instead.
````

**Not this:**

```markdown
You can use pypdf, pdfplumber, PyMuPDF, pdf2image, or camelot...
```

## Complete skill example

A well-structured skill demonstrating multiple patterns:

````yaml
---
name: reviewing-pull-requests
description: Reviews pull requests for code quality, correctness, and adherence to project conventions. Use when the user asks to review a PR, check code changes, or provide feedback on a pull request.
disable-model-invocation: true
allowed-tools: Bash(gh *)
argument-hint: "[PR-number]"
---

# Pull Request Review

## Workflow

```
- [ ] Step 1: Fetch PR context
- [ ] Step 2: Review changes
- [ ] Step 3: Check test coverage
- [ ] Step 4: Post review
```

**Step 1: Fetch PR context**

```bash
gh pr view $ARGUMENTS
gh pr diff $ARGUMENTS
```

**Step 2: Review changes**

For each changed file, assess:
1. Correctness: Logic errors, edge cases, race conditions
2. Readability: Clear naming, appropriate abstractions, no unnecessary complexity
3. Conventions: Follows project patterns (see project CLAUDE.md)

**Step 3: Check test coverage**

Verify that:
- New functionality has tests
- Modified functionality has updated tests
- Edge cases identified in Step 2 are covered

**Step 4: Post review**

Summarize findings. Use this format:

```markdown
## Summary
[1-2 sentence overview]

## Issues found
- **[severity]** file:line — description

## Suggestions
- file:line — suggestion
```

If no issues: approve. If blocking issues: request changes with clear remediation steps.
````

This example demonstrates:

- Clear frontmatter with specific description and trigger keywords
- `disable-model-invocation: true` because reviews are intentional actions
- `allowed-tools` to pre-approve the tools needed
- `argument-hint` for autocomplete guidance
- Workflow with checklist for tracking
- Template pattern for output format
- Concrete instructions at each step
