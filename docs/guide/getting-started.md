# Getting Started

This guide walks you through installing govctl and creating your first governed artifact.

## Requirements

- **Rust 1.96+** (per `Cargo.toml` `rust-version`)

## Installation

```bash
# From crates.io (includes TUI by default)
cargo install govctl

# Without TUI
cargo install govctl --no-default-features

# Or build from source
git clone https://github.com/govctl-org/govctl
cd govctl
cargo build --release
# Binary at ./target/release/govctl
```

### Features

| Feature | Default | Description                                   | Dependencies       |
| ------- | ------- | --------------------------------------------- | ------------------ |
| `tui`   | Yes     | Interactive terminal dashboard (`govctl tui`) | ratatui, crossterm |

## Shell Completion

Generate completion scripts for your shell:

```bash
# Bash
govctl completions bash > ~/.local/share/bash-completion/completions/govctl

# Zsh (add to your .zshrc or install to completion directory)
govctl completions zsh > ~/.zsh/completions/_govctl
# Then add to fpath: fpath=(~/.zsh/completions $fpath)

# Fish
govctl completions fish > ~/.config/fish/completions/govctl.fish

# PowerShell (add to your profile)
govctl completions powershell >> $PROFILE
```

Restart your shell or source the configuration to enable tab completion.

## Initialize a Project

```bash
govctl init
```

This creates the governance directory structure:

```
gov/
├── config.toml       # Configuration (project name, schema version, guards)
├── rfc/              # RFC sources (TOML)
├── adr/              # ADR sources (TOML)
├── work/             # Work item sources (TOML)
├── guard/            # Verification guards (TOML)
├── schema/           # JSON schemas for validation
└── releases.toml     # Release history
```

All governance artifacts use TOML with `#:schema` comment headers for IDE discoverability:

```toml
#:schema ../schema/adr.schema.json

[govctl]
id = "ADR-0001"
title = "My Decision"
status = "proposed"
...
```

## Create Your First RFC

```bash
govctl rfc new "Feature Title"
```

This creates `gov/rfc/RFC-0000/rfc.toml` with the RFC metadata.

## Add a Clause

RFCs are composed of clauses — atomic units of specification:

```bash
govctl clause new RFC-0000:C-SCOPE "Scope" -s "Specification" -k normative
```

## Edit Clause Content

```bash
govctl clause edit RFC-0000:C-SCOPE text --stdin <<'EOF'
The feature MUST do X.
The feature SHOULD do Y.
EOF
```

## View Artifacts

```bash
# Styled markdown to stdout
govctl rfc show RFC-0000
govctl adr show ADR-0001
govctl work show WI-2026-01-17-001
govctl clause show RFC-0000:C-SCOPE

# Interactive TUI dashboard
govctl tui
```

## Search Artifacts

Search looks across RFCs, clauses, ADRs, work items, and verification guards:

```bash
govctl search cache
govctl search "schema migration" --type rfc --type adr
govctl search RFC-0002 --output json
govctl search cli --tag validation -n 5
govctl search loop --reindex
```

Search keeps any persisted index under `.govctl/` as disposable local state.
Artifacts under `gov/` remain authoritative; `--reindex` forces a rebuild before
returning results.

## Validate Everything

```bash
govctl check
```

This validates all governance artifacts against JSON schemas, phase rules, cross-references, and source code annotations.

## Recommended Workflow

Before using govctl on non-trivial work, read the
[Recommended Workflows](./recommended-workflows.md) guide. It explains when to
use RFCs, ADRs, Work Items, execution loops, reviewer agents, and verification
guards together.

## Render to Markdown

```bash
govctl render
```

Generates human-readable markdown in `docs/`.

## Current Views and History

Human-readable `show` output is optimized for the current governance context.
Deprecated RFC bodies, superseded ADR bodies, and deprecated or superseded
Clause text are hidden by default while their identity, lifecycle state, and
replacement metadata remain visible.

```bash
# Current context for humans and agents
govctl rfc show RFC-0002

# Complete historical body content
govctl rfc show RFC-0002 --history

# Complete structured resources for automation
govctl rfc show RFC-0002 --output json
govctl rfc show RFC-0002 --output yaml
govctl rfc show RFC-0002 --output toml
```

`--history` applies only to human-readable `table` and `plain` output. Structured
output is always complete. `render` also always preserves complete historical
content in generated Markdown; it is not affected by the default `show`
projection. Work Items and Guards have no obsolete-body lifecycle state, so
their current and archival human-readable views are equivalent.

