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

## Adopting govctl in an Existing Project

`govctl init` is safe to run in existing repositories — it only creates the `gov/` directory structure alongside existing files.

For AI-assisted migration, use the `/migrate` skill to systematically discover undocumented decisions, backfill ADRs, and annotate source code with `[[...]]` references.

## Next Steps

- [Working with RFCs](./rfcs.md) — Full RFC lifecycle
- [Working with ADRs](./adrs.md) — Decision records
- [Working with Work Items](./work-items.md) — Task tracking
- [Validation & Rendering](./validation.md) — Quality gates and guards
