---
name: quick
description: "Execute the fast path for trivial changes with minimal governance ceremony. Use when: (1) User invokes /quick, (2) Change is doc-only or non-behavioral, (3) No RFC or ADR work is needed"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# /quick - Fast Path Workflow

Execute the lightweight workflow for: `$ARGUMENTS`

Use this only for trivial, non-behavioral changes such as typos, comments, docs fixes, or small internal cleanup.

**Outputs:** Completed trivial non-behavioral change, updated work item memory, and validation evidence.

Do not use this for new behavior, RFC-governed work, or architecture decisions. If the task stops being trivial, switch to `/gov`.

## Critical Rules

1. Keep the fast path small. Do not invent governance work the change does not need.
2. Still use a work item. `govctl check --has-active` is the gate before editing.
3. Read the active work item with `govctl work show <WI-ID>`.
4. Use work item fields correctly:
   - `description`: scope and why
   - `journal`: what you did and what happened
   - `notes`: constraints or lessons future steps must remember
5. If the change becomes behavioral, ambiguous, or architectural, stop using `/quick` and switch to `/gov`.
6. Use `/commit` for raw VCS operations. Do not embed `jj` or `git` procedures here.

## Workflow

### 1. Validate and classify

```bash
govctl status
```

- Confirm the change is still trivial and non-behavioral.
- `/commit` will choose the raw VCS workflow if recording is needed.

### 2. Resolve the work item

```bash
govctl work list pending
```

- Matching active item: use it
- Matching queued item: `govctl work move <WI-ID> active`
- No match: `govctl work new --active "<concise-title>"`

Then:

```bash
govctl work show <WI-ID>
govctl work set <WI-ID> description "Brief scope: what and why"
govctl work add <WI-ID> acceptance_criteria "chore: Change completed"
```

### 3. Implement

Before editing:

```bash
govctl check --has-active
```

Make the change. If code comments reference governance artifacts, use `[[artifact-id]]`.

Run the relevant validation:

```bash
govctl check
```

Update working memory as needed:

```bash
govctl work add <WI-ID> journal "Updated docs; govctl check passes"
govctl work add <WI-ID> notes "Do not use old command name in examples"
```

For very small changes, `journal` may be enough. Add `notes` only when there is something future steps should remember.

### 4. Record

Record the implementation change with `/commit`, typically using `docs(scope)`, `chore(scope)`, or `fix(scope)` as appropriate.

### 5. Complete

```bash
govctl work tick <WI-ID> acceptance_criteria "Change completed" -s done
govctl work move <WI-ID> done
```

### 6. Final record

If work-item closure should be recorded separately, use `/commit` with `chore(work): complete <WI-ID>`.

## Switch to /gov when

- The change affects behavior
- The governing RFC is unclear or missing
- An ADR-level design choice appears
- The task stops being obviously trivial

**BEGIN EXECUTION NOW.**
