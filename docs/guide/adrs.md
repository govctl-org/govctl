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

# Set content field value
govctl adr set ADR-0003 decision "We will use Redis because..."

# Set multi-line content from stdin
govctl adr set ADR-0003 context --stdin <<'EOF'
We need a caching layer that can handle
10k requests per second with sub-millisecond latency.
EOF
```

### Canonical Edit Paths

All ADR fields are accessible through a unified path-based edit interface:

```bash
# Scalar fields
govctl adr edit ADR-0003 content.decision --set "We will use Redis"
govctl adr edit ADR-0003 content.context --stdin < context.md

# Array fields — add, remove, tick
govctl adr edit ADR-0003 refs --add RFC-0010
govctl adr edit ADR-0003 refs --at 0 --remove

# Nested alternatives
govctl adr edit ADR-0003 content.alternatives --add "Option C: Use etcd"
govctl adr edit ADR-0003 "content.alternatives[0].pros" --add "Fast reads"
govctl adr edit ADR-0003 "content.alternatives[0].cons" --add "Operational cost"
govctl adr edit ADR-0003 "content.alternatives[0].status" --set accepted
govctl adr edit ADR-0003 "content.alternatives[0].rejection_reason" --set "Too complex"

# Tick alternative status
govctl adr edit ADR-0003 content.alternatives --tick accepted --at 0
```

Path aliases are available for common fields:

| Alias          | Resolves to                                |
| -------------- | ------------------------------------------ |
| `decision`     | `content.decision`                         |
| `context`      | `content.context`                          |
| `consequences` | `content.consequences`                     |
| `alt`          | `content.alternatives`                     |
| `pro`          | `content.alternatives[i].pros`             |
| `con`          | `content.alternatives[i].cons`             |
| `reason`       | `content.alternatives[i].rejection_reason` |

### Legacy Set/Add/Remove Verbs

The original verbs remain available and compile into the same edit pipeline:

```bash
govctl adr set ADR-0003 decision "We will use Redis because..."
govctl adr add ADR-0003 alternatives "Option C"
govctl adr remove ADR-0003 refs RFC-0001
```

### Tagging ADRs

Once tags are registered in the project vocabulary, apply them to ADRs:

```bash
govctl adr edit ADR-0003 tags --add caching
govctl adr edit ADR-0003 tags --add performance
```

Filter lists by tag:

```bash
govctl adr list --tag caching
govctl adr list --tag caching,performance
```

## Status Lifecycle

```
proposed → accepted → superseded
         ↘ rejected
```

### Accept a Decision

Before accepting, the ADR must have at least 2 alternatives with 1 accepted and 1 rejected per [[ADR-0042]]:

```bash
govctl adr edit ADR-0003 "content.alternatives[0].status" --set accepted
govctl adr edit ADR-0003 "content.alternatives[1].status" --set rejected

govctl adr accept ADR-0003
```

Use `--force` for historical backfills where alternatives cannot be reconstructed:

```bash
govctl adr accept ADR-0003 --force
```

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
