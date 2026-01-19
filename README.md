<p align="center">
  <img src="assets/logo.svg" alt="govctl logo" width="120" height="120">
</p>

<h1 align="center">govctl</h1>

<p align="center">
  <a href="https://github.com/govctl-org/govctl/actions/workflows/ci.yml"><img src="https://github.com/govctl-org/govctl/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/govctl"><img src="https://img.shields.io/crates/v/govctl.svg" alt="Crates.io"></a>
  <a href="https://coderabbit.ai"><img src="https://img.shields.io/coderabbit/prs/github/govctl-org/govctl?label=CodeRabbit" alt="CodeRabbit"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://github.com/govctl-org/govctl"><img src="https://img.shields.io/badge/governed%20by-govctl-6366F1" alt="governed by govctl"></a>
</p>

<p align="center">
  <strong>Opinionated governance CLI for RFC-driven AI-assisted software development.</strong>
</p>

---

## The Problem

AI-assisted coding is powerful but undisciplined:

- **Phase skipping**: Jumping from idea to implementation without specification
- **Documentation drift**: Specs and code diverge silently
- **No enforceable governance**: "Best practices" become optional suggestions

The result: faster typing, slower thinking, unmaintainable systems.

### Without govctl

```
Day 1:  "Let's add caching!"
Day 2:  AI generates 500 lines of Redis integration
Day 7:  "Wait, did we agree on Redis or Memcached?"
Day 14: Half the team implements one, half the other
Day 30: Two incompatible caching layers, no spec, nobody knows why
```

### With govctl

```
Day 1:  govctl rfc new "Caching Strategy"
Day 2:  RFC-0015 defines: Redis, TTL policy, invalidation rules
Day 3:  govctl rfc advance RFC-0015 impl
Day 7:  Implementation complete, traceable to spec
Day 10: govctl rfc advance RFC-0015 test
Day 14: Tests pass, govctl rfc advance RFC-0015 stable
```

---

## What govctl Is

govctl enforces **phase discipline** on software development:

1. **RFC is the source of truth** — No implementation without specification
2. **Phases are enforced** — Each phase has explicit gates and invariants
3. **Governance is executable** — Rules are checked, not suggested
4. **Work is traceable** — Tasks link back to the specs that authorized them

govctl manages three artifact types:

- **RFCs** — Specifications that must exist before implementation
- **ADRs** — Architectural decisions with explicit consequences
- **Work Items** — Tracked tasks tied to governance artifacts

```
┌─────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  SPEC   │ ──► │   IMPL   │ ──► │   TEST   │ ──► │  STABLE  │
└─────────┘     └──────────┘     └──────────┘     └──────────┘
     │                │                │                │
     ▼                ▼                ▼                ▼
  RFC must         Code must       Tests must       Bug fixes
  be normative     match spec      pass gates       only
```

govctl governs itself by its own rules. This repository is the first proof.

---

## Who This Is For

✅ **Teams frustrated by AI "code now, think later" patterns**
✅ **Projects where specifications drift from implementations**
✅ **Organizations needing audit trails for AI-generated code**
✅ **Developers who believe discipline enables velocity**

❌ Not for "move fast and break things" workflows
❌ Not for projects without review processes

---

## Quick Start

```bash
# Install
cargo install govctl

# Or with TUI dashboard
cargo install govctl --features tui

# Initialize project
govctl init

# Create your first RFC
govctl rfc new "Feature Title"

# Validate
govctl check
```

Optionally, show the project is governed by govctl:

```markdown
[![governed by govctl](https://img.shields.io/badge/governed%20by-govctl-6366F1)](https://github.com/govctl-org/govctl)
```

For complete documentation, see the [User Guide](https://govctl-org.github.io/govctl/).

---

## What govctl Is NOT

- **Not a code generator** — govctl doesn't write code; it ensures code follows specs
- **Not a documentation editor** — govctl enforces structure, not style
- **Not about "faster coding"** — govctl is about _correct_ coding
- **Not a framework** — govctl is a constraint system
- **Not a general issue tracker** — Work items exist to trace work back to specs, not to replace Jira

---

## Why No MCP?

govctl doesn't need a dedicated [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) integration because **the CLI itself is the tool**.

Modern AI coding agents (Claude, Cursor, Codex, etc.) can already invoke shell commands. Every govctl operation is a single CLI call. MCP would add complexity without adding capability.

The CLI is the universal interface. Every shell-capable agent already speaks it.

**For Claude/Cursor users:** `govctl init` installs a `/gov <task>` command — a complete governed workflow in one invocation.

---

## Contributing

govctl has an opinionated workflow. Before contributing:

1. Read the [governance RFC](docs/rfc/RFC-0000.md) to understand the model
2. All features require an RFC before implementation
3. Phase gates are enforced — this is the point, not bureaucracy

**This workflow isn't for everyone, and that's okay.** If you thrive in structured, spec-driven development, we'd welcome your contributions.

---

## License

MIT

---

> _"Discipline is not the opposite of creativity. It is the foundation."_
