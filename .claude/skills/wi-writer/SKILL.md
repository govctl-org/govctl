---
name: wi-writer
description: "Write well-structured work items with proper acceptance criteria. Use when: (1) Creating work items, (2) Adding acceptance criteria, (3) User mentions work item, task, WI, or ticket"
---

# Work Item Writer

Write work items with clear descriptions and actionable acceptance criteria.

## Quick Reference

```bash
govctl work new --active "<title>"
govctl work add <WI-ID> acceptance_criteria "<category>: <description>"
govctl work add <WI-ID> refs RFC-NNNN
govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done
govctl work move <WI-ID> done
```

## Work Item Structure

### Title

Concise, action-oriented. Describes _what_ will be done.

- Good: "Add validation for clause cross-references"
- Bad: "Fix stuff" or "Work on the thing"

### Description

**Purpose:** Task scope declaration — what needs to be done.

Replace the placeholder immediately. One paragraph explaining:

- What the work accomplishes
- Why it's needed
- Any relevant context

**Important:** Description is for task scope, NOT execution tracking. Use `journal` for progress updates.

### Journal

**Purpose:** Execution process tracking — how the work is progressing.

Journal entries record progress updates, bug fixes, and verification results during execution. Each entry has:

- `date` (required): ISO date "YYYY-MM-DD"
- `scope` (optional): Topic/module identifier
- `content` (required): Markdown text with details

```bash
# Add journal entry via TOML editing (no CLI command yet)
# In work item TOML file:
[[content.journal]]
date = "2026-02-22"
scope = "render"
content = "Added journal section rendering to work item output."
```

**When to add journal entries:**

- After completing a significant chunk of work
- When fixing bugs during implementation
- After running verification gates
- When making design decisions mid-execution

### Notes

**Purpose:** Ad-hoc key points — quick observations to remember.

Notes are concise points recorded anytime, not just at completion. Use for:

- Key observations discovered during work
- Reminders for future maintainers
- Brief insights that don't fit in journal

```bash
# Add note
# In work item TOML file:
[content]
notes = ["Remember to update migration guide", "API is now async"]
```

### Acceptance Criteria

**Every criterion MUST have a category prefix** for changelog generation:

| Prefix        | Changelog Section | Use for                            |
| ------------- | ----------------- | ---------------------------------- |
| `add:`        | Added             | New features, capabilities         |
| `changed:`    | Changed           | Modifications to existing behavior |
| `deprecated:` | Deprecated        | Features marked for removal        |
| `removed:`    | Removed           | Deleted features                   |
| `fix:`        | Fixed             | Bug fixes                          |
| `security:`   | Security          | Security-related changes           |
| `chore:`      | _(excluded)_      | Internal tasks, tests, maintenance |

```bash
# Feature work
govctl work add <WI-ID> acceptance_criteria "add: Implement clause validation"
govctl work add <WI-ID> acceptance_criteria "add: Error messages include clause ID"

# Bug fix
govctl work add <WI-ID> acceptance_criteria "fix: Duplicate clause detection"

# Internal
govctl work add <WI-ID> acceptance_criteria "chore: All tests pass"
govctl work add <WI-ID> acceptance_criteria "chore: govctl check passes"
```

### References

Link to governing artifacts:

```bash
govctl work add <WI-ID> refs RFC-0001
govctl work add <WI-ID> refs ADR-0023
```

## Field Semantics Summary

| Field                 | Purpose                    | Update Pattern             |
| --------------------- | -------------------------- | -------------------------- |
| `description`         | Task scope declaration     | Define once, rarely change |
| `journal`             | Execution process tracking | Append on each progress    |
| `notes`               | Ad-hoc key points          | Add anytime, concise       |
| `acceptance_criteria` | Completion criteria        | Define then tick           |

**Per ADR-0026:** Keep description focused on "what" - use journal for "how it's going."

## Writing Rules

### Acceptance Criteria Quality

Each criterion should be:

- **Specific** — "Add `validate_refs()` function" not "Add validation"
- **Testable** — Can be verified as done/not-done with no ambiguity
- **Independent** — Each criterion stands alone
- **Categorized** — Always include the category prefix

### Completion Flow

Work items cannot be marked done without ticking all criteria:

```bash
# Tick criteria as you complete them
govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done

# When all criteria are done, close the work item
govctl work move <WI-ID> done
```

### The `chore:` Pattern

Always add at least one `chore:` criterion for validation:

```bash
govctl work add <WI-ID> acceptance_criteria "chore: govctl check passes"
```

This ensures validation is an explicit gate, not an afterthought.

## Common Mistakes

| Mistake                          | Fix                                                         |
| -------------------------------- | ----------------------------------------------------------- |
| Missing category prefix          | Always use `add:`, `fix:`, `chore:`, etc.                   |
| Placeholder description left in  | Replace immediately with real description                   |
| Vague criteria: "Feature works"  | Specific: "add: CLI returns exit code 0 on success"         |
| No `chore:` criterion            | Add "chore: govctl check passes" or "chore: all tests pass" |
| No refs to governing artifacts   | Link RFCs/ADRs with `work add <WI-ID> refs`                 |
| Description used for tracking    | Use journal field for execution progress per ADR-0026       |
| No journal entries for long task | Add journal entries for significant progress updates        |
