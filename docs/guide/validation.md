# Validation & Rendering

govctl provides tools to validate governance artifacts and render them to human-readable formats.

## Validation

### Check All Artifacts

```bash
govctl check
```

This validates:

- Schema conformance (all required fields present)
- Phase discipline (no invalid state transitions)
- Cross-references (refs point to existing artifacts)
- Clause structure (normative clauses in spec sections)

### Check Specific Types

```bash
govctl check rfc
govctl check adr
govctl check work
```

### Exit Codes

- `0` — All validations passed
- `1` — Validation errors found

## Rendering

Render governance artifacts to markdown for documentation.

### Render RFCs

```bash
# All RFCs (committed to repo)
govctl render

# Specific RFC
govctl render --rfc-id RFC-0010
```

Output goes to `docs/rfc/RFC-NNNN.md`.

### Render Other Artifacts

ADRs and work items render to `.gitignore`d local files by default:

```bash
govctl render adr      # → docs/adr/
govctl render work     # → docs/work/
govctl render all      # Everything
```

### Hash Signatures

Rendered markdown includes a SHA-256 signature for tampering detection:

```markdown
<!-- govctl:signature sha256:abc123... -->
```

If the source changes, the signature won't match — indicating the rendered doc is stale.

## Statistics

Get a summary of your governance state:

```bash
govctl stat
```

Shows:

- RFC counts by status and phase
- ADR counts by status
- Work item counts by status
- Any validation warnings

## Building Documentation

For mdbook integration:

```bash
./scripts/build-book.sh          # Build static site
./scripts/build-book.sh --serve  # Live preview
```

This renders all artifacts and generates the book structure.
