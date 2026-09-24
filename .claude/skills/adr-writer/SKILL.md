---
name: adr-writer
description: "Write decision records that explain context, evaluated alternatives, chosen direction, and consequences without creating product obligations"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional ADR topic]"
---

# ADR Writer

Record why one design direction was chosen over alternatives, under what
constraints, and with what consequences. An ADR is not a mini-RFC or an
execution log.

## Discovery

Read related decisions and requirements first with `govctl adr show <ADR-ID>`
and `govctl search <topic>`; add `--history` only when superseded history
matters. Use `govctl <resource> --help` for current syntax.

## Hard Stops

- Do not accept, reject, or supersede an ADR here; hand that to `spec` or `gov`.
- RFCs own obligations, ADRs own design rationale, and Work Items own execution
  scope. Do not create behavior, validation, lifecycle, storage, or
  compatibility obligations in ADR prose, and do not record task scope,
  progress, commands, or validation output.
- Stop if the governing requirement or the decision itself is unclear.

## Writing Order

1. Context: the problem, constraints, and decision drivers.
2. Alternatives actually considered, with only the pros and cons that
   influenced the outcome. Never pad to a quota: one real rejected option beats
   three fabricated ones, and when only one viable direction exists, say so in
   a sentence.
3. Mark chosen and rejected alternatives and why each rejected one lost.
4. Decision: commit clearly to what is decided, as the conclusion of that
   comparison.
5. Consequences: benefits, costs, and mitigations, not only benefits.

For a historical backfill, separate recovered evidence from inference and say
so when alternatives cannot be recovered.

## Decide Only What Must Be Decided Now

An ADR written before implementation has little evidence about low-level
design, yet implementing agents treat everything it states as settled. Record
the direction, boundaries, and constraints that are costly to reverse or that
other work depends on. For each sub-choice, ask whether deciding it during
coding would lose anything; if not, leave it out, or name it as deferred to
implementation with any constraint it must respect.

Data layouts, helper decomposition, algorithms, error-handling mechanics, and
step-by-step procedures rarely belong in an ADR written before code exists.
Record such a choice after implementation has produced the evidence.
Language-specific structure belongs only when it is central to the decision.

## Keep It Short

A typical ADR is a few dozen lines. Cut background the reader already has,
alternatives nobody seriously considered, and pros and cons that did not
matter. Bloat is a defect: it hides the decision.

Normative keywords quoted from an RFC do not make the ADR authoritative.
Rewrite accidental obligation lists as rationale, or move the missing contract
to its RFC.

## Projection Ownership

Per [[RFC-0000:C-ADR-PROJECTION-OWNERSHIP]]:

| Content                                         | Owner                                 |
| ----------------------------------------------- | ------------------------------------- |
| Metadata and fixed section headings             | Renderer                              |
| Reference inventory                             | `refs`                                |
| Options, statuses, pros/cons, rejection reasons | `content.alternatives`                |
| Explanatory prose                               | `context`, `decision`, `consequences` |

Do not write `Context`, `Decision`, `Consequences`, or `Alternatives
Considered` headings into content fields, and do not restate alternatives'
statuses, pros, cons, or rejection reasons in prose.

## Before Handoff

- every alternative was really considered, and an uncontested decision says so;
- the decision leaves choices that coding can settle better to implementation;
- consequences include real costs;
- references connect the decision to its governing RFCs, and configured project
  tags are applied;
- `govctl check` passes; and
- `adr-reviewer` has no unresolved critical finding.
