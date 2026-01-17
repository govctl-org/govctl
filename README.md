# govctl

[![CI](https://github.com/govctl-org/govctl/actions/workflows/ci.yml/badge.svg)](https://github.com/govctl-org/govctl/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/govctl.svg)](https://crates.io/crates/govctl)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **govctl is an opinionated governance CLI for RFC-driven AI software development.**

---

## The Problem

AI-assisted coding is powerful but undisciplined:

- **Phase skipping**: Jumping from idea to implementation without specification
- **Documentation drift**: Specs and code diverge silently
- **No enforceable governance**: "Best practices" become optional suggestions

The result: faster typing, slower thinking, unmaintainable systems.

---

## What govctl Is

govctl enforces **phase discipline** on software development:

1. **RFC is the source of truth** — No implementation without specification
2. **Phases are enforced** — Each phase has explicit gates and invariants
3. **Governance is executable** — Rules are checked, not suggested

govctl governs itself by its own rules. This repository is the first proof.

---

## What govctl Is NOT

- ❌ **Not a code generator** — govctl doesn't write code; it ensures code follows specs
- ❌ **Not a documentation editor** — govctl enforces structure, not style
- ❌ **Not about "faster coding"** — govctl is about _correct_ coding
- ❌ **Not a framework** — govctl is a constraint system

---

## First Principles

These are non-negotiable:

1. **Specification precedes implementation**
2. **Phases cannot be skipped**
3. **Breaking changes require explicit RFC amendments**
4. **govctl itself follows govctl governance**

---

## Current Status

govctl is in active development. The following RFCs define its governance:

| RFC                              | Title                       | Status    | Phase  |
| -------------------------------- | --------------------------- | --------- | ------ |
| [RFC-0000](docs/rfc/RFC-0000.md) | govctl Governance Framework | Normative | Stable |

---

## The `govctl` CLI

govctl is a governance CLI for managing RFCs, ADRs, and Work Items.

### Installation

```bash
cargo build --release
# Binary at ./target/release/govctl
```

### Quick Start

```bash
# Initialize a new project
govctl init

# Create an RFC (auto-assigns next ID)
govctl new rfc "Feature Title"

# Or specify ID manually
govctl new rfc "Feature Title" --id RFC-0010

# Create a clause within an RFC
govctl new clause RFC-0010:C-SCOPE "Scope" -s "Specification" -k normative

# Edit clause text
govctl edit RFC-0010:C-SCOPE --stdin <<'EOF'
The feature MUST...
EOF

# Render RFCs to markdown (default, published to repo)
govctl render

# Render specific RFC
govctl render --rfc-id RFC-0010

# Render ADRs/Work Items locally (not committed, .gitignore'd)
govctl render adr
govctl render work
govctl render all   # Everything

# Validate all governed documents
govctl check

# List RFCs
govctl list rfc

# List clauses
govctl list clause

# Show summary
govctl stat
```

### Data Model

All SSOT (Single Source of Truth) files live under `gov/`, rendered docs go to `docs/`:

```
gov/                          # SSOT (managed by govctl)
├── config.toml               # govctl configuration
├── rfc/                      # RFC-NNNN/rfc.json + clauses/
├── adr/                      # ADR-NNNN-*.toml
├── work/                     # WI-YYYY-MM-DD-NNN-*.toml
├── schema/                   # Schema definitions
└── templates/                # New artifact templates

docs/                         # Rendered (human-readable)
├── rfc/                      # RFC-NNNN.md
├── adr/                      # ADR-NNNN.md
└── work/                     # WI-*.md
```

**Why TOML for ADRs/Work Items?**

- Comments allowed (humans can annotate)
- Multi-line strings are clean (`"""` blocks)
- No YAML ambiguity (`NO` → false problem)
- Round-trip stable (deterministic serialization)

### Lifecycle Commands

```bash
# RFC status: draft → normative → deprecated
govctl finalize RFC-0010 normative

# RFC phase: spec → impl → test → stable
govctl advance RFC-0010 impl

# Version bump with changelog
govctl bump RFC-0010 --minor -m "Add new clause"

# ADR lifecycle
govctl accept ADR-0003
govctl deprecate ADR-0002
govctl supersede ADR-0001 --by ADR-0005

# Work item lifecycle
govctl new work "Task title"              # Creates in queue
govctl new work --active "Urgent task"    # Creates and activates
govctl mv WI-2026-01-17-001 active        # By ID
govctl mv migrate-docs.toml done          # Or by filename

# Structured checklists
govctl add WI-2026-01-17-001 acceptance_criteria "Criterion text"
govctl tick WI-2026-01-17-001 acceptance_criteria "Criterion" done

# Array field matching (per ADR-0007)
govctl remove <ID> <field> "pattern"           # Case-insensitive substring (default)
govctl remove <ID> <field> "exact" --exact     # Exact match
govctl remove <ID> <field> --at 0              # By index (0-based)
govctl remove <ID> <field> --at -1             # Negative index (from end)
govctl remove <ID> <field> "RFC-.*" --regex    # Regex pattern
govctl remove <ID> <field> "pattern" --all     # Remove all matches

# tick uses same matching (no --all)
govctl tick <ID> <field> "substr" done         # Substring match
govctl tick <ID> <field> done --at 2           # By index
```

---

## Deferred Work (Explicit Non-Goals for Now)

The following are **not being worked on** until core governance is stable:

- Block storage / CRDT
- IDE plugins
- MCP integration
- Language-specific toolchains

This is not conservatism. This is focus.

---

## Contributing

Before contributing code, understand:

1. Read all three constitutional RFCs
2. Any code contribution requires a normative RFC in the appropriate phase
3. PhaseOS is not a democracy — it is a discipline

If you disagree with this philosophy, this project is not for you.

---

## License

MIT

---

> _"The first commit is not the start of coding. It is the start of obedience."_
