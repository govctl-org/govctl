---
name: rfc-writer
description: "Write precise, implementation-independent RFCs with testable normative clauses and correct lifecycle handoff"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional RFC topic]"
---

# RFC Writer

Write the product contract: observable obligations, invariants, interfaces,
and compatibility rules. An RFC is not a design diary, code sketch, or task
plan.

## Discovery

Read the RFC, relevant Clauses, and related artifacts first with
`govctl rfc show <RFC-ID>`, `govctl clause show <RFC-ID>:<CLAUSE-ID>`, and
`govctl search <topic>`. Use `govctl <resource> --help` for current syntax.
Every Clause operation uses the root `govctl clause` resource.

## Hard Stops

- Do not implement or run lifecycle verbs (finalize, bump, advance, deprecate,
  supersede, delete) here; hand them to `spec` or `gov`.
- RFCs own obligations, ADRs own design rationale, and Work Items own execution
  scope. Do not put a new obligation in an ADR or Work Item to avoid amending
  the RFC.
- Stop if the requested contract conflicts with normative content or you
  cannot tell the RFC's lifecycle state.
- Do not edit lifecycle-owned metadata such as Clause `since`, or reproduce
  headings and status metadata that `govctl render` generates.

## What Belongs In An RFC

A normative statement belongs here when an external observer, validator,
stored artifact, script, or integration can check whether it is true, and it
stays true if private types, functions, modules, or languages change. That
covers externally relevant behavior, validation and error semantics, lifecycle
and compatibility rules, and public or persisted representations. It excludes
private field layouts, language types, function signatures, helper names,
module organization, implementation steps, and validation logs.

Write normative Clauses with uppercase RFC 2119 keywords. Make each obligation
specific, independently testable, and clear about its subject and conditions;
prefer one obligation per sentence. Rationale explains why the Clause exists
without becoming a second requirements list. Use informative Clauses for scope
and overview, descriptive `C-UPPER-CASE` IDs, and `[[RFC-NNNN:C-NAME]]` links
where another RFC owns the depended-on authority.

## Specify Outcomes, Not Procedure

An RFC constrains behavior, not implementation. Pseudo-code is a defect even
when written in prose and free of language types: step sequences, internal
data flow, caching or lookup strategy, and "first compute X, then derive Y"
phrasing bind the implementation without adding anything an observer can test.
State the observable result, the invariant, or the error instead.

Constrain order only when the order itself is observable, such as output
sequence, side-effect ordering, or which error wins. Leave behavior that no
consumer depends on unspecified, or grant latitude with MAY, rather than
pinning the first plausible design. An obligation the implementation cannot
yet justify will be contradicted by the code and force an amendment.

## Lifecycle

Lifecycle rules come from [[RFC-0000:C-PHASE-LIFECYCLE]],
[[RFC-0000:C-CLAUSE-DEF]], and [[RFC-0002:C-LIFECYCLE-VERBS]]. The ones that
affect writing most:

- Draft RFCs are finalized, never version-bumped.
- A normative RFC in `spec` can be edited freely as the current candidate.
- Editing content after `impl` is an amendment that needs a version bump before
  further phase progression.
- Clause `since` is lifecycle-owned. Inherited Clauses are deprecated or
  superseded, not deleted.

## Before Handoff

- every normative statement is observable, implementation-independent, and
  testable;
- no Clause prescribes an internal procedure or ordering that an observer
  cannot detect;
- nothing duplicates renderer output or another artifact's authority;
- references and configured project tags are current;
- `govctl check` passes; and
- `rfc-reviewer` has no unresolved blocker.