## Interactive TUI

govctl includes an optional read-only terminal cockpit:

```bash
govctl tui
```

The cockpit is for human inspection: overview, artifact lists, search, loop
state and dependency DAGs, guards, releases, tags, and check diagnostics.
State-changing operations remain CLI commands.

### TUI Keyboard Shortcuts

| Key                   | Action                                  |
| --------------------- | --------------------------------------- |
| `1` / `r`             | RFC list                                |
| `2` / `c`             | Clause list                             |
| `3` / `a`             | ADR list                                |
| `4` / `w`             | Work item list                          |
| `5` / `g`             | Guard list                              |
| `6` / `s`             | Search view                             |
| `7` / `l`             | Loop list and loop DAG inspector        |
| `8` / `d`             | Diagnostics view                        |
| `9`                   | Release list                            |
| `t`                   | Tag list                                |
| `j` / `↓`             | Navigate down                           |
| `k` / `↑`             | Navigate up                             |
| `Enter`               | Open selected detail or search result   |
| `Esc`                 | Go back or leave input mode             |
| `/`                   | Filter lists; edit query in search view |
| `e`                   | Edit query in search view               |
| `n` / `p`             | Next/previous filtered match            |
| `g` / `G`             | Jump to top/bottom in lists             |
| `Ctrl+d` / `u`        | Scroll half page in detail views        |
| `PageDown` / `PageUp` | Scroll page in detail views             |
| `?`                   | Toggle help overlay                     |
| `q`                   | Quit                                    |

## Cutting a Release

When a set of work items is complete and ready for release:

```bash
# Collect all unreleased done work items into a version
govctl release 0.2.0

# Specify a custom date
govctl release 0.2.0 --date 2026-04-15
```

This records the release in `gov/releases.toml` and makes those work items available for changelog generation.

## Adopting govctl in an Existing Project

`govctl init` is safe to run in existing repositories — it only creates the `gov/` directory structure alongside existing files.

For AI-assisted migration, use the `/migrate` skill to systematically discover undocumented decisions, backfill ADRs, and annotate source code with `[[...]]` references.

### `govctl migrate` vs the `/migrate` Skill

|            | `govctl migrate`                                    | `/migrate` skill                                      |
| ---------- | --------------------------------------------------- | ----------------------------------------------------- |
| **What**   | Upgrade existing govctl artifacts to current format | Adopt govctl in an existing project                   |
| **When**   | After updating govctl version                       | When starting governance in a brownfield repo         |
| **Effect** | Rewrites TOML files in `gov/` and syncs schemas     | Discovers decisions, backfills ADRs, annotates source |
| **Risk**   | Low — transactional, reversible                     | Medium — requires human review of generated ADRs      |

Run `govctl migrate` when govctl reports an outdated schema version. If a repository still contains legacy RFC or clause JSON storage, migrate it with govctl <0.9 before upgrading. Use the `/migrate` skill when bringing a legacy project under governance for the first time.

## Canonical Edit Surface

All artifact fields are accessible through a unified path-based edit interface:

```bash
# Set a scalar value
govctl rfc edit RFC-0010 version --set 1.2.0

# Add to an array
govctl adr edit ADR-0003 refs --add RFC-0010

# Remove by index
govctl work edit WI-2026-01-17-001 content.acceptance_criteria[0] --remove

# Tick checklist items
govctl adr edit ADR-0003 content.alternatives[0] --tick accepted
govctl work edit WI-2026-01-17-001 content.acceptance_criteria[0] --tick done
```

Nested object fields use dot-delimited paths:

```bash
govctl adr edit ADR-0003 content.decision --set "We will use Redis"
govctl adr edit ADR-0003 "content.alternatives[0].pros" --add "Low latency"
govctl work edit WI-2026-01-17-001 "content.acceptance_criteria[0].category" --set fixed
```

## CLI Self-Description

govctl provides a machine-readable command catalog:

```bash
govctl describe
govctl describe --context   # Includes project context
govctl describe --output json
```

This is designed for agent discoverability — agents can inspect available commands and their semantics without hardcoded knowledge.

## Next Steps

- [Working with RFCs](./rfcs.md) — Full RFC lifecycle
- [Working with ADRs](./adrs.md) — Decision records
- [Working with Work Items](./work-items.md) — Task tracking
- [Validation & Rendering](./validation.md) — Quality gates, guards, tags, and more
