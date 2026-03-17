<p align="center">
  <img src="assets/logo.svg" alt="govctl logo" width="120" height="120">
</p>

<h1 align="center">govctl</h1>

<p align="center">
  <a href="https://github.com/govctl-org/govctl/actions/workflows/ci.yml"><img src="https://github.com/govctl-org/govctl/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://codecov.io/gh/govctl-org/govctl"><img src="https://codecov.io/gh/govctl-org/govctl/graph/badge.svg" alt="codecov"></a>
  <a href="https://crates.io/crates/govctl"><img src="https://img.shields.io/crates/v/govctl.svg" alt="Crates.io"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT"></a>
  <a href="https://discord.gg/buBB9G8Z6n"><img src="https://img.shields.io/discord/1466789912211620066?logo=discord&label=Discord" alt="Discord"></a>
  <a href="https://github.com/govctl-org/govctl"><img src="https://img.shields.io/badge/governed%20by-govctl-6366F1" alt="governed by govctl"></a>
</p>

<p align="center">
  <strong>Governance-as-code for AI-assisted software development.</strong><br>
  <em>Enforce spec-first discipline. Every feature starts with an RFC, not a prompt.</em>
</p>

---

## The Problem

AI-assisted coding is powerful but undisciplined:

- **Phase skipping** — Jumping from idea to implementation without specification
- **Documentation drift** — Specs and code diverge silently
- **No enforceable governance** — "Best practices" become optional suggestions

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
Day 14: Tests pass, govctl rfc advance RFC-0015 stable
```

---

## Quick Start

### Claude Code Plugin

```
/plugin marketplace add govctl-org/govctl
/plugin install govctl@govctl
/govctl:init
```

The plugin provides workflow skills, reviewer agents, and enforcement hooks out of the box.

### CLI Only

```bash
cargo install govctl
govctl init
```

`govctl init` creates the governance structure and installs AI agent skills into `.claude/`.

For complete documentation, see the [User Guide](https://govctl-org.github.io/govctl/).

---

## AI Agent Integration

govctl is built for AI-native development. Install the [Claude Code plugin](#quick-start) or run `govctl init` to get workflow skills that any Claude Code / Cursor / Codex agent can invoke:

| Skill              | Purpose                                                                                 |
| ------------------ | --------------------------------------------------------------------------------------- |
| `/gov <task>`      | Complete governed workflow: work item, RFC/ADR, implement, test, done                   |
| `/migrate`         | Adopt govctl in an existing project: discover decisions, backfill ADRs, annotate source |
| `/discuss <topic>` | Design discussion: explore options, draft RFC or ADR                                    |
| `/commit`          | Smart commit: VCS detection, govctl checks, work item journal updates                   |
| `/quick <task>`    | Fast path for trivial changes (skip governance ceremony)                                |

The plugin also includes enforcement hooks: `govctl status` runs at session start for context, `govctl check` runs at session end as a gate.

Every govctl operation is a single CLI call. No MCP server needed -- the CLI is the universal interface. Every shell-capable agent already speaks it.

---

## What govctl Does

govctl enforces **phase discipline** on software development:

```
┌─────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  SPEC   │ ──► │   IMPL   │ ──► │   TEST   │ ──► │  STABLE  │
└─────────┘     └──────────┘     └──────────┘     └──────────┘
     │                │                │                │
     ▼                ▼                ▼                ▼
  RFC must         Code must       Tests must       Bug fixes
  be normative     match spec      pass gates       only
```

Three artifact types, one lifecycle:

- **RFCs** -- Specifications that must exist before implementation
- **ADRs** -- Architectural decisions with explicit trade-offs
- **Work Items** -- Tracked tasks tied to governance artifacts

govctl governs itself by its own rules. This repository is the first proof.

---

## Who This Is For

- **Teams frustrated by AI "code now, think later" patterns**
- **Existing projects** that need to retroactively establish governance (`/migrate`)
- **Organizations needing audit trails** for AI-generated code
- **Developers who believe discipline enables velocity**

Not for "move fast and break things" workflows. Not for projects without review processes.

---

## TUI Dashboard

```bash
govctl tui
```

The TUI is included by default. Keymap: `1`/`2`/`3` to switch lists, `j`/`k` to navigate, `Enter` to open, `/` to filter, `Ctrl+d`/`Ctrl+u` to page, `?` for help.

---

## Community

- [Discord](https://discord.gg/buBB9G8Z6n) -- Questions, discussions, feedback
- [GitHub Issues](https://github.com/govctl-org/govctl/issues) -- Bug reports and feature requests
- [GitHub Discussions](https://github.com/govctl-org/govctl/discussions) -- Design conversations

---

## Contributing

govctl has an opinionated workflow. Before contributing:

1. Read the [governance RFC](docs/rfc/RFC-0000.md) to understand the model
2. All features require an RFC before implementation
3. Phase gates are enforced -- this is the point, not bureaucracy

**This workflow isn't for everyone, and that's okay.** If you thrive in structured, spec-driven development, we'd welcome your contributions.

---

## Star History

<a href="https://star-history.com/#govctl-org/govctl&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=govctl-org/govctl&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=govctl-org/govctl&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=govctl-org/govctl&type=Date" />
 </picture>
</a>

---

## License

MIT

---

> _"Discipline is not the opposite of creativity. It is the foundation."_
