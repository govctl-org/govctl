---
name: spec
description: "Maintain RFC and ADR artifacts without implementation work"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <artifact-maintenance-task>
---

# Specification Maintenance

Maintain governance artifacts for `$ARGUMENTS` without writing code or creating
execution work.

## Discovery

```bash
govctl status
govctl search <topic>
govctl rfc list
govctl adr list
```

Read artifacts with `show`; add `--history` only when obsolete content matters.
Use `govctl <resource> --help` for current syntax.

## Hard Stops

- Do not write code, create Work Items, or advance an RFC beyond `spec`.
- RFCs own obligations, ADRs own design rationale, and Work Items own execution
  scope. Keep implementation plans out of RFCs and ADRs.
- Do not edit `gov/` files directly; use govctl commands. Clause operations use
  the root `govctl clause` namespace.
- Ask the user before any lifecycle or destructive artifact operation (accept,
  reject, finalize, advance, bump, deprecate, supersede, delete) unless the
  request already authorizes it.
- Stop a lifecycle mutation when you cannot establish the phase, sealed
  baseline, or recovery path it depends on.
- Hand off to `discuss` or `gov` when a "clarification" changes behavior, the
  design is unresolved, or code is required.

## Classify The Change

| Change                                     | Path                                   |
| ------------------------------------------ | -------------------------------------- |
| Clarify an obligation, behavior unchanged  | Edit and review the RFC                |
| Add, change, deprecate, or remove behavior | Amend the RFC, then hand code to `gov` |
| Refine rationale or alternatives           | Edit and review the ADR                |
| Open design question                       | Hand off to `discuss`                  |
| Metadata or reference fix                  | Edit the owning artifact               |

Follow `rfc-writer` or `adr-writer`. Before a lifecycle transition on
substantively changed content, run `rfc-reviewer`, `adr-reviewer`, or
`wi-reviewer` as appropriate.

## RFC Lifecycle

Read the RFC's phase and the rules in [[RFC-0000:C-PHASE-LIFECYCLE]] and
[[RFC-0002:C-LIFECYCLE-VERBS]] before editing:

- A draft RFC is finalized, never bumped.
- A normative RFC in `spec` is still its open candidate; do not bump it just to
  retarget that candidate.
- Content edits after `spec` need an authorized version bump before further
  phase progression.
- A post-`spec` RFC with no trustworthy sealed baseline needs migration or a
  restore from version control, not an inferred bump.
- Do not edit or bump deprecated RFCs.
- Clause `since` is lifecycle-owned; use Clause lifecycle commands.
  Current-version changelog corrections use the changelog edit path and do not
  replace an amendment or bump.

## Done When

- each artifact stays within its authority and its lifecycle state is explicit;
- its reviewer has no unresolved critical finding;
- `govctl check` passes and affected projections are rendered; and
- the final response names changed artifacts, lifecycle state, review result,
  and the next workflow: `discuss` for open design, `gov` for code, `quick` for
  unrelated non-behavioral cleanup, or `commit` for VCS.

Leave unapproved artifacts in their current draft or proposed state.
