---
name: gov
description: "Execute governed implementation workflow with work items, RFC/ADR checks, phase gates, testing, and closure. Use when: (1) User invokes /gov, (2) A non-trivial change needs work item tracking, (3) Implementation may require RFC/ADR handling"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# /gov - Governed Workflow

Execute a complete, auditable workflow for: `$ARGUMENTS`

**Outputs:** Implemented change, updated governance artifacts where needed, validated tests/checks, and a completed work item.

## Agent Patterns

### CLI choice

Use `govctl` for governance operations.

When working on the `govctl` repo itself, use `cargo run --quiet --` instead. Commands below use `govctl` for brevity.

### Non-interactive commands

Use non-interactive CLI commands only. Prefer `--stdin` for multi-line content.

### Verification

After each governance write or substantive code change, run the relevant validation (`govctl check`, tests, render when needed).

## Quick Reference

```bash
govctl status
govctl work list pending
govctl work show <WI-ID>
govctl work new --active "<title>"
govctl work move <WI-ID> <status>
govctl work set <WI-ID> description "Scope and why"
govctl work add <WI-ID> acceptance_criteria "add: Implement feature X"
govctl work add <WI-ID> journal "Ran tests; fixed parser bug" --scope parser
govctl work add <WI-ID> notes "Do not retry old fixture path; it fails because snapshots are stale"
govctl work add <WI-ID> refs RFC-0001
govctl work add <WI-ID> tags <tag>
govctl tag new <tag>
govctl tag list
govctl rfc list
govctl adr list
govctl rfc new "<title>"
govctl adr new "<title>"
govctl rfc finalize <RFC-ID> normative
govctl rfc advance <RFC-ID> <impl|test|stable>
govctl check
govctl render
```

## Critical Rules

1. Use `govctl` for governance operations. Never edit governed files directly.
2. Respect phase discipline: `spec -> impl -> test -> stable`.
3. Behavior changes must be grounded in a normative RFC. If behavior is unspecified or ambiguous, stop and escalate.
4. Ask permission before `govctl rfc finalize ...` or `govctl rfc advance ...` unless `$ARGUMENTS` explicitly grant full authority.
5. Keep an active work item before implementation. `govctl check --has-active` is the gate.
6. In source comments, reference artifacts with `[[artifact-id]]`.
7. Use work item fields correctly:
   - `description`: task scope and why; set once, rarely change
   - `journal`: execution log; append actions and outcomes
   - `notes`: durable learnings; record constraints, decisions, retry rules, and failure causes
8. Avoid loops. If the same approach already failed, do not repeat it unchanged.
9. Spec-only governance maintenance does not belong here. Use `/spec` when no implementation work is required.
10. Work items are operational memory, not normative authority. If implementation needs a new requirement or design decision, amend the RFC or ADR instead of stuffing it into `description`, `journal`, or `notes`.

## Working Memory

The active work item is persistent working memory. Read it with `govctl work show <WI-ID>`; do not rely on raw TOML.

### Read order

1. `description` tells you the scope.
2. `journal` tells you what was tried and what happened.
3. `notes` tells you what to remember before the next attempt.
4. `acceptance_criteria` tells you what must be true before closure.

### Write rules

- Add a `journal` entry after meaningful progress, verification, or failure.
- Add a `notes` entry when you learn something future steps must obey.
- On failure, write both when appropriate:
  - `journal`: what you ran and what failed
  - `notes`: why it failed, what not to retry, or what to try instead

## Workflow

### 0. Initialize

```bash
govctl status
```

- Read `gov/config.toml`.
- Classify the task:
  - Doc-only: skip governance analysis, but still use a work item
  - Bug fix: usually no new RFC if behavior is already specified
  - Feature: likely requires an RFC or ADR
  - Deprecation or removal: amend the governing RFC before implementation
  - Refactor: ADR may be needed if it changes architecture

### 1. Resolve the work item

```bash
govctl work list pending
```

- Matching active item: use it
- Matching queued item: `govctl work move <WI-ID> active`
- No match: `govctl work new --active "<concise-title>"`

Then immediately:

```bash
govctl work show <WI-ID>
govctl work set <WI-ID> description "Brief scope: what and why"
govctl work add <WI-ID> acceptance_criteria "chore: govctl check passes"
```

