---
name: quick
description: "Execute the fast path for trivial changes with minimal governance ceremony. Use when: (1) User invokes /quick, (2) Change is doc-only or non-behavioral, (3) No RFC or ADR work is needed"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# /quick - Fast Path Workflow

Execute the lightweight workflow for: `$ARGUMENTS`

Use this only for trivial, non-behavioral changes such as typos, comments, docs fixes, or small internal cleanup.

**Outputs:** Completed trivial non-behavioral change and validation evidence. Work-item updates are optional and only used when durable tracking already exists or is explicitly needed.

Do not use this for new behavior, RFC-governed work, or architecture decisions. If the task stops being trivial, switch to `/gov`.

## Critical Rules

1. Keep the fast path small. Do not invent governance work the change does not need.
2. Do not create work-item noise. For trivial cleanup, docs fixes, typos, comments, and small internal maintenance, prefer no work item unless a matching active/queued item already exists or the user explicitly asks for one.
3. If using a work item, read it with `govctl work show <WI-ID>`.
4. Use work item fields correctly:
   - `description`: scope and why
   - `notes`: closure-worthy durable constraints or lessons only
5. If the change becomes behavioral, ambiguous, or architectural, stop using `/quick` and switch to `/gov`.
6. Use `/commit` for raw VCS operations. Do not embed `jj` or `git` procedures here.
7. Do not create multiple work items for trivial cleanup batches. Use the commit diff as the record, or one coarse work item only when durable tracking is explicitly valuable.

## Workflow

### 1. Validate and classify

```bash
govctl status
```

- Confirm the change is still trivial and non-behavioral.
- `/commit` will choose the raw VCS workflow if recording is needed.

### 2. Check optional work-item context

```bash
govctl work list pending
```

- Matching active item that fits this exact cleanup: use it
- Matching queued item: `govctl work move <WI-ID> active`
- No matching item: continue without a work item for trivial cleanup.
- User explicitly wants tracking, or the cleanup has a durable reader-facing outcome: create at most one coarse work item.
- Interrupted tracked batch work: run `govctl loop list open` before resuming so you use the persisted generated loop ID.

If using a work item:

```bash
govctl work show <WI-ID>
govctl work set <WI-ID> description "Brief scope: what and why"
govctl work add <WI-ID> acceptance_criteria "chore: govctl check passes"
govctl work add <WI-ID> acceptance_criteria "<category>: <specific observable outcome for this trivial change>"
```

The second criterion must be concrete and diff-specific. Examples:

- `docs: CLI example uses the current subcommand name`
- `chore: remove unused import from parser module`
- `fix: typo in error message is corrected`

### 3. Implement

If using a work item, verify the active gate before editing:

```bash
govctl check --has-active
```

Otherwise, make the change without creating a work item. If code comments reference governance artifacts, use `[[artifact-id]]`.

Run the relevant validation:

```bash
govctl check
```

If using a work item, add a note only when there is a durable lesson that should remain useful after closure:

```bash
govctl work add <WI-ID> notes "Do not use the old command name in generated examples; it was removed in v0.9"
```

Do not write progress, command output, review status, current plans, next actions, temporary blockers, or TODOs to `notes`. Transient execution progress belongs in loop state and round artifacts, not in work item fields.
When a tracked cleanup batch gains or loses durable roots, use `govctl loop add <LOOP-ID> work <ROOT-WI-ID>`, `govctl loop remove <LOOP-ID> work <ROOT-WI-ID>`, or `govctl loop replan <LOOP-ID>` rather than creating a new loop for each small item. `wi` is accepted as a short alias for the loop `work` field, but examples should prefer `work`.

### 4. Complete

If a work item was used, tick matching criteria and move it to `done` only when all acceptance criteria are satisfied; otherwise, keep it active. If no work item was used, skip this step.

### 5. Record

Record the implementation change with `/commit`, typically using `docs(scope)`, `chore(scope)`, or `fix(scope)` as appropriate. Include any work-item closure in this commit; do not create a separate closure commit by default.

## Switch to /gov when

- The change affects behavior
- The governing RFC is unclear or missing
- An ADR-level design choice appears
- The task stops being obviously trivial

**BEGIN EXECUTION NOW.**
