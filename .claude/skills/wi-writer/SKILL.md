---
name: wi-writer
description: "Write durable work items with scoped descriptions, testable categorized acceptance criteria, governing references, and risk-matched guards"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional work-item topic]"
---

# Work Item Writer

A Work Item records one durable execution outcome and how to tell it is done.
It is not a specification, a design decision, or a progress journal. This
helper owns field quality; the invoking workflow owns code, lifecycle
transitions, loops, and VCS operations.

## Discovery

```bash
govctl work list active
govctl work list queue
govctl work show <WI-ID>
govctl guard list
govctl loop list open
```

Use `govctl <resource> --help` for current syntax.

## Hard Stops

- RFCs own obligations, ADRs own design rationale, and Work Items own execution
  scope. New behavior, validation, storage, compatibility, or lifecycle rules
  need an RFC; a design choice needs an ADR, not an explanation in the item.
- Never put progress, commands run, validation output, review status, temporary
  blockers, hypotheses, or next actions in `description` or `notes`.
- Do not split one outcome into items for mechanical substeps.
- Do not delete a Work Item without explicit user authorization; hand deletion
  to the invoking workflow.
- Stop when no governing authority exists to define the acceptance criteria.

## Fields

### Description

One concise paragraph: what the task accomplishes, why, and its scope. Do not
restate contract language or track progress.

Describe what exists today by its role. Do not name types, functions, fields,
or files that do not exist yet: a reader cannot resolve a forward reference,
and the implementing agent owns those names. Introduce a planned concept by its
role ("a per-clone coordination registry"), not its planned identifier.

### Acceptance Criteria

Each criterion is independently testable, describes an observable outcome, and
carries a changelog category: `add`, `fix`, `change`, `remove`, `deprecate`,
`security`, or `chore`. Use `chore` for internal outcomes that stay out of the
release changelog. Do not prescribe incidental private structure or planned
symbol names.

To correct a criterion, use
`govctl work edit <ID> "acceptance_criteria[N]" --set <value>`. A recognized
category prefix updates text and category; other input updates only text;
checklist status is kept. Use the `.text` child path to keep a recognized
prefix as literal text, and `--tick` to change status. Quote every path with
brackets. Use `--stdin` for text containing backticks, `$()`, or other shell
syntax.

### Notes

Only closure-worthy constraints, durable implementation facts, or reasons an
approach should not be retried. A `cancelled` item records its reason here.
Notes cannot override an RFC or accepted ADR. Transient state belongs in loop
rounds or the final response; progress belongs in criterion status.

### References And Dependencies

Reference the RFCs that authorize behavior and the ADRs that constrain the
approach. Use `depends_on` only for hard execution order; informational links
go in `refs`.

Create a separate item only for an independently reviewable outcome. Helper
extraction, fixtures, file moves, and formatting stay inside the parent. Use a
multi-item loop only when the batch has several durable outcomes.

## Guards

- Project `default_guards` are the intersection of checks every Work Item
  needs.
- Work Item `required_guards` add reusable checks for this item's risk
  domains, chosen from the changed surface, references, and criteria.
- Prefer the narrowest guard that proves the behavior; use full suites only
  for cross-cutting changes.
- Run one-off diagnostics directly instead of turning them into guards.
- Do not add a criterion that just repeats an effective guard's command
  success. A short `chore` criterion may summarize validation not covered by
  guards.

Use `guard-writer` when a reusable check is missing.

## Checklist

- The title is short and action-oriented; the item is one durable outcome.
- The description is scope, not contract, rationale, or progress.
- Every criterion is categorized and testable; tick it only when its outcome
  exists.
- References give authority for user-visible behavior; dependencies are real
  ordering.
- Guards match the item's risk without inflating defaults.
- `govctl check` passes after substantive edits.

The invoking workflow moves the item to `done` once criteria are complete;
`govctl work move <WI-ID> done` runs the effective guards, so do not rerun them
just before.
