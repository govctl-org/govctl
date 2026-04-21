# Working with Work Items

Work Items track units of work from inception to completion. They provide an audit trail of what was done and when.

> **See also:** [Tags](../guide/validation.md#controlled-vocabulary-tags), [TUI](../guide/getting-started.md#interactive-tui), [Canonical Edit](../guide/getting-started.md#canonical-edit-surface)

## Creating Work Items

```bash
# Create in queue (pending)
govctl work new "Implement caching layer"

# Create and activate immediately
govctl work new --active "Urgent bug fix"
```

Work items are automatically assigned IDs like `WI-2026-01-17-001`.

## Work Item Structure

Work items are TOML files with `#:schema` headers:

```toml
#:schema ../schema/work.schema.json

[govctl]
id = "WI-2026-01-17-001"
title = "Implement caching layer"
status = "active"
started = "2026-01-17"
refs = ["RFC-0010", "ADR-0003"]

[content]
description = "Add Redis caching for the query endpoint."

[[content.acceptance_criteria]]
text = "add: Cache invalidation on write"
status = "pending"

[[content.acceptance_criteria]]
text = "chore: govctl check passes"
status = "pending"

[[content.journal]]
date = "2026-01-17"
text = "Implemented core logic"
scope = "backend"

[[content.notes]]
text = "Do not retry the old validation path"
```

Work items contain:

- **Title** — Brief description
- **Description** — Task scope declaration
- **Journal** — Execution tracking entries with date and scope (per [[ADR-0026]])
- **Notes** — Durable learnings and constraints
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

Moving to `done` requires all verification guards to pass (see [Validation](./validation.md#verification-guards)).

## Acceptance Criteria

### Add Criteria

```bash
govctl work add WI-2026-01-17-001 acceptance_criteria "chore: Unit tests pass"
govctl work add WI-2026-01-17-001 acceptance_criteria "add: Documentation updated"
```

Category prefixes (`add:`, `fix:`, `change:`, `chore:`, etc.) are required and drive changelog generation. Conventional-commit aliases like `feat:`, `refactor:`, `test:`, `docs:` are also accepted.

Canonical changelog categories are still the preferred form in stored artifacts. The conventional-commit aliases are accepted as input sugar and normalized into the changelog model.

### Mark Criteria Complete

```bash
govctl work tick WI-2026-01-17-001 acceptance_criteria "Unit tests" -s done
```

The pattern matches case-insensitively by substring.

### Canonical Edit Paths

All work item fields are accessible through the unified path-based edit interface:

```bash
# Set scalar fields
govctl work edit WI-2026-01-17-001 content.description --stdin <<'EOF'
New description here
EOF

# Add to array fields
govctl work edit WI-2026-01-17-001 refs --add RFC-0010
govctl work edit WI-2026-01-17-001 acceptance_criteria --add "fix: Handle edge case"

# Remove by index
govctl work edit WI-2026-01-17-001 acceptance_criteria --at 0 --remove

# Tick checklist items
govctl work edit WI-2026-01-17-001 acceptance_criteria --tick done --at 0
govctl work edit WI-2026-01-17-001 acceptance_criteria --tick cancelled --at 1

# Nested journal fields
govctl work edit WI-2026-01-17-001 "journal[0].scope" --set backend
govctl work edit WI-2026-01-17-001 "journal[0].content" --stdin <<'EOF'
Detailed progress update here
EOF
```

Path aliases are available for common fields:

| Alias         | Resolves to                               |
| ------------- | ----------------------------------------- |
| `description` | `content.description`                     |
| `ac`          | `content.acceptance_criteria`             |
| `journal`     | `content.journal`                         |
| `notes`       | `content.notes`                           |
| `category`    | `content.acceptance_criteria[i].category` |
| `scope`       | `content.journal[i].scope`                |

### Tagging Work Items

Once tags are registered in the project vocabulary, apply them to work items:

```bash
govctl work edit WI-2026-01-17-001 tags --add backend
govctl work edit WI-2026-01-17-001 tags --add performance
```

Filter lists by tag:

```bash
govctl work list --tag backend
govctl work list --tag backend,performance
```

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

## Per-Work-Item Guards

Work items can require extra verification guards in addition to the project's default guard set.

Example:

```toml
[verification]
required_guards = ["GUARD-CLIPPY"]
```

This means:

- `GUARD-CLIPPY` is required for this work item even if it is not a project default
- project defaults from `gov/config.toml` still apply when verification is enabled
- the work item cannot move to `done` until the effective required guards pass or are explicitly waived

To run the effective guard set for a single work item:

```bash
govctl verify --work WI-2026-01-17-001
```

### Waiving A Guard

If a specific guard must be waived for this work item, record that in the artifact with a reason:

```toml
[[verification.waivers]]
guard = "GUARD-CARGO-TEST"
reason = "Temporarily flaky on macOS runners; tracked in issue #123"
```

Waivers are per-work-item only. They do not disable the guard globally, and they should remain rare and justified.

## Notes

Add durable notes for future steps:

```bash
govctl work add WI-2026-01-17-001 notes "Do not retry the old validation path; it fails on missing refs"
```

Nested path edits are also available for structured fields:

```bash
govctl work set WI-2026-01-17-001 "journal[0].scope" "parser"
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

## Listing and Viewing

```bash
govctl work list
govctl work list queue      # Pending items
govctl work list active     # In progress
govctl work list done       # Completed
govctl work show WI-2026-01-17-001  # Styled markdown to stdout
```
