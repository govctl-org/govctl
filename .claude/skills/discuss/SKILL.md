---
name: discuss
description: "Explore a governance design, resolve ambiguity, and draft RFC or ADR artifacts without implementation"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <topic-or-question>
---

# Design Discussion

Understand `$ARGUMENTS`, relate it to existing governance, and produce only the
design artifacts justified by the discussion.

## Operational Baseline

### Discovery

Start from repository context rather than a blank design:

```bash
govctl status
govctl search <topic>
govctl rfc list
govctl adr list
```

Read relevant current artifacts with `show`; use `--history` only when prior
content matters. Use `govctl <resource> --help` for current authoring syntax. In
the govctl repository itself, invoke the development binary as
`cargo run --quiet --`.

### Hard Stops

- This is a design workflow: do not implement code or create Work Items.
- Do not perform lifecycle or destructive artifact mutations here, including
  acceptance/rejection, phase or version changes, deprecation, supersession,
  and deletion.
- Do not issue raw VCS commands; use the `commit` skill when drafts should be
  recorded.
- RFCs own obligations, ADRs own design rationale, and Work Items own execution.
  Do not use one artifact to compensate for missing content in another.
- Stop and ask when requirements conflict, a breaking consequence is
  unacknowledged, or the available evidence cannot resolve a material ambiguity.

## Decision Policy

### Classify The Outcome

| Question answered                            | Result                               |
| -------------------------------------------- | ------------------------------------ |
| What behavior or invariant must be true?     | RFC or RFC amendment                 |
| Why was one design chosen over alternatives? | ADR                                  |
| What does an existing artifact mean?         | Discussion or clarification          |
| What work should be executed now?            | Hand off to `gov`; no Work Item here |

Not every discussion needs an artifact. Prefer clarification in conversation
when no durable obligation or decision changes.

### Explore Before Concluding

Identify constraints from existing RFCs and ADRs, then compare plausible
options. Ask only questions whose answers materially change the design. For
high-risk or difficult trade-offs, use `decision-analysis`.

When drafting:

- use `rfc-writer` for normative Clause quality;
- use `adr-writer` and establish alternatives before the decision;
- use root `govctl clause` commands for every Clause operation;
- use `[[artifact-id]]` references in governed prose; and
- keep implementation details out unless they are an external contract.

Draft lifecycle operations belong to the later `spec` or `gov` handoff. A
behavior-changing amendment needs governed implementation; a clarification with
no implementation can use `spec`.

### Review And Validate

Run `govctl check` after substantive artifact edits. Use `rfc-reviewer` for an
RFC draft, `adr-reviewer` for an ADR draft, and `wi-reviewer` for a Work Item
when independent semantic review is warranted. Resolve critical findings before
presenting the artifact as ready. Reviewer isolation is for semantic quality;
govctl remains responsible for structural validation.

## Completion Evidence

Conclude with:

- the problem and constraints understood;
- options considered and the current recommendation;
- draft artifacts created or changed, with status;
- unresolved questions and material risks;
- validation and reviewer results; and
- the correct handoff: continue discussion, `spec`, `gov`, or `commit`.

Leave artifacts in draft/proposed state until the user authorizes the owning
lifecycle transition.
