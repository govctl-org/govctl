---
name: adr-writer
description: "Write decision records that explain context, evaluated alternatives, chosen direction, and consequences without creating product obligations"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional ADR topic]"
---

# ADR Writer

Record why one design direction was chosen over alternatives, under what
constraints, and with what consequences. ADRs justify decisions; they are not
mini-RFCs or execution logs.

This helper owns ADR content quality. Use `spec` or `gov` for acceptance,
rejection, supersession, or implementation work.

## Discovery

Inspect related decisions and requirements before writing:

```bash
govctl adr show <ADR-ID>
govctl search <topic>
```

Use `govctl adr --help` and subcommand help for current creation and nested-field
editing syntax. Use `govctl adr show <ADR-ID> --history` when superseded history
matters.

## Hard Stops

- Do not accept, reject, or supersede an ADR from this helper.
- Do not create externally visible behavior, validation, lifecycle, storage, or
  compatibility obligations in ADR prose. Establish them in an RFC.
- Do not store task scope, progress, commands run, validation output, or next
  actions in an ADR.
- Do not duplicate renderer-owned headings or structured alternatives in prose.
- Stop when the governing requirement or the decision being made is unclear.

## Writing Policy

### Let The Decision Follow The Evidence

Write in this order for a new decision:

1. Describe the context, problem, constraints, and decision drivers.
2. Add credible alternatives and their material pros and cons.
3. Mark the chosen and rejected alternatives, recording why rejected options
   lost.
4. State the decision as the conclusion of that comparison.
5. Record positive, negative, and neutral consequences.

For a historical backfill, distinguish recovered evidence from inference. When
alternatives cannot be recovered, say so instead of inventing them.

The context should let a future reader understand why a decision was necessary.
The alternatives should reflect real choices rather than straw options. The
decision should be decisive and explain why the chosen option won. Consequences
must include material costs and mitigations, not only benefits.

### Preserve Artifact Authority

ADRs may reference requirements and explain how they constrain the choice, but
RFCs own product obligations. Work Items own execution scope and acceptance
criteria. Loop state and responses own transient execution evidence.

Normative keywords in quoted or referenced constraints do not transfer
authority to the ADR. Rewrite accidental RFC-style obligation lists as decision
rationale, or move the missing contract to its RFC.

Language-specific structures belong only when the concrete structure is central
to the architectural choice. Prefer stable design properties over private
implementation details.

### Respect Projection Ownership

The canonical authoring surfaces are:

| Content                                         | Owner                                 |
| ----------------------------------------------- | ------------------------------------- |
| Metadata and fixed section headings             | Renderer                              |
| Reference inventory                             | `refs`                                |
| Options, statuses, pros/cons, rejection reasons | `content.alternatives`                |
| Explanatory prose                               | `context`, `decision`, `consequences` |

Do not include renderer-generated `Context`, `Decision`, `Consequences`, or
`Alternatives Considered` headings in content fields. Do not restate structured
alternative statuses, pros, cons, or rejection reasons in a parallel prose
inventory. This boundary follows [[RFC-0000:C-ADR-PROJECTION-OWNERSHIP]].

`govctl adr show` presents the current projection and hides a superseded ADR's
body by default. `--history` restores the historical view; rendered Markdown
remains complete.

## Quality Tests

An ADR is ready when:

- context identifies the actual problem, constraints, and decision drivers;
- alternatives were evaluated before the conclusion, with at least one credible
  rejected option for a new decision;
- the decision states the chosen direction and why it prevailed;
- consequences name meaningful benefits, costs, and mitigations;
- RFC and ADR references connect the decision to its governing context;
- no section invents normative product behavior or task execution state; and
- project tags are applied when configured.

## Completion Evidence

Run `govctl check` after substantive edits and use `adr-reviewer` before
acceptance or handoff. Use `spec` for decision-only governance work and `gov`
when the accepted decision accompanies implementation.
