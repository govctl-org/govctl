---
name: gov
description: "Execute governed implementation with work-item traceability, RFC/ADR authority, risk-scoped verification, and closure"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# Governed Implementation

Deliver `$ARGUMENTS` under the repository's governance model. Use `quick` for
trivial non-behavioral changes and `spec` when no code is needed.

## Discovery

```bash
govctl status
govctl work list active
govctl work list queue
govctl loop list open
govctl search <topic>
```

Read the matching Work Item and relevant RFCs and ADRs with their `show`
commands. Use `govctl <resource> --help` for current syntax.

## Hard Stops

- Stop when the request conflicts with a normative RFC or the behavior is
  unspecified. Do not implement against a draft RFC.
- Ask the user before any lifecycle or destructive artifact operation (accept,
  reject, finalize, advance, bump, deprecate, supersede, delete) unless the
  request already authorizes it.
- Do not edit `gov/` files directly; use govctl commands. Clause operations use
  the root `govctl clause` namespace.
- Substantive implementation needs a matching active Work Item. Do not create
  Work Items for mechanical substeps.

## Choose The Path

| Situation                                           | Action                                           |
| --------------------------------------------------- | ------------------------------------------------ |
| A normative RFC fully specifies the change          | Implement against it                             |
| Behavior is new, ambiguous, deprecated, or breaking | Amend the RFC first; add an ADR only for its why |
| Trivial and non-behavioral                          | Hand off to `quick`                              |
| Governance-only                                     | Hand off to `spec`                               |

One Work Item covers one durable outcome. Reuse a matching active item, activate
a queued one, or create one with `wi-writer`, choosing the narrowest guards that
cover the changed risk. Use a loop for non-trivial execution that needs round
evidence; let govctl generate its ID and reuse an open matching loop.

RFCs own obligations, ADRs own design rationale, and Work Items own execution
scope. Work Item `description` states scope and reason; `notes` hold only facts
or retry constraints still useful after closure. Progress, validation output,
and temporary blockers go in loop state or the final response. A Conformance
Case maps a scenario to RFC Clauses and Guards but is never authority for
behavior; use `govctl conformance trace` to navigate.

## RFC Lifecycle

Read the RFC's phase and the rules in [[RFC-0000:C-PHASE-LIFECYCLE]] and
[[RFC-0002:C-LIFECYCLE-VERBS]] before mutating it:

- Do not implement against draft or deprecated content.
- Content edits after `impl` need an authorized version bump before further
  phase progression.
- Stop before mutating an artifact whose lifecycle state or recovery path you
  cannot establish, including a post-`spec` RFC without a sealed baseline.
- Clause `since` is lifecycle-owned; use Clause lifecycle commands.

## Implement And Verify

Keep code within the Work Item and its governing artifacts. When work reveals a
specification defect, repair the specification through its lifecycle instead of
silently deviating.

RFCs and ADRs bind only what they state; choices they leave open belong to
implementation. When coding shows a stated low-level detail is wrong, raise it
for repair instead of building around it. When a significant design choice
emerges during coding, record its ADR once implementation has produced the
evidence.

With source scanning enabled, set its domain in `source_scan.include`.
`.gitignore` provides baseline exclusions; governance-specific exclusions and
re-inclusions go in `.govignore`. A custom `source_scan.pattern` uses capture
group 1 as the artifact ID.

Run narrow checks while developing. Do not rerun guards by hand right before
`govctl work move <WI-ID> done`, which runs them. Use `compliance-checker` when
RFC-governed behavior changes materially. After a failure, change the approach;
do not repeat the same command without new evidence.

## Done When

- the code matches governing RFCs and accepted ADRs;
- `govctl check`, focused tests, and rendered projections are current;
- no critical review finding remains;
- acceptance criteria reflect the delivered outcome and the Work Item moves to
  `done` with its guards passing; and
- the final response reports the result, validation, and residual risk.

Use `commit` for VCS operations.
