---
name: wi-writer
description: "Write well-structured work items with proper acceptance criteria. Use when: (1) Creating work items, (2) Adding acceptance criteria, (3) User mentions work item, task, WI, or ticket"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional work-item topic]
---

# Work Item Writer

Write work items with clear descriptions and actionable acceptance criteria.

## Invocation Mode

This helper skill may be used standalone or by `/gov`, `/quick`, or `/commit`.
It is responsible for work-item content quality and field semantics, not code changes or VCS operations.

## Authority

Work items track execution: what this task is doing, what happened, and what remains before closure.
They are operational memory, not normative authority and not decision records.

## Quick Reference

```bash
govctl work new --active "<title>"
govctl work set <WI-ID> description "Task scope description"
govctl work add <WI-ID> acceptance_criteria "<category>: <description>"
govctl work add <WI-ID> journal "Progress update" --scope module
govctl work add <WI-ID> notes "Key observation"
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

**Important:** Description is for task scope, NOT execution tracking. Use `journal` for progress and outcomes, and `notes` for durable learnings.
It must not introduce new product behavior requirements that are missing from the governing RFC or ADR.

### Journal

**Purpose:** Execution process tracking — how the work is progressing.

Journal entries record progress updates, bug fixes, and verification results during execution. Each entry has:

- `date` (required): ISO date "YYYY-MM-DD"
- `scope` (optional): Topic/module identifier
- `content` (required): Markdown text with details

```bash
# Add journal entry (date is auto-filled to today)
govctl work add <WI-ID> journal "Added journal section rendering to work item output."

# With scope (topic/module tag)
govctl work add <WI-ID> journal "Fixed parser edge case" --scope parser

# Multi-line via stdin
govctl work add <WI-ID> journal --scope render --stdin <<'EOF'
Completed the rendering pipeline.
All snapshot tests updated.
EOF
```

**When to add journal entries:**

- After completing a significant chunk of work
- When fixing bugs during implementation
- After running verification gates
- After a failed attempt that changed the next step

### Notes

**Purpose:** Durable learnings — constraints, decisions, and retry rules to remember before the next step.

Notes are concise points recorded anytime, not just at completion. Use for:

- Why an approach failed
- What not to retry
- Constraints or decisions future steps must obey

These notes may explain local execution constraints, but they do not override RFCs or accepted ADRs.

```bash
govctl work add <WI-ID> notes "Remember to update migration guide"
govctl work add <WI-ID> notes "API is now async"
```

### Acceptance Criteria

**Every criterion MUST have a category prefix** for changelog generation:

| Prefix       | Changelog Section | Aliases                           |
| ------------ | ----------------- | --------------------------------- |
| `add:`       | Added             | `feat:`, `feature:`, `added:`     |
| `fix:`       | Fixed             | `fixed:`                          |
| `change:`    | Changed           | `changed:`, `refactor:`, `perf:`  |
| `remove:`    | Removed           | `removed:`                        |
| `deprecate:` | Deprecated        | `deprecated:`                     |
| `security:`  | Security          | `sec:`                            |
| `chore:`     | _(excluded)_      | `test:`, `docs:`, `ci:`, `build:` |

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

| Field                 | Purpose                    | Update Pattern                                |
| --------------------- | -------------------------- | --------------------------------------------- |
| `description`         | Task scope declaration     | Define once, rarely change                    |
| `journal`             | Execution process tracking | Append on each progress                       |
| `notes`               | Durable learnings          | Add when future steps must remember something |
| `acceptance_criteria` | Completion criteria        | Define then tick                              |

**Per ADR-0026:** Keep description focused on "what", journal on "what happened", and notes on "what to remember next".
If you discover a missing requirement or unresolved design choice, stop and route that back to RFC/ADR work rather than inventing it inside the work item.

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
| Work item invents new requirements | Move those requirements into an RFC or ADR first          |
