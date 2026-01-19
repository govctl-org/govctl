# Working with ADRs

ADRs (Architectural Decision Records) document significant design choices. They explain _why_ things are built a certain way.

## Creating ADRs

```bash
govctl adr new "Use Redis for caching"
```

This creates a TOML file in `gov/adr/` with the decision context.

## ADR Structure

ADRs contain:

- **Context** — The situation requiring a decision
- **Decision** — What was decided
- **Consequences** — Expected outcomes (positive and negative)
- **Status** — `proposed`, `accepted`, `deprecated`, or `superseded`

## Editing ADRs

ADRs are TOML files — edit them directly in your editor or use govctl to get/set fields:

```bash
# Get specific field
govctl adr get ADR-0003 status

# Set field value
govctl adr set ADR-0003 status accepted
```

## Status Lifecycle

```
proposed → accepted → deprecated
                   ↘ superseded
```

### Accept a Decision

When consensus is reached:

```bash
govctl adr accept ADR-0003
```

### Deprecate

When a decision is no longer relevant:

```bash
govctl adr deprecate ADR-0003
```

### Supersede

When a new decision replaces an old one:

```bash
govctl adr supersede ADR-0001 --by ADR-0005
```

This marks ADR-0001 as superseded and records ADR-0005 as its replacement.

## Listing ADRs

```bash
govctl adr list
govctl adr list accepted    # Filter by status
```

## Why TOML?

ADRs use TOML (not JSON or YAML) because:

- **Comments allowed** — Humans can annotate inline
- **Multi-line strings** — Clean `"""` blocks for prose
- **No YAML ambiguity** — `NO` stays `NO`, not `false`
- **Round-trip stable** — Deterministic serialization
