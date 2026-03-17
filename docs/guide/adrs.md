# Working with ADRs

ADRs (Architectural Decision Records) document significant design choices. They explain _why_ things are built a certain way.

## Creating ADRs

```bash
govctl adr new "Use Redis for caching"
```

This creates a TOML file in `gov/adr/` with the decision context.

## ADR Structure

ADRs are TOML files with `#:schema` headers:

```toml
#:schema ../schema/adr.schema.json

[govctl]
id = "ADR-0003"
title = "Use Redis for caching"
status = "proposed"
date = "2026-03-17"
refs = ["RFC-0001"]

[content]
context = "We need a caching layer for..."
decision = "We will use Redis because..."
consequences = "Positive: faster reads. Negative: operational complexity."

[[content.alternatives]]
text = "Memcached"
pros = ["Simpler"]
cons = ["No persistence"]
rejection_reason = "Persistence is required for our use case"
```

ADRs contain:

- **Context** — The situation requiring a decision
- **Decision** — What was decided
- **Consequences** — Expected outcomes (positive and negative)
- **Alternatives** — Options considered with pros, cons, and rejection reasons (per [[ADR-0027]])
- **Status** — `proposed`, `accepted`, `rejected`, or `superseded`

## Editing ADRs

Use govctl to get/set fields:

```bash
# Get specific field
govctl adr get ADR-0003 status

# Set field value
govctl adr set ADR-0003 status accepted

# Set multi-line content from stdin
govctl adr set ADR-0003 context --stdin <<'EOF'
We need a caching layer that can handle
10k requests per second with sub-millisecond latency.
EOF
```

For alternatives (pros/cons/rejection reason), path-based edits are supported:

```bash
# Direct nested edit
govctl adr set ADR-0001 "alt[2].pros[0]" "Updated pro"
govctl adr add ADR-0001 "alt[0].cons" "New disadvantage"
```

## Status Lifecycle

```
proposed → accepted → superseded
         ↘ rejected
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

## Listing and Viewing

```bash
govctl adr list
govctl adr list accepted    # Filter by status
govctl adr show ADR-0003    # Styled markdown to stdout
```
