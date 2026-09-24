---
name: quick
description: "Execute a trivial non-behavioral change without unnecessary governance ceremony"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# Quick Change

Complete `$ARGUMENTS` only while it stays small, local, and non-behavioral.

Run `govctl status`. If the task names a Work Item, read it with
`govctl work show <WI-ID>`; check `govctl work list active` and `queue` only
when an existing item may already own the cleanup. Use
`govctl <resource> --help` for current syntax.

## Hard Stops

- Leave this workflow when the change affects behavior, edits a governance
  artifact, exposes an architectural choice, or depends on an ambiguous
  requirement.
- Do not create a Work Item for a typo, comment, formatting fix, or mechanical
  cleanup.
- Do not perform artifact lifecycle or destructive operations.
- Do not issue raw VCS commands; use `commit`.
- Stop when the scope or authority is no longer clear.

## Routing

| Situation                                                | Action                           |
| -------------------------------------------------------- | -------------------------------- |
| Typo, comment, small guide fix, local mechanical cleanup | Continue with `quick`            |
| A matching Work Item already owns the cleanup            | Keep its scope and closure rules |
| The user needs durable tracking                          | Use at most one coarse Work Item |
| Governance artifact maintenance only                     | Hand off to `spec`               |
| Open design or requirement ambiguity                     | Hand off to `discuss`            |
| Behavior, implementation, or broader verification        | Hand off to `gov`                |

Activate a queued Work Item only when it exactly owns this trivial cleanup;
otherwise hand off to `gov`. Never implement under or close a queued item. Keep
criteria observable and notes durable; progress goes in an existing loop or the
final response. Do not create a loop for an untracked trivial change.

Make the smallest coherent edit. `govctl check` must pass; run focused checks
when code or generated output changes, and switch to `gov` if they reveal
behavioral risk. When closing a tracked item, let `govctl work move <WI-ID>
done` run its guards instead of rerunning them by hand first.

## Done When

- the change is still non-behavioral and within scope;
- focused checks and `govctl check` pass;
- any Work Item reflects the result and passes closure; and
- the final response reports the edit and validation.

Use `commit` to record the change.
