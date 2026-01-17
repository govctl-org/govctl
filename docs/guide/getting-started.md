# Getting Started

This guide walks you through installing govctl and creating your first governed artifact.

## Requirements

- **Rust 1.85+** (uses edition 2024)

## Installation

```bash
# From crates.io
cargo install govctl

# With TUI dashboard feature
cargo install govctl --features tui

# Or build from source
git clone https://github.com/govctl-org/govctl
cd govctl
cargo build --release
# Binary at ./target/release/govctl

# Build with TUI
cargo build --release --features tui
```

### Optional Features

| Feature | Description                                   | Dependencies       |
| ------- | --------------------------------------------- | ------------------ |
| `tui`   | Interactive terminal dashboard (`govctl tui`) | ratatui, crossterm |

## Initialize a Project

```bash
govctl init
```

This creates the governance directory structure:

```
gov/
├── config.toml       # Configuration
├── rfc/              # RFC sources
├── adr/              # ADR sources
├── work/             # Work item sources
├── schema/           # JSON schemas
└── templates/        # New artifact templates
```

## Create Your First RFC

```bash
govctl new rfc "Feature Title"
```

This creates `gov/rfc/RFC-0000/rfc.json` with the RFC metadata.

## Add a Clause

RFCs are composed of clauses — atomic units of specification:

```bash
govctl new clause RFC-0000:C-SCOPE "Scope" -s "Specification" -k normative
```

## Edit Clause Content

```bash
govctl edit RFC-0000:C-SCOPE --stdin <<'EOF'
The feature MUST do X.
The feature SHOULD do Y.
EOF
```

## Validate Everything

```bash
govctl check
```

This validates all governance artifacts against the schema and phase rules.

## Render to Markdown

```bash
govctl render
```

Generates human-readable markdown in `docs/rfc/RFC-0000.md`.

## Next Steps

- [Working with RFCs](./rfcs.md) — Full RFC lifecycle
- [Working with ADRs](./adrs.md) — Decision records
- [Working with Work Items](./work-items.md) — Task tracking
- [Validation & Rendering](./validation.md) — Quality gates
