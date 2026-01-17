# Working with ADRs

ADRs (Architectural Decision Records) document significant design choices. They explain _why_ things are built a certain way.

## Creating ADRs

```bash
govctl new adr "Use Redis for caching"
```

This creates a TOML file in `gov/adr/` with the decision context.

## ADR Structure

ADRs contain:

- **Context** — The situation requiring a decision
- **Decision** — What was decided
- **Consequences** — Expected outcomes (positive and negative)
- **Status** — `proposed`, `accepted`, `deprecated`, or `superseded`

## Editing ADRs

ADRs are TOML files — edit them directly or use govctl:

```bash
govctl edit ADR-0003
```

## Status Lifecycle

```
proposed → accepted → deprecated
                   ↘ superseded
```

### Accept a Decision

When consensus is reached:

```bash
govctl accept ADR-0003
```

### Deprecate

When a decision is no longer relevant:

```bash
govctl deprecate ADR-0003
```

### Supersede

When a new decision replaces an old one:

```bash
govctl supersede ADR-0001 --by ADR-0005
```

This marks ADR-0001 as superseded and records ADR-0005 as its replacement.

## Listing ADRs

```bash
govctl list adr
govctl list adr --status accepted
```

## Why TOML?

ADRs use TOML (not JSON or YAML) because:

- **Comments allowed** — Humans can annotate inline
- **Multi-line strings** — Clean `"""` blocks for prose
- **No YAML ambiguity** — `NO` stays `NO`, not `false`
- **Round-trip stable** — Deterministic serialization
