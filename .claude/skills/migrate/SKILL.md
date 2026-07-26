---
name: migrate
description: "Adopt govctl in an existing project by discovering and confirming historical decisions, specifications, and active work before backfilling a governed baseline"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: '[optional scope hint, e.g. "focus on database decisions"]'
---

# Brownfield Migration

Adopt govctl incrementally in an existing codebase according to [[ADR-0032]].
Recover durable governance from available evidence without rewriting history,
inventing rationale, or changing product behavior.

This is a historical backfill workflow. Use `gov` for implementation,
`discuss` for unresolved design, `init` for installation and scaffolding policy,
and `commit` for VCS operations.

## Operational Baseline

### Discovery

Check for `gov/config.toml` before running resource discovery. Its absence means
the repository is not initialized; use the `init` skill when the adoption
request authorizes scaffolding, or confirm that step with the user first.

When the config exists, establish what was previously migrated:

```bash
govctl status
govctl search <topic>
govctl adr list
govctl rfc list
govctl work list active
govctl work list queue
```

If existing governance instead reports an outdated artifact or schema format,
use the deterministic `govctl migrate` command and its diagnostics. That format
upgrade is distinct from this brownfield adoption skill.

Use resource and subcommand `--help` for current creation, editing, lifecycle,
and rendering syntax. Never assume a rerun starts from an empty governance
directory.

Read the smallest useful set of repository evidence:

- project, architecture, contribution, and changelog documentation;
- manifests, dependency declarations, schemas, API contracts, and deployment
  configuration;
- code structure and comments that expose durable constraints;
- VCS history when it can recover why or when a choice was made; and
- issue or planning systems only when accessible and relevant to active work.

Search existing governed artifacts before proposing a candidate. Reuse,
cross-reference, or extend an existing artifact when it already owns the
subject.

### Hard Stops

- Discovery is read-only. Present candidates and obtain user-selected scope
  before creating artifacts or annotating source.
- Obtain explicit authorization before any lifecycle mutation, artifact
  deletion, or source annotation. This includes acceptance or rejection,
  finalization, phase or version changes, deprecation, and supersession. A clear
  approval may cover a stated batch.
- Use canonical govctl resource commands for governed files. Clause operations
  use the root `govctl clause` namespace.
- Do not infer undocumented rationale, rejected alternatives, requirements,
  implementation status, test status, or active work as fact.
- Do not make a reconstructed RFC normative or advance it merely because
  related code exists.
- Do not create migration-tracking Work Items in the target project. Backfill
  Work Items only for confirmed work already in progress.
- Do not change product code except for explicitly authorized reference
  annotations, and keep those annotations behavior-neutral.
- Stop when evidence conflicts, the requested backfill would misrepresent
  history, or a lifecycle transition lacks authorization or supporting evidence.

## Decision Policy

### Select Durable Candidates

Backfill only information whose future value justifies governance:

| Candidate        | Use when                                                                                     |
| ---------------- | -------------------------------------------------------------------------------------------- |
| ADR              | Evidence shows a consequential choice, its constraints, and why the chosen direction matters |
| RFC              | An existing specification or stable contract can be recovered without inventing obligations  |
| Work Item        | Evidence and user confirmation identify unfinished work that is currently active or queued   |
| Source reference | A high-signal implementation site clearly relates to a recovered artifact                    |

Prefer decisions that are hard to reverse, cross-cutting, or repeatedly
questioned. Tool-enforced style choices, incidental dependencies, TODOs without
ownership, and behavior inferred only from implementation usually do not
deserve backfill.

An ADR explains a historical choice; it does not retroactively create product
requirements. An RFC records an existing contract; it is not a dump of current
implementation behavior. A Work Item records unfinished execution; it is not a
list of every discovered cleanup opportunity.

### Preserve Evidence Quality

For every candidate, retain:

- the evidence location;
- what is directly supported;
- what remains uncertain or inferred;
- whether alternatives and rationale are recoverable; and
- the proposed artifact type and migration priority.

State missing evidence plainly. Historical ADRs may omit unrecoverable
alternatives when their context says so. Do not manufacture a rejected option
to satisfy a template. RFC clauses must still be observable,
implementation-independent, and testable; use `rfc-writer` for that quality
test.

Present a compact discovery report before mutation. Include the candidate,
artifact type, evidence, confidence or uncertainty, likely duplicates, and a
recommended scope. Let the user select, defer, or reject each group.

## Backfill Loop

Work in small coherent batches so partial migration remains useful:

1. Reinspect current governed state and selected evidence.
2. Draft the selected artifacts with `adr-writer`, `rfc-writer`, or `wi-writer`.
3. Review drafts independently before lifecycle publication.
4. Show uncertainties and proposed lifecycle outcomes to the user.
5. Perform only the authorized transitions.
6. Run `govctl check` and render the affected projections.
7. Optionally record the coherent milestone through `commit`.

### Historical ADRs

Reconstruct context and alternatives before the decision where evidence allows.
Distinguish observed consequences from predictions. Use `adr-reviewer` before
acceptance.

An already-adopted choice may be accepted as historical only after user
confirmation. If the choice is unresolved or being reconsidered, leave it
proposed and route the discussion through `discuss`.

### Existing Specifications

Backfill RFCs only from identifiable specifications or confirmed contracts.
Preserve traceability to the source material and use first-class Clause
resources. Use `rfc-reviewer` before finalization.

Finalization and each phase progression require user authorization and evidence
appropriate to the target state. Confirmed existing implementation can support
entry to `impl`; evidence that implementation is complete can support `test`;
`stable` requires evidence that both implementation and relevant tests are
complete. Missing evidence leaves the RFC at the last defensible phase rather
than filling the lifecycle optimistically.

### Active Work

Create Work Items only for user-confirmed unfinished work. Record task scope,
governing references, testable categorized acceptance criteria, and
risk-matched guards. Do not convert every issue, TODO, or branch into a Work
Item automatically.

### Source References

Source annotation is optional and modifies existing files. Obtain authorization,
respect the repository's comment conventions, and annotate only stable,
high-signal implementation points with resolvable `[[...]]` references. Avoid
blanket annotations, generated files, and comments that claim stronger
conformance than the evidence supports.

## Recovery And Incremental Use

Migration may stop after any validated batch. On resume:

- rediscover existing artifacts and source references;
- compare candidates by subject and evidence, not only by title;
- skip completed backfill and continue unresolved scope;
- preserve user changes and do not overwrite artifacts to force idempotence; and
- report prior partial state instead of silently starting over.

When `govctl check`, rendering, or review fails, keep the batch unpublished,
correct the evidence or artifact content, and rerun the narrow failing check.
Do not advance lifecycle state to make validation appear complete. Use
diagnostics and command help for recovery; escalate when no authoritative path
is available.

## Completion Evidence

A migration scope is complete when:

- the user-selected candidates are created, deferred, or rejected explicitly;
- each artifact distinguishes recovered fact from uncertainty;
- accepted ADRs and normative RFC phases have supporting evidence and
  authorization;
- optional source annotations are behavior-neutral and resolve;
- no duplicate artifacts or untracked product changes were introduced;
- `govctl check` passes and affected projections are current; and
- the final report lists created artifacts, lifecycle states, annotated paths,
  omitted or uncertain history, remaining scope, and validation results.

The project can begin using `discuss`, `spec`, `gov`, and `quick` after the first
coherent baseline; exhaustive historical backfill is not a prerequisite.
