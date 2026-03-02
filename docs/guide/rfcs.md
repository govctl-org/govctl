# Working with RFCs

RFCs (Requests for Comments) are the normative specifications in govctl. They define what will be built before implementation begins.

## Creating RFCs

```bash
# Auto-assign next available ID
govctl rfc new "Feature Title"

# Specify ID manually
govctl rfc new "Feature Title" --id RFC-0010
```

## RFC Structure

An RFC consists of:

- **Metadata** (`rfc.json`) — ID, title, status, phase, version
- **Clauses** — Atomic units of specification

## Working with Clauses

### Create a Clause

```bash
govctl clause new RFC-0010:C-SCOPE "Scope" -s "Specification" -k normative
```

Options:

- `-s, --section` — Section name (e.g., "Specification", "Rationale")
- `-k, --kind` — `normative` (binding) or `informative` (explanatory)

### Edit Clause Text

```bash
# From stdin (recommended for multi-line)
govctl clause edit RFC-0010:C-SCOPE --stdin <<'EOF'
The system MUST validate all inputs.
The system SHOULD log validation failures.
EOF

# Inline text
govctl clause edit RFC-0010:C-SCOPE --text "The system MUST validate all inputs."

# From file
govctl clause edit RFC-0010:C-SCOPE --text-file clause-text.md
```

### Delete a Clause

Accidentally created clauses can be deleted from **draft** RFCs only:

```bash
govctl clause delete RFC-0010:C-MISTAKE -f
```

**Safety:** Deletion is only allowed when:

- The RFC status is `draft` (normative RFCs are immutable)
- No other artifacts reference the clause

For normative RFCs, use `govctl clause deprecate RFC-0010:C-OLD` instead.

### List Clauses

```bash
govctl clause list
govctl clause list RFC-0010
```

## Status Lifecycle

RFCs have three statuses:

```
draft → normative → deprecated
```

### Finalize to Normative

When the spec is complete and approved:

```bash
govctl rfc finalize RFC-0010 normative
```

This makes the RFC binding — implementation must conform to it.

### Deprecate

When an RFC is superseded or obsolete:

```bash
govctl rfc finalize RFC-0010 deprecated
```

## Phase Lifecycle

RFCs progress through four phases:

```
spec → impl → test → stable
```

### Advance Phase

```bash
govctl rfc advance RFC-0010 impl    # Ready for implementation
govctl rfc advance RFC-0010 test    # Implementation complete, ready for testing
govctl rfc advance RFC-0010 stable  # Tested, ready for production
```

Phase transitions are gated:

- `spec → impl` requires `status = normative`
- Each phase has invariants that must be satisfied

## Versioning

RFCs use semantic versioning:

```bash
# Bump version with changelog entry
govctl rfc bump RFC-0010 --patch -m "Fix typo in clause C-SCOPE"
govctl rfc bump RFC-0010 --minor -m "Add new clause for edge case"
govctl rfc bump RFC-0010 --major -m "Breaking change to API contract"
```

## Listing RFCs

```bash
govctl rfc list
govctl rfc list normative    # Filter by status
govctl rfc list impl         # Filter by phase
```
