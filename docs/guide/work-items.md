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
govctl work move WI-2026-01-17-001 active
govctl work move WI-2026-01-17-001 done

# By filename (without path)
govctl work move implement-caching.toml active
```

## Acceptance Criteria

### Add Criteria

```bash
govctl work add WI-2026-01-17-001 acceptance_criteria "Unit tests pass"
govctl work add WI-2026-01-17-001 acceptance_criteria "Documentation updated"
```

### Mark Criteria Complete

```bash
govctl work tick WI-2026-01-17-001 acceptance_criteria "Unit tests" -s done
```

The pattern matches case-insensitively by substring.

## Notes

Add context or progress notes:

```bash
govctl work add WI-2026-01-17-001 notes "Discovered edge case in validation"
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
govctl delete WI-2026-01-17-999 -f
# Or explicitly specify it's a work item:
govctl delete --work WI-2026-01-17-999 -f
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
