# Getting Started

This guide walks you through installing govctl and creating your first governed artifact.

## Requirements

- **Rust 1.94+** (uses edition 2024)

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

## Validate Everything

```bash
govctl check
```

This validates all governance artifacts against JSON schemas, phase rules, cross-references, and source code annotations.

## Render to Markdown

```bash
govctl render
```

Generates human-readable markdown in `docs/`.

## Interactive TUI

govctl includes an optional interactive terminal dashboard:

```bash
govctl tui
```

### TUI Keyboard Shortcuts

| Key        | Action                                   |
| ---------- | ---------------------------------------- |
| `1` / `r`  | View RFCs                                |
| `2` / `a`  | View ADRs                                |
| `3` / `w`  | View Work Items                          |
| `j` / `↓`  | Navigate down                            |
| `k` / `↑`  | Navigate up                              |
| `Enter`    | Open detail view                         |
| `Esc`      | Go back                                  |
| `/`        | Filter list (type query, Enter to apply) |
| `g`        | Jump to top                              |
| `G`        | Jump to bottom                           |
| `Ctrl+d`   | Scroll half page down (detail view)      |
| `Ctrl+u`   | Scroll half page up (detail view)        |
| `PageDown` | Scroll page down (detail view)           |
| `PageUp`   | Scroll page up (detail view)             |
| `?`        | Toggle help overlay                      |
| `q`        | Quit                                     |

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
| **Effect** | Rewrites TOML/JSON files in `gov/`                  | Discovers decisions, backfills ADRs, annotates source |
| **Risk**   | Low — transactional, reversible                     | Medium — requires human review of generated ADRs      |

Run `govctl migrate` when govctl tells you a migration is needed (error `E0505`). Use the `/migrate` skill when bringing a legacy project under governance for the first time.

## Canonical Edit Surface

All artifact fields are accessible through a unified path-based edit interface:

```bash
# Set a scalar value
govctl rfc edit RFC-0010 version --set 1.2.0

# Add to an array
govctl adr edit ADR-0003 refs --add RFC-0010

# Remove by index
govctl work edit WI-2026-01-17-001 acceptance_criteria --at 0 --remove

# Tick checklist items
govctl adr edit ADR-0003 alternatives --tick accepted --at 0
govctl work edit WI-2026-01-17-001 acceptance_criteria --tick done --at 0
```

Nested object fields use dot-delimited paths:

```bash
govctl adr edit ADR-0003 content.decision --set "We will use Redis"
govctl adr edit ADR-0003 "content.alternatives[0].pros" --add "Low latency"
govctl work edit WI-2026-01-17-001 "journal[0].scope" --set backend
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
