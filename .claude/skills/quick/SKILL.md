---
name: quick
description: "Execute a trivial non-behavioral change without unnecessary governance ceremony"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# Quick Change

Complete `$ARGUMENTS` only while it remains small, local, and non-behavioral.

## Operational Baseline

### Discovery

Use `govctl status` to confirm repository context. If task context names a Work
Item, inspect it with `govctl work show <WI-ID>`; otherwise check
`govctl work list active` and `govctl work list queue` only when an existing
tracked task may already own the cleanup. Use resource `--help` for current
syntax. In the govctl repository itself, invoke governance commands as
`cargo run --quiet --`.

### Hard Stops

- Stop using this workflow when the change affects behavior, changes a
  governance artifact, exposes an architectural choice, or depends on an
  ambiguous requirement.
- Do not create a Work Item merely to record a typo, comment, formatting fix,
  or mechanical cleanup.
- Do not perform artifact lifecycle or destructive operations.
- Do not issue raw VCS commands; use the `commit` skill.
- Stop before mutation when the task's scope or authority is no longer clear.

## Decision Policy

| Situation                                                     | Action                               |
| ------------------------------------------------------------- | ------------------------------------ |
| Typo, comment, small guide fix, or local mechanical cleanup   | Continue with `quick`                |
| Existing matching Work Item already owns the cleanup          | Preserve its scope and closure rules |
| User explicitly needs durable tracking for the outcome        | Use at most one coarse Work Item     |
| Governance artifact maintenance with no implementation        | Hand off to `spec`                   |
| Open design or requirement ambiguity                          | Hand off to `discuss`                |
| Behavior, implementation, or broader verification is involved | Hand off to `gov`                    |

Activate a matching queued Work Item through its canonical lifecycle command
only when it exactly owns the cleanup and the task remains trivial. Otherwise
hand off to `gov`. Never implement under or attempt to close a queued item.

When a matching Work Item exists, keep acceptance criteria observable and notes
durable. Transient progress belongs in an existing loop or the final response,
not Work Item notes. Do not create a new loop for a trivial untracked change.

Make the smallest coherent edit and run validation proportional to the affected
surface. `govctl check` must pass in a governed repository. Run focused project
checks when code or generated output is touched; if those checks reveal broader
behavioral risk, switch to `gov`.

When closing a tracked Work Item, let `govctl work move <WI-ID> done` execute its
effective guards instead of manually repeating them immediately beforehand.

## Completion Evidence

The quick change is complete when:

- it remains demonstrably non-behavioral and within the original scope;
- affected focused checks and `govctl check` pass;
- any existing Work Item accurately reflects the result and passes closure; and
- the final response reports the edit and validation.

Use `commit` to record the coherent change.
