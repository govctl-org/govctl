---
name: spec
description: "Maintain RFC and ADR artifacts without implementation work"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <artifact-maintenance-task>
---

# Specification Maintenance

Maintain governance artifacts for `$ARGUMENTS` without implementing code or
creating execution work.

## Operational Baseline

### Discovery

Establish artifact and lifecycle state first:

```bash
govctl status
govctl search <topic>
govctl rfc list
govctl adr list
```

Read the current projection with the resource `show` command and use `--history`
only when obsolete content matters. Use `govctl <resource> --help` for current
authoring and lifecycle syntax. In the govctl repository itself, invoke the
development binary as `cargo run --quiet --`.

### Hard Stops

- This workflow is artifact-only. Do not write implementation code, create Work
  Items, or advance an RFC beyond `spec`.
- RFCs own obligations; ADRs own design rationale. Do not put implementation
  plans into either artifact.
- Use canonical govctl resource commands rather than editing `gov/` files.
  Clause operations use the root `govctl clause` namespace.
- Obtain user authorization before lifecycle-owned or destructive artifact
  operations, including acceptance/rejection, finalization, version changes,
  deprecation, supersession, and deletion, unless already granted.
- Stop when a clarification changes behavior, design remains unresolved, or the
  task requires implementation. Route those cases to `discuss` or `gov`.
- Stop lifecycle mutation when authoritative phase, signature baseline, or
  required recovery state cannot be established.

## Decision Policy

### Classify The Change

| Change                                          | Path                                             |
| ----------------------------------------------- | ------------------------------------------------ |
| Clarify an obligation without changing behavior | Edit and review the RFC                          |
| Change, add, deprecate, or remove behavior      | Amend the RFC, then hand implementation to `gov` |
| Refine rationale or alternatives                | Edit and review the ADR                          |
| Resolve an open design question                 | Hand off to `discuss`                            |
| Fix governance metadata or references           | Edit the owning artifact                         |

Follow `rfc-writer` or `adr-writer` for artifact quality. Use the independent
matching reviewer before treating content as ready for a lifecycle transition.

### Respect Candidate Boundaries

Inspect the RFC and its governing lifecycle clauses before editing:

- Draft RFC content remains in its initial candidate and is finalized rather
  than version-bumped.
- A normative RFC already in `spec` remains open for current-candidate
  authoring; do not bump merely to retarget that candidate.
- Editing sealed content after `spec` creates an amendment that needs an
  authorized version-changing bump before later phase progression.
- A post-`spec` RFC without a trustworthy sealed baseline requires the
  documented migration or version-control restoration path, not an inferred
  bump.
- Deprecated RFC content is historical and is not edited or version-bumped.

Clause `since` and version assignment are lifecycle-owned. Use the Clause
lifecycle surface rather than rewriting history. Current-version changelog
corrections use the canonical changelog edit path and do not replace a content
amendment or lifecycle bump.

### Validate And Hand Off

Run `govctl check` after substantive artifact edits and render affected
projections. Resolve structural diagnostics and critical reviewer findings
before requesting a lifecycle transition.

Use:

- `discuss` when the decision remains open;
- `gov` when code or implementation tests are required;
- `quick` only for unrelated non-behavioral cleanup outside governance
  artifacts; and
- `commit` for raw VCS operations.

## Completion Evidence

Spec maintenance is complete when:

- the artifact stays within its authority boundary;
- lifecycle state and required authorization are explicit;
- the matching reviewer has no unresolved critical finding;
- `govctl check` passes and affected projections are current; and
- the final response identifies changed artifacts, lifecycle state, review
  result, and the correct next workflow.

Leave unapproved artifacts in their existing draft/proposed lifecycle state.
