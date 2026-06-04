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

Work items track durable operational state: task scope, lifecycle, acceptance criteria, dependencies, references, and notes that should remain after closure.
They are operational memory, not normative authority and not decision records.

## Quick Reference

```bash
govctl work new --active "<title>"
govctl work set <WI-ID> description "Task scope description"
govctl work add <WI-ID> acceptance_criteria "<category>: <description>"
govctl work add <WI-ID> notes "Key observation"
govctl work add <WI-ID> refs RFC-NNNN
govctl work add <WI-ID> depends_on <BLOCKING-WI-ID>
govctl work add <WI-ID> tags <tag>
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

**Important:** Description is for task scope, NOT execution tracking. Use loop state and round artifacts for execution trace when available, and `notes` for durable learnings that belong on the work item.
It must not introduce new product behavior requirements that are missing from the governing RFC or ADR.

### Execution Trace

**Where execution information goes now:**

- Round-by-round execution trace belongs in loop state and round artifacts.
- Durable lessons, constraints, retry rules, and future-facing observations belong in `notes`.
- Acceptance progress belongs in `acceptance_criteria` status.

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

### Dependencies and Batches

Create multiple work items only when each item is independently meaningful to future readers:

- Each work item should represent a durable outcome, user-visible behavior, spec/ADR obligation, or independently reviewable deliverable.
- Do not create work items for mechanical helper extraction, fixture sharing, file moves, module normalization, formatting, comment cleanup, snapshot reshaping, or other low-level execution steps.
- For trivial cleanup or docs-only edits, no work item may be the right answer; follow the invoking workflow instead of inventing tracking.
- For one coherent cleanup/refactor, prefer one coarse work item over many narrow slices.
- Use `depends_on` only for hard execution ordering; keep `refs` for informational links.
- Use one batch loop only when there are multiple durable work items; do not use loops to justify creating mechanical work-item fragments.
- Let govctl generate the loop ID with `govctl loop start <ROOT-WI-ID> [<ROOT-WI-ID>...]`; use the returned `LOOP-YYYY-MM-DD-NNN` ID for later loop commands.
- Use `govctl loop list open` to discover existing non-terminal loops before resuming interrupted batch work.

If the batch scope changes after the loop starts, update the same loop:

```bash
govctl loop add <LOOP-ID> work <ROOT-WI-ID>
govctl loop remove <LOOP-ID> work <ROOT-WI-ID>
govctl loop replan <LOOP-ID>
```

`work` is the editable loop work-item field. `wi` is accepted as a short alias, but examples should prefer `work`.

Do not hand-write descriptive loop IDs or encode time finer than the day in loop IDs.

## Field Semantics Summary

| Field                 | Purpose                | Update Pattern                                |
| --------------------- | ---------------------- | --------------------------------------------- |
| `description`         | Task scope declaration | Define once, rarely change                    |
| `notes`               | Durable learnings      | Add when future steps must remember something |
| `acceptance_criteria` | Completion criteria    | Define then tick                              |

**Per ADR-0047:** Keep description focused on "what", notes on durable "what to remember next", and execution trace outside the work item field surface.
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

### Guardable Command Checks

Prefer verification guards over plain acceptance criteria for repeatable command-style checks.
When a criterion is only "`cargo test` passes", "`clippy` passes", or another shell command succeeds, first check whether an existing guard already covers it.
If so, require that guard through the work item's `[verification]` section instead of duplicating the command as a behavioral acceptance criterion.
Use `verification.required_guards` for per-work-item guard requirements.

Project-level `verification.default_guards` apply broadly to work-item completion when verification is enabled.
Use work-item-level `verification.required_guards` to add task-specific guards on top of those defaults, not to repeat guards that already apply globally.
Waivers apply to the effective guard set: project defaults plus work-item-specific required guards.

```toml
[verification]
required_guards = ["GUARD-CARGO-TEST"]
```

Use acceptance criteria for observable task outcomes.
Keep `chore:` criteria for validation summaries, especially when the validation is not fully enforced by a guard or the work item needs an explicit closure checklist item.

## Common Mistakes

| Mistake                            | Fix                                                                                  |
| ---------------------------------- | ------------------------------------------------------------------------------------ |
| Missing category prefix            | Always use `add:`, `fix:`, `chore:`, etc.                                            |
| Placeholder description left in    | Replace immediately with real description                                            |
| Vague criteria: "Feature works"    | Specific: "add: CLI returns exit code 0 on success"                                  |
| No `chore:` criterion              | Add "chore: govctl check passes" or "chore: all tests pass"                          |
| No refs to governing artifacts     | Link RFCs/ADRs with `work add <WI-ID> refs`                                          |
| Description used for tracking      | Use loop state and round artifacts for execution trace or `notes` for durable memory |
| Progress details stored as notes   | Keep `notes` durable; put transient round logs in loop state and round artifacts     |
| Mechanical substeps become WIs     | Use no WI or one coarse WI; leave helper/test/file-move details to the commit diff   |
| Work item invents new requirements | Move those requirements into an RFC or ADR first                                     |
