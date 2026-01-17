# PhaseOS

> **PhaseOS is an opinionated governance engine for RFC-driven AI software development.**

---

## The Problem

AI-assisted coding is powerful but undisciplined:

- **Phase skipping**: Jumping from idea to implementation without specification
- **Documentation drift**: Specs and code diverge silently
- **No enforceable governance**: "Best practices" become optional suggestions

The result: faster typing, slower thinking, unmaintainable systems.

---

## What PhaseOS Is

PhaseOS enforces **phase discipline** on software development:

1. **RFC is the source of truth** — No implementation without specification
2. **Phases are enforced** — Each phase has explicit gates and invariants
3. **Governance is executable** — Rules are checked, not suggested

PhaseOS governs itself by its own rules. This repository is the first proof.

---

## What PhaseOS Is NOT

- ❌ **Not a code generator** — PhaseOS doesn't write code; it ensures code follows specs
- ❌ **Not a documentation editor** — PhaseOS enforces structure, not style
- ❌ **Not about "faster coding"** — PhaseOS is about _correct_ coding
- ❌ **Not a framework** — PhaseOS is a constraint system

---

## First Principles

These are non-negotiable:

1. **Specification precedes implementation**
2. **Phases cannot be skipped**
3. **Breaking changes require explicit RFC amendments**
4. **PhaseOS itself follows PhaseOS governance**

---

## Current Status

PhaseOS is in its constitutional phase. The following RFCs define its governance:

| RFC                                | Title            | Status |
| ---------------------------------- | ---------------- | ------ |
| [RFC-0000](docs/rfc/RFC-0000.md)   | What an RFC Is   | Draft  |
| [RFC-0001](docs/rfc/RFC-0001.md)   | PhaseOS Vision   | Draft  |
| [RFC-0002](docs/rfc/RFC-0002.md)   | Phase Discipline | Draft  |

---

## The `phaseos` CLI

PhaseOS includes a governance CLI for managing RFCs, ADRs, and Work Items.

### Installation

```bash
cargo build --release
# Binary at ./target/release/phaseos
```

### Quick Start

```bash
# Initialize a new project
phaseos init

# Create an RFC
phaseos new rfc RFC-0010 "Feature Title"

# Create a clause within an RFC
phaseos new clause RFC-0010:C-SCOPE "Scope" -s "Specification" -k normative

# Edit clause text
phaseos edit RFC-0010:C-SCOPE --text-stdin <<'EOF'
The feature MUST...
EOF

# Render RFC markdown from JSON
phaseos render RFC-0010

# Validate all governed documents
phaseos check

# List RFCs
phaseos list rfc

# List clauses
phaseos list clause

# Show summary
phaseos stat
```

### Data Model

- **RFCs**: JSON source of truth (`spec/rfcs/RFC-NNNN/rfc.json`) → rendered Markdown (`docs/rfc/RFC-NNNN.md`)
- **Clauses**: Individual requirement units within RFCs (`spec/rfcs/RFC-NNNN/clauses/C-NAME.json`)
- **ADRs**: Architecture Decision Records (`docs/adr/ADR-NNNN-*.md`) with YAML frontmatter
- **Work Items**: Task tracking (`worklogs/items/YYYY-MM-DD-*.md`) with YAML frontmatter

### Lifecycle Commands

```bash
# RFC status: draft → normative → deprecated
phaseos finalize RFC-0010 normative

# RFC phase: spec → impl → test → stable
phaseos advance RFC-0010 impl

# Version bump with changelog
phaseos bump RFC-0010 --minor -m "Add new clause"

# ADR lifecycle
phaseos accept ADR-0003
phaseos deprecate ADR-0002
phaseos supersede ADR-0001 --by ADR-0005

# Work item status
phaseos mv work-item.md active
phaseos mv work-item.md done
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
