---
name: gov
description: "Execute governed implementation with work-item traceability, RFC/ADR authority, risk-scoped verification, and closure"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <what-to-do>
---

# Governed Implementation

Deliver `$ARGUMENTS` under the repository's governance model. Use this skill for
substantive implementation; use `quick` for trivial non-behavioral changes and
`spec` when no implementation is required.

## Operational Baseline

### Discovery

Establish current state before choosing a workflow:

```bash
govctl status
govctl work list active
govctl work list queue
govctl loop list open
govctl search <topic>
```

Read a matching Work Item with `govctl work show <WI-ID>` and inspect relevant
RFCs or ADRs through their `show` commands. Use `govctl <resource> --help` for
current syntax and diagnostics for recovery. In the govctl repository itself,
invoke the development binary as `cargo run --quiet --`.

### Hard Stops

- RFCs are authoritative. Stop when the requested behavior conflicts with a
  normative RFC or is materially unspecified.
- Do not implement behavior that depends on a draft RFC.
- Obtain user authorization before lifecycle-owned or destructive artifact
  operations, including acceptance/rejection, phase or version changes,
  deprecation, supersession, and deletion, unless the request already grants it.
- Do not edit files under `gov/` directly; use canonical govctl resource
  commands. Clause operations use the root `govctl clause` namespace.
- Substantive implementation needs a matching active Work Item. Do not create
  separate Work Items for mechanical substeps.
- Stop before mutation when authoritative lifecycle state or the required
  recovery path cannot be established.

## Decision Policy

### Choose The Smallest Governance Path

| Situation                                               | Action                                                                              |
| ------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Existing normative RFC fully specifies the change       | Implement against it                                                                |
| Behavior is new, ambiguous, deprecated, or incompatible | Establish the obligation in an RFC; use an ADR only for supporting design rationale |
| Change is trivial and non-behavioral                    | Hand off to `quick`                                                                 |
| Work is governance-only                                 | Hand off to `spec`                                                                  |

Use one Work Item for one durable outcome. Reuse a matching active item, activate
a queued item, or create one only when the result deserves durable tracking.
Use `wi-writer` for field quality and choose the narrowest guards that cover the
changed risk. Project defaults are for checks required by every Work Item.

Use a loop when non-trivial execution needs local round evidence. Let govctl
generate the loop ID, reuse an open matching loop, and keep transient progress in
loop state rather than Work Item notes.

### Preserve Artifact Authority

- RFC: obligations and externally relevant invariants.
- ADR: design choice, rationale, and consequences.
- Work Item: task scope, acceptance criteria, dependencies, and durable
  execution facts.
- Conformance Case: a derived, non-normative mapping from a project scenario to
  versioned RFC Clauses and reusable Guards.

Work Item `description` states scope and reason. `notes` hold only facts or retry
constraints that remain useful after closure. Progress, validation output,
plans, and temporary blockers belong in loop evidence or the final response.
Use `govctl conformance trace` for requirement-to-scenario navigation. Never
treat a Case as authority for behavior that is absent from its RFC requirements.

### Respect RFC Lifecycle Boundaries

Inspect the current RFC with `show` and read its governing lifecycle clauses
before mutation. Do not implement against draft or deprecated content, progress
an amended sealed version without its authorized bump, or mutate a post-`spec`
RFC whose sealed baseline cannot be established. Clause `since` is
lifecycle-owned; use canonical Clause lifecycle commands rather than editing
history.

### Implement And Verify

Keep implementation scoped to the Work Item and governing artifacts. When work
reveals a specification defect, repair the specification through the authorized
lifecycle rather than silently deviating.

Run the narrowest useful checks while developing. Before closing the Work Item,
do not manually repeat guards that `govctl work move <WI-ID> done` is about to
run. Standalone verification is for diagnosis or evidence while the item remains
active. Use `compliance-checker` when RFC-governed behavior changes materially.

Recover from diagnostics by changing the failing assumption or approach. Do not
repeat the same failed command without new evidence.

## Completion Evidence

The task is complete when:

- implementation matches the governing RFCs and accepted ADRs;
- `govctl check` passes for the governed repository;
- relevant focused tests and generated projections are current;
- semantic review has no unresolved critical finding;
- Work Item acceptance criteria reflect the delivered outcome;
- moving the Work Item to `done` passes its effective guards; and
- the final response reports the result, validation, and any residual risk.

Use the `commit` skill for raw VCS operations.
