# README Formats by Project Type

Section structures and templates for each project type. Choose the format that matches the project, then adapt sections as needed.

## Contents

- [Library / Package](#library--package)
- [Application](#application)
- [CLI Tool](#cli-tool)
- [Framework / Platform](#framework--platform)
- [Starter / Template](#starter--template)
- [Collection / Curated List](#collection--curated-list)
- [Mature Project (has docs site)](#mature-project-has-docs-site)

## Library / Package

For installable packages, modules, or libraries consumed as a dependency.

**Audience:** Developers evaluating whether to add this to their project.
**Goal:** Convince them to install it with a code example and a one-liner.

### Section order

1. **Name + Badges** — Project name as H1. CI status, version, license badges.
2. **Tagline** — One sentence or blockquote. Front-load the problem it solves.
3. **Highlights** — 3-6 bullet selling points. Most compelling first.
4. **Overview** — 1-2 paragraphs on what it does and why. Optionally compare to alternatives. Author/Credits subsection.
5. **Usage** — Minimal code examples with syntax highlighting. Screenshots or GIFs for visual output. Do not document the full API here.
6. **Installation** — One-liner package manager command. Minimum requirements (language version, OS). No dev instructions.
7. **Contributing** — Invite feedback. Link to Discussions/Issues. Point to CONTRIBUTING.md.
8. **License** — Name and link to LICENSE file.

### Template

````markdown
# my-library

[![CI](badge-url)](ci-url) [![PyPI](badge-url)](pypi-url) [![License](badge-url)](license-url)

> A short, compelling description of what this library does and why you need it.

## Highlights

- Feature one — the most impressive thing
- Feature two — solves a common pain point
- Feature three — differentiator from alternatives

## Overview

A paragraph explaining the library in more depth. What problem does it solve? How does it approach the problem? What makes it different?

### Authors

Built by [Name](link). Contributions welcome.

## Usage

```python
import my_library

result = my_library.do_thing("input")
print(result)
```

![Screenshot or GIF of output](./docs/example.png)

## Installation

```bash
pip install my-library
```

Requires Python 3.9+.

## Contributing

Found a bug? Have an idea? Open an [issue](link) or start a [discussion](link).

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup.

## License

[MIT](LICENSE)
````

### Exemplary library READMEs

- [fatiando/pooch](https://github.com/fatiando/pooch) — Strong hook, projects-using section, welcoming contribution message
- [gruns/furl](https://github.com/gruns/furl) — Centered logo, well-organized API examples
- [nschloe/meshio](https://github.com/nschloe/meshio) — Format list as blockquote, performance comparisons with charts
- [giampaolo/psutil](https://github.com/giampaolo/psutil) — Category-organized examples with output, notable adopters
- [ahupp/python-magic](https://github.com/ahupp/python-magic) — Concise, platform-specific install, troubleshooting section

## Application

For desktop apps, mobile apps, web apps, or any software end users download and run directly.

**Audience:** End users who want to use the software.
**Goal:** Show them what it looks like, get them to download it.

### Section order

1. **Name + Icon/Logo** — App name with icon. Download badge or button.
2. **Description** — One sentence on what the app does.
3. **Screenshot(s)** — At least one screenshot of the main UI. **This is the most important element.**
4. **Download / Install** — Primary download method (Homebrew, direct download, app store). Multiple methods are fine.
5. **Features** — Detailed feature list, optionally with screenshots per feature.
6. **How to Use** — User-facing step-by-step instructions (not developer-facing).
7. **Requirements / Compatibility** — OS versions, hardware requirements, supported platforms.
8. **Building from Source** — For developers. Near the bottom. Prerequisites and build steps.
9. **Contributing** — Link to CONTRIBUTING.md.
10. **Credits / License**

### Template

````markdown
# My App

![App Icon](./icon.png)

> One sentence describing what the app does for the user.

[![Download](badge)](download-url) [![License](badge)](license-url)

![Screenshot of main interface](./docs/screenshot.png)

## Download

### Homebrew

```bash
brew install --cask my-app
```

### Direct Download

Download the latest release from [Releases](link).

## Features

- **Feature A** — description
- **Feature B** — description
- **Feature C** — description

## Usage

1. Open the app
2. Do the thing
3. See the result

## Requirements

- macOS 13+ / Windows 10+ / Ubuntu 22.04+

## Building from Source

See [CONTRIBUTING.md](CONTRIBUTING.md) for build instructions.

## License

[MIT](LICENSE)
````

### Exemplary application READMEs

- [MonitorControl/MonitorControl](https://github.com/MonitorControl/MonitorControl) — App icon, screenshots, compatibility table, step-by-step usage

## CLI Tool

For command-line programs users invoke from a terminal.

**Audience:** Developers and power users comfortable with the terminal.
**Goal:** Show what the commands look like and what they produce.

### Section order

1. **Name + Badges**
2. **Description** — One sentence on the tool's purpose.
3. **Installation** — One-liner (package manager). Put this early — CLI users want to try it fast.
4. **Quick Start** — The single most common use case, one command.
5. **Usage / Commands** — Command examples with terminal output. Show realistic sessions.
6. **Configuration** — Config file format, environment variables (if applicable).
7. **Contributing**
8. **License**

### Template

````markdown
# my-tool

[![CI](badge)](ci-url) [![Crates.io](badge)](crates-url)

> One sentence: what it does.

## Installation

```bash
cargo install my-tool
```

## Quick Start

```bash
my-tool process input.csv
```

## Usage

### Convert files

```bash
$ my-tool convert data.json --format csv
Converting data.json → data.csv... done (42 rows)
```

### Inspect metadata

```bash
$ my-tool info report.pdf
Type:    PDF 1.7
Pages:   12
Size:    2.4 MB
Created: 2024-01-15
```

## Configuration

Create `~/.my-tool.toml`:

```toml
[defaults]
format = "csv"
output_dir = "~/exports"
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
````

### Exemplary CLI READMEs

- [mher/flower](https://github.com/mher/flower) — Feature list, usage commands, API examples
- [nschloe/meshio](https://github.com/nschloe/meshio) — CLI commands alongside library usage

## Framework / Platform

For frameworks, SDKs, and platforms that developers build on top of.

**Audience:** Developers evaluating whether to adopt this for their project.
**Goal:** Show the breadth of what's possible and get them started fast.

### Section order

1. **Name + Logo + Badges**
2. **What it is** — 1-2 sentences on the framework's purpose and scope.
3. **Key Links** — Navigation row: Documentation | Getting Started | Examples | Community
4. **Quick Start** — Minimal working example showing the framework in action.
5. **What You Can Build** — Showcase 2-3 diverse examples or use cases.
6. **Installation**
7. **Documentation** — Link to full docs.
8. **Community / Support** — Forums, Discord, mailing list.
9. **Projects Using This** — Social proof, notable adopters.
10. **Contributing**
11. **Citation** (for academic projects)
12. **License**

### Template

````markdown
# My Framework

![Logo](./logo.png)

> One sentence: what this framework enables.

[Documentation](link) | [Getting Started](link) | [Examples](link) | [Community](link)

[![CI](badge)](ci-url) [![npm](badge)](npm-url) [![License](badge)](license-url)

## Quick Start

```bash
npm install my-framework
```

```javascript
import { App } from 'my-framework'

const app = new App()
app.render('#root')
```

## What You Can Build

- **Interactive dashboards** — [example link]
- **Data visualizations** — [example link]
- **Real-time editors** — [example link]

## Documentation

Full documentation at [docs.myframework.dev](link).

## Community

- [Discord](link) — Chat with the community
- [Discussions](link) — Ask questions, share ideas

## Projects Using My Framework

- [Notable Project A](link)
- [Notable Project B](link)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[Apache-2.0](LICENSE)
````

### Exemplary framework READMEs

- [Kitware/ITK](https://github.com/Kitware/ITK) — Multi-platform CI table, extensive links, citation info
- [curvenote/components](https://github.com/curvenote/components) — Animated GIF demo, HTML code example, component list
- [marcomusy/vedo](https://github.com/marcomusy/vedo) — Expandable `<details>` sections for features and references

## Starter / Template

For project scaffolds, boilerplates, and templates users clone to start their own project.

**Audience:** Developers who want to start building immediately.
**Goal:** Get them from clone to running in under 60 seconds.

### Section order

1. **Name**
2. **Screenshot or Demo** — Show the running result. "This is what you get out of the box."
3. **What's Included** — Tech stack, features already configured.
4. **Quick Start** — Clone → install → run. Must be dead simple.
5. **Project Structure** — Directory overview explaining what's where.
6. **Customization** — How to adapt it (rename, configure, extend).
7. **Tech Choices** — Why these tools/libraries were chosen (optional, brief).
8. **Contributing**
9. **License**

### Template

````markdown
# my-starter

> A [framework] starter with [key features] already configured.

![Screenshot of running app](./docs/screenshot.png)

## What's Included

- [Framework] with [feature A]
- [Tool B] for testing
- [Tool C] for linting
- CI/CD via GitHub Actions
- Docker support

## Quick Start

```bash
git clone https://github.com/user/my-starter.git my-project
cd my-project
npm install
npm run dev
```

Open http://localhost:3000.

## Project Structure

```
my-project/
├── src/
│   ├── components/    # UI components
│   ├── pages/         # Route pages
│   └── utils/         # Shared utilities
├── tests/             # Test files
├── public/            # Static assets
└── package.json
```

## Customization

1. Update `package.json` with your project name
2. Replace `src/components/Logo.tsx` with your logo
3. Edit `src/config.ts` for app-specific settings

## License

[MIT](LICENSE)
````

## Collection / Curated List

For repositories where the README IS the content — awesome lists, resource collections, cookbooks.

**Audience:** Learners and practitioners browsing for resources.
**Goal:** Enable fast navigation to the right resource.

### Section order

1. **Name + Description**
2. **Table of Contents** — Essential. This is the primary navigation.
3. **Sections** — Well-defined categories. Each item gets a one-line description.
4. **Contributing** — How to suggest additions. Quality criteria for inclusion.
5. **Code of Conduct**

### Template

````markdown
# Awesome Topic

> A curated list of resources for [topic].

## Contents

- [Section A](#section-a)
- [Section B](#section-b)
- [Section C](#section-c)

## Section A

- [Resource Name](link) — One-line description of what it is and why it's notable.
- [Resource Name](link) — One-line description.

## Section B

- [Resource Name](link) — One-line description.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on suggesting additions.

Items must meet these criteria:
- [Quality criterion 1]
- [Quality criterion 2]
````

For cookbook-style collections with code, include reproducibility instructions (dependencies, how to run scripts) and use per-directory READMEs for organization.

## Mature Project (has docs site)

When a project has dedicated documentation, the README becomes an elevator pitch and routing layer. Don't duplicate the docs.

**Audience:** Mixed — first-time visitors and returning users.
**Goal:** Orient the reader and route them to the right resource.

### Section order

1. **Name + Logo + Badges**
2. **Tagline** — One sentence, maximum impact.
3. **Key Links** — Prominent navigation to docs, install guide, community.
4. **Brief Highlights** — 3-5 bullets of what makes this compelling.
5. **Quick Install + Example** — Minimal. Just enough to taste it.
6. **Contributing**
7. **License**

### Template

````markdown
# My Project

![Logo](./logo.png)

> One powerful sentence about what this does.

[Documentation](link) | [Install Guide](link) | [API Reference](link) | [Community](link)

[![CI](badge)](ci-url) [![Version](badge)](version-url) [![License](badge)](license-url)

## Highlights

- Highlight 1
- Highlight 2
- Highlight 3

## Quick Start

```bash
pip install my-project
```

```python
import my_project
my_project.demo()
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE)
````

Keep it under 60 lines. The docs site is the real documentation — the README just gets people there.
