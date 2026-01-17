# govctl

> **govctl is an opinionated governance CLI for RFC-driven AI software development.**

## The Problem

AI-assisted coding is powerful but undisciplined:

- **Phase skipping**: Jumping from idea to implementation without specification
- **Documentation drift**: Specs and code diverge silently
- **No enforceable governance**: "Best practices" become optional suggestions

The result: faster typing, slower thinking, unmaintainable systems.

## What govctl Is

govctl enforces **phase discipline** on software development:

1. **RFC is the source of truth** — No implementation without specification
2. **Phases are enforced** — Each phase has explicit gates and invariants
3. **Governance is executable** — Rules are checked, not suggested

### Phase Discipline Workflow

```
┌─────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  SPEC   │ ──► │   IMPL   │ ──► │   TEST   │ ──► │  STABLE  │
│  phase  │     │  phase   │     │  phase   │     │  phase   │
└─────────┘     └──────────┘     └──────────┘     └──────────┘
     │                │                │                │
     ▼                ▼                ▼                ▼
  RFC must         Code must       Tests must       Bug fixes
  be normative     match spec      pass gates       only
```

## First Principles

These are non-negotiable:

1. **Specification precedes implementation**
2. **Phases cannot be skipped**
3. **Breaking changes require explicit RFC amendments**
4. **govctl itself follows govctl governance**

## Quick Start

```bash
# Install
cargo install govctl

# Initialize a new project
govctl init

# Create an RFC
govctl new rfc "Feature Title"

# Validate all governed documents
govctl check

# List RFCs
govctl list rfc

# Show summary
govctl stat
```

## Data Model

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