Add task-specific acceptance criteria and refs as needed:

```bash
govctl work add <WI-ID> acceptance_criteria "add: Implement feature X"
govctl work add <WI-ID> refs RFC-0001
```

Follow the **wi-writer** skill for acceptance criteria quality.

### 2. Analyze governance

Skip this step for doc-only changes.

```bash
govctl rfc list
govctl adr list
```

Choose the smallest thing that matches reality:

- New behavior not covered by an RFC: draft an RFC
- Ambiguous interpretation or architectural choice: draft an ADR
- Deprecation or removal of specified behavior: amend the governing RFC first
- Existing normative RFC already specifies the change: proceed
- Spec-only artifact maintenance with no implementation: stop and use `/spec`

If you create artifacts:

- Follow `rfc-writer` or `adr-writer`
- Review drafts with the appropriate reviewer agent
- Fix critical findings before implementation

### 3. Enter implementation

Before writing code:

```bash
govctl check --has-active
```

For RFC-governed work, verify the RFC state:

- `draft/spec`: ask permission, then finalize and advance to `impl`
- `normative/spec`: ask permission, then advance to `impl`
- `normative/impl+`: proceed
- `deprecated`: stop

If implementation reveals a spec bug, do not silently deviate. Amend the RFC per [[ADR-0016]] or stop and ask.

Implementation rules:

1. Keep changes focused.
2. Follow RFC clauses and cite them in source comments when useful.
3. After each substantive change, run the relevant validation.
4. Update working memory as you go:

```bash
govctl work add <WI-ID> journal "Implemented X; govctl check passes" --scope <scope>
govctl work add <WI-ID> notes "Do not retry Y; it fails because Z"
```

### 4. Test

If an RFC exists, ask permission before `govctl rfc advance <RFC-ID> test` unless full authority was granted.

Run the relevant verification for the change:

- `govctl check`
- Project tests
- Render commands when governed output changed

If a check fails:

- Record the failed attempt in `journal`
- Record the lesson or retry rule in `notes`
- Change approach before retrying

Do not continue until green.

If the change implements, removes, or materially alters RFC-governed behavior, invoke the **compliance-checker** agent before moving to `stable`.
Treat Critical compliance findings as a release gate; fix them before continuing.

### 5. Complete

Run final validation:

```bash
govctl check
govctl render
```

If an RFC exists and all required testing is done, ask permission before `govctl rfc advance <RFC-ID> stable` unless full authority was granted.

Before advancing to `stable` for RFC-governed behavior:

1. Run **compliance-checker**
2. Fix any Critical findings
3. Re-run the relevant checks if code changed

Before closing the work item:

1. Review the work item with `wi-reviewer`
2. Tick completed acceptance criteria
3. Move the work item to `done`

Example:

```bash
govctl work show <WI-ID>
govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done
govctl work move <WI-ID> done
```

## Error Handling

### Stop and ask when

1. Requirements are ambiguous.
2. A normative RFC conflicts with the requested change.
3. The change would break existing behavior.
4. Security or secret-handling issues appear.
5. The task grows beyond the original scope.
6. The same failure recurs and you do not have a materially different next step.

### Otherwise recover and continue

| Problem                       | Recovery                                                        |
| ----------------------------- | --------------------------------------------------------------- |
| `govctl check` fails          | Read diagnostics, fix, rerun                                    |
| Tests fail                    | Debug, fix, rerun                                               |
| `work move ... done` rejected | Add or tick acceptance criteria first                           |
| Same failure repeats          | Read `notes`, then `journal`; record a new plan or stop and ask |

## Commit Conventions

- `chore(work)`: activate or complete a work item
- `docs(rfc)` / `docs(adr)`: draft governance artifacts
- `feat(scope)` / `fix(scope)` / `refactor(scope)` / `docs(scope)` / `test(scope)`: implementation commits

Use the `commit` skill for all raw VCS operations.

## Execution Checklist

- [ ] Environment validated; config read
- [ ] Active work item exists
- [ ] `govctl work show <WI-ID>` read before implementation
- [ ] `description`, `journal`, and `notes` used correctly
- [ ] Governance analysis completed or explicitly skipped
- [ ] Validation and tests passed
- [ ] Acceptance criteria ticked
- [ ] Work item closed

**BEGIN EXECUTION NOW.**
