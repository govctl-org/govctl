# Working with Work Items

Work Items track units of work from inception to completion. They provide an audit trail of what was done and when.

## Creating Work Items

```bash
# Create in queue (pending)
govctl work new "Implement caching layer"

# Create and activate immediately
govctl work new --active "Urgent bug fix"
```

Work items are automatically assigned IDs like `WI-2026-01-17-001`.

## Work Item Structure

Work items contain:

- **Title** — Brief description
- **Description** — Task scope declaration
- **Journal** — Execution tracking entries with date and scope (per [[ADR-0026]])
- **Notes** — Ad-hoc key points (array of strings)
- **Acceptance Criteria** — Checkable completion criteria with changelog category
- **Refs** — Links to related RFCs, ADRs, or external resources

## Status Lifecycle

```
queue → active → done
    ↘        ↘ cancelled
```

### Move Between States

```bash
# By ID
govctl work move WI-2026-01-17-001 active
govctl work move WI-2026-01-17-001 done

# By filename (without path)
govctl work move implement-caching.toml active
```

## Acceptance Criteria

### Add Criteria

```bash
govctl work add WI-2026-01-17-001 acceptance_criteria "chore: Unit tests pass"
govctl work add WI-2026-01-17-001 acceptance_criteria "add: Documentation updated"
```

Category prefixes (`add:`, `fix:`, `change:`, `chore:`, etc.) are required and drive changelog generation. Conventional-commit aliases like `feat:`, `refactor:`, `test:`, `docs:` are also accepted.

### Mark Criteria Complete

```bash
govctl work tick WI-2026-01-17-001 acceptance_criteria "Unit tests" -s done
```

The pattern matches case-insensitively by substring.

## Journal

Track execution progress with dated journal entries:

```bash
govctl work add WI-2026-01-17-001 journal "Implemented core logic"

# With scope tag
govctl work add WI-2026-01-17-001 journal "Fixed edge case" --scope parser

# Multi-line via stdin
govctl work add WI-2026-01-17-001 journal --scope backend --stdin <<'EOF'
Completed the API layer.
All integration tests passing.
EOF
```

## Notes

Add durable notes for future steps:

```bash
govctl work add WI-2026-01-17-001 notes "Do not retry the old validation path; it fails on missing refs"
```

Nested path edits are also available for structured fields:

```bash
# Before: append a replacement journal entry to adjust one field
govctl work add WI-2026-01-17-001 journal --scope parser "Fixed parser edge case"

# After: direct nested update
govctl work set WI-2026-01-17-001 journal[0].scope "parser"
```

## Removing Items

Remove items from array fields using flexible matching:

```bash
# Substring match (default, case-insensitive)
govctl work remove WI-2026-01-17-001 notes "edge case"

# Exact match
govctl work remove WI-2026-01-17-001 notes "Discovered edge case in validation" --exact

# By index (0-based)
govctl work remove WI-2026-01-17-001 notes --at 0

# Negative index (from end)
govctl work remove WI-2026-01-17-001 notes --at -1

# Regex pattern
govctl work remove WI-2026-01-17-001 refs "RFC-.*" --regex

# Remove all matches
govctl work remove WI-2026-01-17-001 refs "obsolete" --all
```

## Deleting Work Items

Accidentally created work items can be deleted if they're still in **queue** status:

```bash
govctl work delete WI-2026-01-17-999 -f
```

**Safety:** Deletion is only allowed when:

- The work item status is `queue` (never activated)
- No other artifacts reference it

For work items that have been activated, use status transitions instead:

```bash
govctl work move WI-2026-01-17-001 cancelled
```

## Listing Work Items

```bash
govctl work list
govctl work list queue      # Pending items
govctl work list active     # In progress
govctl work list done       # Completed
```

## Why TOML?

Like ADRs, work items use TOML for human-friendly editing with comments and clean multi-line strings.
