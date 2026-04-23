# README Reference

Detailed guidance for writing each section, handling supporting files, and avoiding common mistakes.

## Contents

- [Section writing guide](#section-writing-guide)
- [Visual elements](#visual-elements)
- [Code examples](#code-examples)
- [Supporting files](#supporting-files)
- [Anti-patterns](#anti-patterns)

## Section writing guide

### Title and badges

- Project name as H1
- Logo or icon directly below title if available
- Badges in a single row below title or logo
- Only include badges that provide real value to the reader:
  - **High value:** CI status, latest version, license, platform support
  - **Medium value:** Downloads, code coverage, documentation status
  - **Low value:** Repo size, language percentage, contributor count
- For each badge ask: "does this help the reader answer one of the four questions?"

### Tagline / description

- One sentence or blockquote
- Front-load the problem it solves, not the technology used
- **Good:** "A fast, cross-platform tool for converting between mesh file formats"
- **Bad:** "A Python library built on top of numpy that provides classes and methods for..."
- If you can't explain it in one sentence, the project scope may need clarifying

### Highlights / features

- 3-6 bullets maximum
- Each bullet is one line
- Lead with the benefit, not the implementation
- **Good:** "Processes 10GB files without loading them into memory"
- **Bad:** "Uses streaming I/O with configurable buffer sizes"
- Most impressive or unique feature first

### Overview

- 1-2 paragraphs, no more
- Explain what it does, not how it's implemented
- Mention ecosystem context — how this fits with other tools
- Respectfully compare to alternatives if the comparison is favorable
- Include an Author/Credits subsection: name, link, brief motivation

### Usage / examples

- Start with the simplest possible example
- Show realistic, runnable code — not pseudocode
- Use syntax highlighting (`python`, `bash`, `javascript`, etc.)
- Include expected output when it clarifies the example
- For visual output, include screenshots or GIFs
- 2-3 examples maximum in the README — link to more
- Do NOT document the full API here — that belongs in dedicated docs

### Installation

- Lead with the one-liner for the most common package manager
- Add alternative methods below the primary one
- Specify minimum requirements: language version, OS, system dependencies
- Do not include dev setup here — those instructions go in CONTRIBUTING.md or the very bottom of the README
- Use `<details>` for alternative installation methods to keep the section scannable

### Contributing / feedback

- Invite engagement explicitly — don't just link, say "we'd love your feedback"
- Link to Discussions tab for questions and ideas
- Link to Issues for bug reports and feature requests
- Point to CONTRIBUTING.md for development setup and contribution process
- Consider a welcoming statement to lower the barrier for first-time contributors

### License

- Name the license and link to the LICENSE file
- One line: `[MIT](LICENSE)`

## Visual elements

### Screenshots and GIFs

- Screenshots are the most effective way to communicate what software does — most people skim looking at the pictures
- Place the primary screenshot near the top, before technical details
- For CLI tools, show terminal output with realistic data
- For libraries with visual output, show code + result
- Animated GIFs are excellent for demonstrating interactive features or workflows
- Keep images reasonable in file size (optimize for web)
- Always provide alt text for accessibility

### Badges

Badges communicate project health at a glance. Use them selectively.

Standard badge providers: [shields.io](https://shields.io) for custom badges, GitHub-native for CI status.

Layout: single row, grouped logically (build status | version | meta).

### Collapsible sections

Use `<details>` for content that's useful but not essential:

```html
<details>
<summary>Alternative installation methods</summary>

Content here...

</details>
```

Good candidates for collapsible sections: long feature lists, extensive examples, compatibility tables, citation info, build-from-source instructions, full changelogs.

## Code examples

- Always use fenced code blocks with language identifiers
- Examples must be copy-pasteable and runnable
- Include import statements — don't assume the reader knows them
- Show expected output when it clarifies what happened
- Use realistic variable names and data, not `foo`/`bar`
- Keep examples minimal — demonstrate one concept per example

**Terminal examples** — use `$` prompt prefix to distinguish commands from output:

```bash
$ my-tool convert data.json
Converting data.json → data.csv... done (42 rows)
```

## Supporting files

### CONTRIBUTING.md

Create for any project that accepts contributions. Include:

- Development environment setup (clone, install deps, run tests)
- Code style and conventions
- PR process (branch naming, commit format, review expectations)
- Issue guidelines (what makes a good bug report / feature request)
- How to run the test suite
- Optional: architecture overview for new contributors

The README says "See [CONTRIBUTING.md](CONTRIBUTING.md) for development instructions" — dev setup does not belong in the README.

### CHANGELOG.md

Create for projects with versioned releases. Follow [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
# Changelog

## [1.2.0] - 2024-03-15

### Added
- New feature X

### Fixed
- Bug in Y

## [1.1.0] - 2024-02-01
...
```

Categories: Added, Changed, Deprecated, Removed, Fixed, Security.

### CODE_OF_CONDUCT.md

Create for projects with community interaction. [Contributor Covenant](https://www.contributor-covenant.org/) is the most widely adopted standard.

### SECURITY.md

Create for projects that handle user data or have a significant user base. Include:

- How to report vulnerabilities (email, not public issues)
- Expected response timeline
- Supported versions receiving security updates

### LICENSE

Every project should have one. Common choices:

- **MIT** — Maximum permissiveness, most common for open source
- **Apache-2.0** — Permissive with patent protection
- **GPL-3.0** — Copyleft (derived works must also be open source)

## Anti-patterns

1. **The wall of text.** Paragraphs of prose without visual breaks. Use headers, bullets, code blocks, and images.

2. **The API dump.** Documenting every method in the README. Show 1-3 examples; full API belongs in dedicated docs.

3. **The absent README.** Just a title and a link to docs. At minimum: one-liner, install command, quick example.

4. **The dev-first README.** Leading with `git clone` and build instructions. Most users want a one-liner install, not a build from source.

5. **The stale README.** References to features that don't exist, commands that don't work, links that 404. Stale READMEs erode trust fast.

6. **The badge wall.** 15+ badges consuming the top of the README. Each badge should answer a question the reader actually has.

7. **The copy-paste template.** Template sections left with placeholder text or sections that don't apply. Fewer genuine sections beat many hollow ones.

8. **Missing screenshots.** For any project with visual output (apps, UIs, CLIs, plotting libraries), screenshots are essential.

9. **Burying the install.** Putting installation after 500 lines of documentation. People need to install before they can try anything.

10. **Dev instructions at the top.** `git clone ... && cmake ... && make` scares off casual users. Dev setup goes in CONTRIBUTING.md or at the very bottom.
