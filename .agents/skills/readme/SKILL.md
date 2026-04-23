---
name: readme
description: Authors and maintains README files and supporting project documentation (CONTRIBUTING, CHANGELOG, CODE_OF_CONDUCT, SECURITY). Adapts README structure to the project type — library, application, CLI tool, framework, starter template, or collection. Use when creating, rewriting, reviewing, or improving a README or related project documentation.
when_to_use: When the user asks to create, write, improve, review, or update a README, or when working with CONTRIBUTING.md, CHANGELOG.md, CODE_OF_CONDUCT.md, SECURITY.md, or other standard project documentation.
argument-hint: "[path-to-project or path-to-existing-readme]"
effort: max
---

# README Authoring Guide

Follow this guide when creating or improving READMEs. For format templates by project type see [formats.md](formats.md). For detailed section-writing guidance see [reference.md](reference.md).

## The core principle

> *Your documentation is a direct reflection of your software — hold it to the same standards.*

A README is often the first and only thing anyone sees about a project. People judge software by its README. A poorly written README translates to poorly written software in readers' minds. The README is the best marketing material you have.

## Creating a README

### Step 1: Determine the project type

The project type dictates the README structure, audience, and emphasis. Get this right first — everything else follows.

| Project type | Audience | Goal | Key emphasis |
|---|---|---|---|
| Library / Package | Developers seeking a dependency | Convince them to install | Code examples, one-liner install |
| Application | End users | Get them to download and use | Screenshots, download/install |
| CLI Tool | Power users, developers | Show command-line capability | Command examples with output |
| Framework / Platform | Developers evaluating tools | Show breadth, enable quick start | Quickstart, showcase |
| Starter / Template | Developers starting a project | Clone and run immediately | Quick start, project structure |
| Collection / List | Learners, practitioners | Navigate to resources | Table of contents, organization |

If the project has a dedicated documentation site, the README becomes a **minimal elevator pitch and link fest** regardless of type — don't duplicate the docs.

See [formats.md](formats.md) for the complete section structure and template for each type.

### Step 2: Study the project

Before writing, gather the material:

1. **Identity**: What does this project do? One sentence.
2. **Selling points**: What are the 3-5 most compelling features or capabilities?
3. **Installation**: How does a user install or get this? What's the simplest path?
4. **Quick demo**: What's the shortest code/command/screenshot that shows it working?
5. **Requirements**: What platforms, languages, or dependencies are needed?
6. **Links**: Documentation, CI, deployments, community, issue tracker.
7. **Authors**: Who made this and how to reach them?
8. **License**: What are the usage terms?

Read the existing codebase, package manifests (`package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, etc.), existing docs, CI config, and any prior README.

### Step 3: Draft the README

Follow the format template from [formats.md](formats.md) for the identified project type.

**Writing principles:**

- **Answer four questions fast.** Every reader asks: (1) Does this solve my problem? (2) Can I use this? (3) Who made this? (4) How can I learn more? Structure the README so a quick scan answers all four.
- **Cognitive funneling.** Broad and most pertinent at the top, increasingly specific as you scroll down. Name → tagline → visual/example → details → install → license.
- **Show, don't tell.** Screenshots, animated GIFs, and code examples beat prose. Most people skim looking at pictures.
- **Keep it concise.** The ideal README is as short as it can be without being any shorter.
- **Gear toward the average user.** Assume someone less familiar with your domain is reading. Make the software feel approachable.
- **One-liner installs.** Show the simplest installation command first. Save dev setup for CONTRIBUTING.md or the very bottom.
- **Sell the best part first.** Find the most appealing aspect of the software and lead with it.

**Formatting rules:**

- Syntax-highlighted code blocks for all code and commands
- Badges for CI status, version, license — only badges that provide real value
- `<details>` for useful-but-not-essential content (long lists, alternative install methods)
- Clear, scannable section headers

### Step 4: Handle supporting files

A README doesn't exist in isolation. Create supporting files when appropriate:

| File | When to create | What it contains |
|---|---|---|
| `CONTRIBUTING.md` | Open source projects accepting contributions | Dev setup, coding standards, PR process |
| `CHANGELOG.md` | Projects with versioned releases | Release history (Keep a Changelog format) |
| `CODE_OF_CONDUCT.md` | Projects with community interaction | Community behavior standards |
| `SECURITY.md` | Projects handling sensitive data or with users | Vulnerability reporting process |
| `LICENSE` | Always (if not already present) | License text |

The README links to these files where relevant. Never duplicate their content in the README.

See [reference.md](reference.md) for detailed guidance on each supporting file.

### Step 5: Verify quality

Run through every item in [checklist.md](checklist.md) before finalizing.

## Improving an existing README

When reviewing or improving an existing README:

1. **Identify the project type** and compare against [formats.md](formats.md)
2. **Test the four questions**: Can a reader quickly determine what this does, how to use it, who made it, and where to learn more?
3. **Check conciseness**: Filler, redundancy, or content that belongs in docs?
4. **Check visual appeal**: Code examples, screenshots, badges? Is it scannable?
5. **Check installation**: Is the install path clear and simple?
6. **Check freshness**: Do referenced links, commands, and features still exist?
7. **Run the checklist**: [checklist.md](checklist.md)
8. **Propose specific changes**: Organized as add / remove / rewrite / move to supporting file

## Reference

- **Formats**: [formats.md](formats.md) — Section templates for each project type (library, app, CLI, framework, starter, collection)
- **Writing guide**: [reference.md](reference.md) — Section-by-section content guidance, supporting files, anti-patterns
- **Checklist**: [checklist.md](checklist.md) — Quality verification checklist
