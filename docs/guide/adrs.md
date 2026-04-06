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
selected_option = "Redis"

[content.consequences]
positive = ["Faster reads"]
neutral = ["Adds a dedicated cache tier"]

[[content.consequences.negative]]
text = "Operational complexity increases"
mitigations = ["Use managed Redis with automated backups"]

[[content.alternatives]]
text = "Memcached"
pros = ["Simpler"]
cons = ["No persistence"]
rejection_reason = "Persistence is required for our use case"
```

ADRs contain:

- **Context** — The situation requiring a decision
- **Decision** — What was decided and why
- **Selected option** — The chosen path recorded explicitly
- **Consequences** — Expected outcomes grouped as positive, negative, and neutral
- **Alternatives** — Non-selected options with pros, cons, and rejection reasons
- **Status** — `proposed`, `accepted`, `rejected`, or `superseded`

## Editing ADRs

Use canonical path-first edits for ADR content:

```bash
# Get specific field
govctl adr get ADR-0003 status

# Set multi-line context from stdin
govctl adr edit ADR-0003 context --set --stdin <<'EOF'
We need a caching layer that can handle
10k requests per second with sub-millisecond latency.
EOF

# Record the chosen option explicitly
govctl adr edit ADR-0003 selected_option --set "Redis"

# Add structured consequences
govctl adr edit ADR-0003 consequences.positive --add "Faster reads"
govctl adr edit ADR-0003 consequences.negative --add "Operational complexity increases"
govctl adr edit ADR-0003 consequences.negative[0].mitigations --add "Use managed Redis"

# Add rejected alternatives and rationale
govctl adr edit ADR-0003 alternatives --add "Memcached"
govctl adr edit ADR-0003 alternatives[0].pros --add "Simpler"
govctl adr edit ADR-0003 alternatives[0].cons --add "No persistence"
govctl adr edit ADR-0003 alternatives[0].rejection_reason --set "Persistence is required"
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

Accepted ADRs must record `selected_option` before they can be accepted.

### Supersede

When a new decision replaces an old one:

```bash
govctl adr supersede ADR-0001 --by ADR-0005
```

This marks ADR-0001 as superseded and records ADR-0005 as its replacement.

ADRs are superseded rather than deprecated. If a decision is obsolete, create a replacement ADR and supersede the old one.

## Listing and Viewing

```bash
govctl adr list
govctl adr list accepted    # Filter by status
govctl adr show ADR-0003    # Styled markdown to stdout
```
