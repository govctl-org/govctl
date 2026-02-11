---
name: wi-writer
description: "Write well-structured work items with proper acceptance criteria. Use when: (1) Creating work items, (2) Adding acceptance criteria, (3) User mentions work item, task, WI, or ticket"
---

# Work Item Writer

Write work items with clear descriptions and actionable acceptance criteria.

## Quick Reference

```bash
{{GOVCTL}} work new --active "<title>"
{{GOVCTL}} work add <WI-ID> acceptance_criteria "<category>: <description>"
{{GOVCTL}} work add <WI-ID> refs RFC-NNNN
{{GOVCTL}} work tick <WI-ID> acceptance_criteria "<pattern>" -s done
{{GOVCTL}} work move <WI-ID> done
```

## Work Item Structure

### Title

Concise, action-oriented. Describes *what* will be done.

- Good: "Add validation for clause cross-references"
- Bad: "Fix stuff" or "Work on the thing"

### Description

Replace the placeholder immediately. One paragraph explaining:
- What the work accomplishes
- Why it's needed
- Any relevant context

### Acceptance Criteria

**Every criterion MUST have a category prefix** for changelog generation:

| Prefix | Changelog Section | Use for |
|--------|-------------------|---------|
| `add:` | Added | New features, capabilities |
| `changed:` | Changed | Modifications to existing behavior |
| `deprecated:` | Deprecated | Features marked for removal |
| `removed:` | Removed | Deleted features |
| `fix:` | Fixed | Bug fixes |
| `security:` | Security | Security-related changes |
| `chore:` | _(excluded)_ | Internal tasks, tests, maintenance |

```bash
# Feature work
{{GOVCTL}} work add <WI-ID> acceptance_criteria "add: Implement clause validation"
{{GOVCTL}} work add <WI-ID> acceptance_criteria "add: Error messages include clause ID"

# Bug fix
{{GOVCTL}} work add <WI-ID> acceptance_criteria "fix: Duplicate clause detection"

# Internal
{{GOVCTL}} work add <WI-ID> acceptance_criteria "chore: All tests pass"
{{GOVCTL}} work add <WI-ID> acceptance_criteria "chore: govctl check passes"
```

### References

Link to governing artifacts:

```bash
{{GOVCTL}} work add <WI-ID> refs RFC-0001
{{GOVCTL}} work add <WI-ID> refs ADR-0023
```

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
{{GOVCTL}} work tick <WI-ID> acceptance_criteria "<pattern>" -s done

# When all criteria are done, close the work item
{{GOVCTL}} work move <WI-ID> done
```

### The `chore:` Pattern

Always add at least one `chore:` criterion for validation:

```bash
{{GOVCTL}} work add <WI-ID> acceptance_criteria "chore: govctl check passes"
```

This ensures validation is an explicit gate, not an afterthought.

## Common Mistakes

| Mistake | Fix |
|---------|-----|
| Missing category prefix | Always use `add:`, `fix:`, `chore:`, etc. |
| Placeholder description left in | Replace immediately with real description |
| Vague criteria: "Feature works" | Specific: "add: CLI returns exit code 0 on success" |
| No `chore:` criterion | Add "chore: govctl check passes" or "chore: all tests pass" |
| No refs to governing artifacts | Link RFCs/ADRs with `work add <WI-ID> refs` |
