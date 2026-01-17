# Working with RFCs

RFCs (Requests for Comments) are the normative specifications in govctl. They define what will be built before implementation begins.

## Creating RFCs

```bash
# Auto-assign next available ID
govctl new rfc "Feature Title"

# Specify ID manually
govctl new rfc "Feature Title" --id RFC-0010
```

## RFC Structure

An RFC consists of:

- **Metadata** (`rfc.json`) — ID, title, status, phase, version
- **Clauses** — Atomic units of specification

## Working with Clauses

### Create a Clause

```bash
govctl new clause RFC-0010:C-SCOPE "Scope" -s "Specification" -k normative
```

Options:

- `-s, --section` — Section name (e.g., "Specification", "Rationale")
- `-k, --kind` — `normative` (binding) or `informative` (explanatory)

### Edit Clause Text

```bash
# From stdin
govctl edit RFC-0010:C-SCOPE --stdin <<'EOF'
The system MUST validate all inputs.
The system SHOULD log validation failures.
EOF

# Open in editor
govctl edit RFC-0010:C-SCOPE
```

### List Clauses

```bash
govctl list clause
govctl list clause --rfc RFC-0010
```

## Status Lifecycle

RFCs have three statuses:

```
draft → normative → deprecated
```

### Finalize to Normative

When the spec is complete and approved:

```bash
govctl finalize RFC-0010 normative
```

This makes the RFC binding — implementation must conform to it.

### Deprecate

When an RFC is superseded or obsolete:

```bash
govctl finalize RFC-0010 deprecated
```

## Phase Lifecycle

RFCs progress through four phases:

```
spec → impl → test → stable
```

### Advance Phase

```bash
govctl advance RFC-0010 impl    # Ready for implementation
govctl advance RFC-0010 test    # Implementation complete, ready for testing
govctl advance RFC-0010 stable  # Tested, ready for production
```

Phase transitions are gated:

- `spec → impl` requires `status = normative`
- Each phase has invariants that must be satisfied

## Versioning

RFCs use semantic versioning:

```bash
# Bump version with changelog entry
govctl bump RFC-0010 --patch -m "Fix typo in clause C-SCOPE"
govctl bump RFC-0010 --minor -m "Add new clause for edge case"
govctl bump RFC-0010 --major -m "Breaking change to API contract"
```

## Listing RFCs

```bash
govctl list rfc
govctl list rfc --status normative
govctl list rfc --phase impl
```
