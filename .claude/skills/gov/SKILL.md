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
govctl work add <WI-ID> notes "Do not retry old fixture path; it fails because snapshots are stale"
govctl work add <WI-ID> refs RFC-0001
govctl work add <WI-ID> tags <tag>
govctl work add <WI-ID> depends_on <BLOCKING-WI-ID>
govctl tag new <tag>
govctl tag list
govctl loop list open
govctl loop start <ROOT-WI-ID> [<ROOT-WI-ID>...]
govctl loop run <LOOP-ID>
govctl loop replan <LOOP-ID>
govctl loop add <LOOP-ID> work <ROOT-WI-ID>
govctl loop remove <LOOP-ID> work <ROOT-WI-ID>
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
   - `notes`: durable learnings; record constraints, decisions, retry rules, and failure causes
8. Avoid retry cycles. If the same approach already failed, do not repeat it unchanged.
9. Spec-only governance maintenance does not belong here. Use `/spec` when no implementation work is required.
10. Work items are operational memory, not normative authority. If implementation needs a new requirement or design decision, amend the RFC or ADR instead of stuffing it into `description` or `notes`.
11. For related work, create the work item batch first, declare `depends_on` ordering, then execute it through one generated-ID loop.
12. Do not invent loop IDs. Omit `--id` when starting a loop; use the generated `LOOP-YYYY-MM-DD-NNN` ID printed by the command for later `run`, `show`, `replan`, `add`, or `remove`.

## Working Memory

The active work item is persistent working memory. Read it with `govctl work show <WI-ID>`; do not rely on raw TOML.

### Read order

1. `description` tells you the scope.
2. `notes` tells you what to remember before the next attempt.
3. `acceptance_criteria` tells you what must be true before closure.

### Write rules

- Add a `notes` entry when you learn something future steps must obey.
- Record execution trace in loop state when available.
- On failure, put durable retry rules in `notes`.

### Loop usage

For a multi-step task, cleanup run, refactor, or feature that naturally splits into related work items:

1. Create or activate the known work items before implementation starts.
2. Add `depends_on` edges for hard execution ordering.
3. Run `govctl check` so dependency cycles or missing work item IDs are caught before the loop starts.
4. Start one loop for the batch root set with `govctl loop start <ROOT-WI-ID> [<ROOT-WI-ID>...]`; let govctl generate the `LOOP-YYYY-MM-DD-NNN` ID.
5. Continue with `govctl loop run <LOOP-ID>` or `govctl loop resume <LOOP-ID>`.

When resuming after an interruption or inspecting current local execution state, run `govctl loop list open` first. Use the listed generated loop ID for `run`, `show`, `resume`, `add`, `remove`, or `replan`; do not guess a loop ID from memory.

If the scope changes during execution, keep the same loop identity:

- Use `govctl loop add <LOOP-ID> work <ROOT-WI-ID>` when newly discovered work belongs in the current batch.
- Use `govctl loop remove <LOOP-ID> work <ROOT-WI-ID>` when a root no longer belongs in the batch.
- Use `govctl loop replan <LOOP-ID>` after dependency edits that should refresh the current closure.

`work` is the editable loop work-item field. `wi` is accepted as a short alias, but examples should prefer `work`.

Do not create scattered single-item loops for work that is part of one coherent batch.

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
- No match and the task is single-slice: `govctl work new --active "<concise-title>"`
- No match and the task splits naturally: create the related work items first, wire `depends_on`, then start one generated-ID loop for the batch.

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
4. Update durable working memory as you go:

```bash
govctl work add <WI-ID> notes "Do not retry Y; it fails because Z"
```

### 4. Test

If an RFC exists, ask permission before `govctl rfc advance <RFC-ID> test` unless full authority was granted.

Run the relevant verification for the change:

- `govctl check`
- Project tests
- Render commands when governed output changed

If a check fails:

- Record the failed attempt in loop state when available
- Record the durable lesson or retry rule in `notes`
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
| Same failure repeats          | Read `notes`; record a new plan or stop and ask |

## Commit Conventions

- `chore(work)`: activate or complete a work item
- `docs(rfc)` / `docs(adr)`: draft governance artifacts
- `feat(scope)` / `fix(scope)` / `refactor(scope)` / `docs(scope)` / `test(scope)`: implementation commits

Use the `commit` skill for all raw VCS operations.

## Execution Checklist

- [ ] Environment validated; config read
- [ ] Active work item exists
- [ ] Related work was batched into one loop where applicable
- [ ] `govctl work show <WI-ID>` read before implementation
- [ ] `description` and `notes` used correctly
- [ ] Governance analysis completed or explicitly skipped
- [ ] Validation and tests passed
- [ ] Acceptance criteria ticked
- [ ] Work item closed

**BEGIN EXECUTION NOW.**
