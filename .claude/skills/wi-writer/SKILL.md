---
name: wi-writer
description: "Write durable work items with scoped descriptions, testable categorized acceptance criteria, governing references, and risk-matched guards"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional work-item topic]"
---

# Work Item Writer

Define durable execution scope and completion evidence. Work Items are
operational records, not normative specifications, design decisions, or
round-by-round journals.

This helper owns Work Item field quality. The invoking workflow owns code,
lifecycle transitions, loop execution, and VCS operations.

## Discovery

Before creating or editing an item, inspect existing work and reusable guards:

```bash
govctl work list active
govctl work list queue
govctl work show <WI-ID>
govctl guard list
govctl loop list open
```

Use `govctl work --help`, `govctl guard --help`, and subcommand help for current
syntax and nested-field editing.

## Hard Stops

- Do not introduce product obligations in Work Item fields. New behavior,
  validation, storage, compatibility, or lifecycle rules need governing RFC
  authority.
- Do not use an ADR-like explanation as a substitute for recording a design
  choice in an ADR.
- Do not put progress, commands run, validation output, review status, temporary
  blockers, hypotheses, or next actions in `description` or `notes`.
- Do not split a coherent outcome into Work Items for mechanical substeps.
- Do not permanently delete a Work Item without explicit user authorization;
  hand the destructive operation to the invoking workflow.
- Stop when the task lacks the governing authority needed to define its
  acceptance criteria.

## Field Policy

### Description

State what the task will accomplish, why it is needed, and the relevant scope in
one concise paragraph. It describes the execution target without restating
normative contract language or tracking progress.

### Acceptance Criteria

Each criterion must be independently testable and use a changelog category:
`add`, `fix`, `change`, `remove`, `deprecate`, `security`, or `chore`.

Criteria describe observable task outcomes. They should be specific enough to
decide done/not-done without prescribing incidental private structure. Use
`chore` for internal validation or documentation outcomes that should not enter
the release changelog.

Correct an existing criterion with
`govctl work edit <ID> "acceptance_criteria[N]" --set <value>`. A recognized
category prefix updates both text and category; other input updates only text.
The operation preserves checklist status. Use the `.text` child path when a
recognized prefix must remain literal text, and use `--tick` for status changes.
Quote every path containing brackets. Use `--stdin` for rich text containing
backticks, `$()`, or other shell syntax.

### Notes

Use notes sparingly for closure-worthy constraints, durable implementation facts,
or reasons an approach should not be retried. A note should remain useful after
the item is done. A Work Item moved to `cancelled` records its cancellation
reason here. Notes cannot override an RFC or accepted ADR.

Transient execution state belongs in loop rounds or the final response.
Acceptance progress belongs in criterion status.

### References And Dependencies

Reference RFCs that authorize behavior and ADRs that constrain the approach.
Use `depends_on` only for a hard execution ordering dependency. Informational
relationships belong in `refs`.

Create separate Work Items only for independently meaningful, reviewable
outcomes. Keep helper extraction, fixtures, file moves, formatting, and similar
mechanical work inside the parent item. Use a multi-item loop only when the batch
contains multiple durable outcomes.

## Verification Policy

Select guards from the changed surface, governing references, and acceptance
criteria:

- Project `default_guards` are the intersection of checks needed by every Work
  Item.
- Work Item `required_guards` add reusable checks for this task's risk domains.
- Prefer the narrowest guard that proves the relevant behavior.
- Use full test or lint suites for cross-cutting changes or when narrower checks
  cannot cover the blast radius.
- Run one-off diagnostic commands directly rather than turning them into
  completion guards.

Do not duplicate an effective guard as a plain command-success criterion merely
to run it twice. A concise `chore` criterion may summarize validation outcomes
that are not fully represented by guards.

Use `guard-writer` when a stable reusable check is missing.

## Quality Tests

A Work Item is well formed when:

- its title is concise and action-oriented;
- its description is task scope rather than contract, rationale, or progress;
- every criterion is categorized, specific, and testable;
- references provide authority for user-visible behavior;
- dependencies represent real execution ordering;
- notes contain only durable post-closure context;
- selected guards match the item's risk without broad default-suite inflation;
  and
- the item represents one durable outcome rather than a mechanical fragment.

## Completion Evidence

Tick criteria only when their outcomes exist. The invoking workflow may move the
item to `done` after all criteria are complete and effective guards pass. Do not
manually rerun those same guards immediately before the transition.

Run `govctl check` after substantive artifact edits. Use loop state for
non-trivial execution memory and the `commit` skill for VCS operations.
