---
name: migrate
description: "Adopt govctl in an existing project by discovering and confirming historical decisions, specifications, and active work before backfilling a governed baseline"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: '[optional scope hint, e.g. "focus on database decisions"]'
---

# Brownfield Migration

Adopt govctl incrementally in an existing codebase per [[ADR-0032]], recovering
governance from real evidence without inventing history or changing behavior.
This differs from the `govctl migrate` command, which upgrades outdated
artifact formats in an already governed project.

## Discovery

If `gov/config.toml` is missing, use `init` when the request authorizes
scaffolding; otherwise ask first. If it exists, find what earlier runs already
migrated with `govctl status`, `govctl search <topic>`, and the `rfc`, `adr`,
and `work` list commands. Use `govctl <resource> --help` for current syntax.

Read the smallest useful evidence: docs and changelogs, manifests, schemas, API
contracts, deployment config, code comments that expose durable constraints,
VCS history that shows why a choice was made, and issue trackers only for
active work. Extend an existing artifact that already owns a subject instead of
duplicating it.

## Hard Stops

- Discovery is read-only. Create artifacts or annotate source only after the
  user selects the scope.
- Ask the user before any lifecycle or destructive artifact operation (accept,
  reject, finalize, advance, bump, deprecate, supersede, delete) unless the
  request already authorizes it. Source annotation also needs approval; one
  approval may cover a stated batch.
- Edit governed files only through govctl commands, using root `govctl clause`
  for Clauses.
- Never present inferred rationale, alternatives, requirements, implementation
  or test status, or active work as fact.
- Do not make an RFC normative or advance it just because related code exists.
- Do not create Work Items that track the migration itself.
- Change product code only through approved, behavior-neutral annotations.
- Stop when evidence conflicts, a backfill would misrepresent history, or a
  transition lacks approval or evidence.

## Choosing Candidates

| Candidate        | Backfill when                                                | Never as                           |
| ---------------- | ------------------------------------------------------------ | ---------------------------------- |
| ADR              | Evidence shows a consequential choice and why it was made    | A retroactive source of obligation |
| RFC              | An existing specification or stable contract is identifiable | A dump of current behavior         |
| Work Item        | The user confirms unfinished active or queued work           | A list of every TODO or cleanup    |
| Source reference | A stable, high-signal code site relates to a recovered item  | Blanket or generated-file markup   |

Prefer decisions that are hard to reverse, cross-cutting, or repeatedly
questioned; skip tool-enforced style, incidental dependencies, and behavior
inferred only from code.

For each candidate record the evidence location, what it supports, what is
inferred, whether rationale and alternatives are recoverable, and the proposed
type and priority. Say plainly when evidence is missing: a historical ADR may
omit unrecoverable alternatives but never invents one, and RFC Clauses still
pass the `rfc-writer` contract test. Present this as a compact report and let
the user select, defer, or reject each group before any change.

## Backfill Loop

Work in small batches so partial migration stays useful:

1. Recheck governed state and the selected evidence.
2. Draft with `adr-writer`, `rfc-writer`, or `wi-writer`.
3. Review with `adr-reviewer` or `rfc-reviewer`.
4. Show uncertainties and proposed lifecycle outcomes to the user.
5. Perform only approved transitions.
6. Run `govctl check` and render affected projections.
7. Optionally record the batch with `commit`.

- **ADRs:** reconstruct context and alternatives where evidence allows and
  separate observed consequences from predictions. Accept an adopted choice as
  historical only after user confirmation; route an unresolved one to
  `discuss`.
- **RFCs:** keep traceability to the source. Each phase needs approval and
  evidence: existing implementation for `impl`, complete implementation for
  `test`, and complete implementation and tests for `stable`. Otherwise stop at
  the last defensible phase.
- **Work Items:** include scope, governing references, categorized testable
  criteria, and risk-matched guards.
- **Source references:** follow the repository's comment conventions, use
  resolvable `[[...]]` links, and never claim more conformance than the
  evidence supports.

## Resuming And Failures

On resume, rediscover existing artifacts and references, match candidates by
subject and evidence rather than title, skip completed work, keep user changes
instead of overwriting them, and report the prior partial state.

When a check, render, or review fails, keep the batch unpublished, fix the
content, and rerun the failing check. Never advance lifecycle state to make
validation pass; escalate when no authoritative recovery path exists.

## Done When

- each selected candidate is created, deferred, or rejected explicitly;
- artifacts separate recovered fact from inference, and every lifecycle state
  has evidence and approval;
- annotations resolve and no duplicates or product changes were introduced;
- `govctl check` passes and projections are current; and
- the report lists artifacts, states, annotated paths, uncertain history,
  remaining scope, and validation results.

Normal workflows (`discuss`, `spec`, `gov`, `quick`) can start after the first
coherent baseline; exhaustive backfill is not required.
