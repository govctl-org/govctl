# Working with Work Items

Work Items track units of work from inception to completion. They provide an audit trail of what was done and when.

## Creating Work Items

```bash
# Create in queue (pending)
govctl new work "Implement caching layer"

# Create and activate immediately
govctl new work --active "Urgent bug fix"
```

Work items are automatically assigned IDs like `WI-2026-01-17-001`.

## Work Item Structure

Work items contain:

- **Title** — Brief description
- **Notes** — Detailed context (array of strings)
- **Acceptance Criteria** — Checkable completion criteria
- **Refs** — Links to related RFCs, ADRs, or external resources

## Status Lifecycle

```
queue → active → done
              ↘ blocked
              ↘ cancelled
```

### Move Between States

```bash
# By ID
govctl mv WI-2026-01-17-001 active
govctl mv WI-2026-01-17-001 done

# By filename (without path)
govctl mv implement-caching.toml active
```

## Acceptance Criteria

### Add Criteria

```bash
govctl add WI-2026-01-17-001 acceptance_criteria "Unit tests pass"
govctl add WI-2026-01-17-001 acceptance_criteria "Documentation updated"
```

### Mark Criteria Complete

```bash
govctl tick WI-2026-01-17-001 acceptance_criteria "Unit tests" -s done
```

The pattern matches case-insensitively by substring.

## Notes

Add context or progress notes:

```bash
govctl add WI-2026-01-17-001 notes "Discovered edge case in validation"
```

## Removing Items

Remove items from array fields using flexible matching:

```bash
# Substring match (default, case-insensitive)
govctl remove WI-2026-01-17-001 notes "edge case"

# Exact match
govctl remove WI-2026-01-17-001 notes "Discovered edge case in validation" --exact

# By index (0-based)
govctl remove WI-2026-01-17-001 notes --at 0

# Negative index (from end)
govctl remove WI-2026-01-17-001 notes --at -1

# Regex pattern
govctl remove WI-2026-01-17-001 refs "RFC-.*" --regex

# Remove all matches
govctl remove WI-2026-01-17-001 refs "obsolete" --all
```

## Listing Work Items

```bash
govctl list work
govctl list work queue      # Pending items
govctl list work active     # In progress
govctl list work done       # Completed
```

## Why TOML?

Like ADRs, work items use TOML for human-friendly editing with comments and clean multi-line strings.
