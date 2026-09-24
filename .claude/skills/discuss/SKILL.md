---
name: discuss
description: "Explore a governance design, resolve ambiguity, and draft RFC or ADR artifacts without implementation"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <topic-or-question>
---

# Design Discussion

Understand `$ARGUMENTS`, relate it to existing governance, and draft only the
artifacts the discussion justifies.

## Discovery

```bash
govctl status
govctl search <topic>
govctl rfc list
govctl adr list
```

Read relevant artifacts with `show`; add `--history` only when prior content
matters. Use `govctl <resource> --help` for current syntax.

## Hard Stops

- Do not write code or create Work Items.
- Do not perform lifecycle or destructive operations (accept, reject, finalize,
  advance, bump, deprecate, supersede, delete); they belong to `spec` or `gov`.
- Do not issue raw VCS commands; use `commit`.
- RFCs own obligations, ADRs own design rationale, and Work Items own execution
  scope. Do not use one to fill gaps in another.
- Ask when requirements conflict, a breaking consequence is unacknowledged, or
  evidence cannot resolve a material ambiguity.

## Classify The Outcome

| Question answered                        | Result                               |
| ---------------------------------------- | ------------------------------------ |
| What behavior or invariant must be true? | RFC or RFC amendment                 |
| Why was one design chosen over others?   | ADR                                  |
| What does an existing artifact mean?     | Clarify in conversation              |
| What work should be done now?            | Hand off to `gov`; no Work Item here |

Not every discussion needs an artifact.

## Explore And Draft

Start from constraints in existing RFCs and ADRs, then compare plausible
options. Ask only questions whose answers change the design. Use
`decision-analysis` for high-risk trade-offs.

When drafting:

- follow `rfc-writer` for Clauses and `adr-writer` for decisions, with
  alternatives before the conclusion;
- use root `govctl clause` commands and `[[artifact-id]]` references;
- leave out implementation details that are not an external contract; and
- separate choices that must be settled now from those coding can settle
  better, and leave the latter out of RFC and ADR drafts.

Run `govctl check` after edits and the matching reviewer (`rfc-reviewer`,
`adr-reviewer`) before calling a draft ready. Resolve critical findings first.

## Done When

The final response covers:

- the problem, constraints, options, and recommendation;
- drafts created or changed, with their status;
- open questions, risks, and validation and review results; and
- the handoff: keep discussing, `spec` for clarifications, `gov` for
  behavior-changing amendments, or `commit`.

Leave artifacts in draft or proposed state until the user authorizes a
transition.
