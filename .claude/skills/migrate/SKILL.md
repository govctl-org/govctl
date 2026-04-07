---
name: migrate
description: "Adopt govctl in an existing project. Discovers undocumented decisions, backfills ADRs/RFCs, annotates source code. Use when: (1) Project has no governance yet, (2) User mentions migrate, adopt, onboard, or brownfield"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional scope hint, e.g. "focus on database decisions"]
---

# /migrate — Adopt govctl in an Existing Project

Migrate an existing codebase to govctl governance per [[ADR-0032]].

**Purpose:** Systematically discover undocumented decisions and specifications in an existing project, codify them as govctl artifacts, and annotate source code with cross-references.

**Outputs:** Discovery report, backfilled governance artifacts, annotated source references, and an initial governed baseline.

**Properties:**

- **Interactive** — Confirms discoveries with the user before creating artifacts
- **Incremental** — Each phase can be run independently; partial migration is valid
- **Non-destructive** — Never overwrites existing files; only adds governance artifacts

## Critical Rules

1. This is a historical backfill workflow, not an implementation workflow.
2. Do not create work items during discovery or backfill phases unless you are establishing baseline tracking for already in-progress work.
3. Ask permission before lifecycle-owned verbs used in migration: `govctl adr accept`, `govctl rfc finalize`, and `govctl rfc advance`.
4. Never edit governed files directly. Use `govctl` verbs only.
5. Use `/commit` to record migration milestones. Do not embed raw VCS procedures in this workflow.
6. If migration uncovers unresolved design questions for future work, hand them off to `/discuss` rather than inventing rationale.

---

## QUICK REFERENCE

```bash
# Scaffold
govctl init                               # Initialize governance structure
govctl status                             # Verify setup

# Backfill ADRs
govctl adr new "<decision title>"
govctl adr set <ADR-ID> context --stdin <<'EOF' ... EOF
govctl adr add <ADR-ID> alternatives "Option: ..." --pro "..." --con "..." --reject-reason "..."
govctl adr tick <ADR-ID> alternatives --at 0 -s accepted
govctl adr set <ADR-ID> decision --stdin <<'EOF' ... EOF
govctl adr set <ADR-ID> consequences --stdin <<'EOF' ... EOF
govctl adr accept <ADR-ID>

# Backfill RFCs (optional)
govctl rfc new "<spec title>"
govctl clause new <RFC-ID>:C-<NAME> "<title>" -s "Specification" -k normative
govctl clause edit <RFC-ID>:C-<NAME> text --stdin <<'EOF' ... EOF
govctl rfc finalize <RFC-ID> normative
govctl rfc advance <RFC-ID> impl
govctl rfc advance <RFC-ID> test
govctl rfc advance <RFC-ID> stable

# Validate
govctl check
```

---

## PHASE 0: SCAFFOLD

### 0.1 Initialize govctl

```bash
govctl init
```

This creates the `gov/` directory structure alongside existing project files. It is safe to run in an existing repo — it does not overwrite existing files.

### 0.2 Verify Setup

```bash
govctl status
```

Confirm the governance structure was created. Read `gov/config.toml` and adjust if needed (e.g., `source_scan.include` patterns for the project's language).

### 0.3 Detect VCS

If migration milestones should be recorded, `/commit` will choose the raw VCS workflow.

### 0.4 Initial Commit

If the user wants to record scaffold creation, use `/commit` with `chore(gov): initialize govctl governance structure`.

---

## PHASE 1: DISCOVER

Systematically scan the project to find implicit governance artifacts. **Do not create any govctl artifacts yet** — this phase is purely discovery.

### 1.1 Read Project Overview

Read these files (if they exist) to understand the project:

- `README.md` — Project purpose, tech stack, architecture overview
- `CONTRIBUTING.md` — Development conventions and processes
- `ARCHITECTURE.md` or `docs/architecture.md` — System design
- `CHANGELOG.md` — History of changes and decisions
- `Makefile`, `Justfile`, `package.json`, `Cargo.toml` — Build system and dependencies

### 1.2 Discover Architectural Decisions

Scan for implicit decisions in:

| Source              | What to look for                                                                     |
| ------------------- | ------------------------------------------------------------------------------------ |
| README/docs         | "We chose X because...", "This project uses..."                                      |
| Config files        | Framework choices, database configs, deployment targets                              |
| Dependencies        | Major library choices (ORM, web framework, test framework)                           |
| Directory structure | Architectural patterns (monorepo, microservices, MVC, hexagonal)                     |
| Code comments       | "TODO: migrate to...", "HACK: because X doesn't support...", "We use X instead of Y" |
| Git/jj history      | Large refactors, technology migrations, design pivots                                |

For each discovered decision, note:

- **What** was decided
- **Why** (if discernible from context)
- **What alternatives** existed (if known)
- **Where** in the code this decision is implemented

### 1.3 Discover Existing Specifications

Look for documents that function as specifications:

- API contracts (OpenAPI specs, GraphQL schemas, protobuf definitions)
- Design docs or RFCs in markdown
- Interface definitions or type contracts
- Formal requirements documents

### 1.4 Discover In-Progress Work

Check for:

- Open issues in the repo (if accessible)
- TODO/FIXME/HACK comments in code
- Feature branches
- Draft PRs

### 1.5 Present Discovery Report

**Present findings to the user before proceeding.** Format:

```
=== MIGRATION DISCOVERY REPORT ===

Architectural Decisions Found: N
  1. [Brief description] — source: [where found]
  2. ...

Existing Specifications Found: N
  1. [Brief description] — source: [file path]
  2. ...

In-Progress Work Found: N
  1. [Brief description] — source: [where found]
  2. ...

Recommended migration scope:
  - ADRs to create: [list]
  - RFCs to create: [list or "none"]
  - Work items to create: [list or "none"]
```

**Ask the user:** Which items should be backfilled? The user may choose to skip some or prioritize others. Respect their choices.

---

## PHASE 2: BACKFILL ADRs

For each decision the user confirmed, create an ADR.

### 2.1 Create ADR

Follow the **adr-writer** skill for quality guidelines.

```bash
govctl adr new "<decision title>"
```

### 2.2 Populate Context and Consequences

```bash
govctl adr set <ADR-ID> context --stdin <<'EOF'
[Problem statement and what prompted the decision]
EOF

govctl adr set <ADR-ID> consequences --stdin <<'EOF'
### Positive
- [Observed benefits]

### Negative
- [Observed downsides or trade-offs]

### Neutral
- [Side effects]
EOF
```

### 2.3 Add Alternatives (if known)

```bash
govctl adr add <ADR-ID> alternatives "Chosen: <what was adopted>" \
  --pro "..." --con "..."
govctl adr add <ADR-ID> alternatives "Rejected: <what was not chosen>" \
  --pro "..." --con "..." --reject-reason "..."
govctl adr tick <ADR-ID> alternatives --at 0 -s accepted
```

Preserve the normal ADR discussion order during backfill when possible:

1. Record the alternatives that are still recoverable
2. Mark the chosen option `accepted`
3. Mark non-chosen options `rejected` with reasons when known
4. Only then write or finalize the `decision` prose

If non-selected alternatives are not recoverable, it is acceptable to omit `alternatives` entirely and say so explicitly in the ADR context. Historical backfills may not be able to reconstruct rejected options, and reviewers should evaluate them with that limitation in mind.

### 2.4 Write the Decision Last

```bash
govctl adr set <ADR-ID> decision --stdin <<'EOF'
We will [what was decided].

[Rationale — why this was chosen]
EOF
```

Treat `decision` as the conclusion of the reconstructed alternatives discussion, not as the starting point.

### 2.5 Review Backfilled ADRs

Invoke the **adr-reviewer** agent on each newly created ADR. For large batches, review the most important 3-5 ADRs and spot-check the rest.

Fix Critical findings before accepting the ADRs.

### 2.6 Accept and Cross-Reference

Since these are historical decisions already in effect:

```bash
govctl adr accept <ADR-ID>
govctl adr add <ADR-ID> refs <related-ADR-or-RFC>
```

### 2.6 Commit Batch

Group related ADRs into logical commits:

Use `/commit` with `docs(adr): backfill ADRs for existing architectural decisions`.

---

## PHASE 3: BACKFILL RFCs (Optional)

**Only if the user confirmed existing specifications in Phase 1.** Most migrations skip this phase.

### 3.1 Create RFC from Existing Spec

```bash
govctl rfc new "<spec title>"
```

### 3.2 Create Clauses from Existing Requirements

For each requirement in the existing specification:

```bash
govctl clause new <RFC-ID>:C-<NAME> "<title>" -s "Specification" -k normative
govctl clause edit <RFC-ID>:C-<NAME> text --stdin <<'EOF'
[Clause text extracted from existing spec, rewritten with RFC 2119 keywords]
EOF
```

### 3.3 Review Backfilled RFCs

Invoke the **rfc-reviewer** agent on each newly created RFC. For large batches, review the most important RFCs first and spot-check the rest.

Fix Critical findings before making the RFC authoritative.

### 3.4 Finalize and Advance

Migration is a historical backfill workflow. Only finalize and advance the RFC after the user confirms the spec is already implemented and tested:

```bash
govctl rfc finalize <RFC-ID> normative
govctl rfc advance <RFC-ID> impl
govctl rfc advance <RFC-ID> test
govctl rfc advance <RFC-ID> stable
```

### 3.5 Commit

Use `/commit` with `docs(rfc): backfill RFCs for existing specifications`.

---

## PHASE 4: ANNOTATE SOURCE

Add `[[...]]` references to existing source code so `govctl check` can trace implementations to their governing artifacts.

### 4.1 Scan and Annotate

For each newly created ADR/RFC, find the source files that implement the decision or specification:

```bash
# Example: if ADR-0001 decided to use PostgreSQL
# Find database-related files and add reference comments:
```

Add comments in the project's comment style:

```python
# Per [[ADR-0001]], we use PostgreSQL for persistence
```

```rust
// Implements [[RFC-0001:C-VALIDATION]]
```

```typescript
// Per [[ADR-0003]], API responses use camelCase
```

### 4.2 Validate References

```bash
govctl check
```

Fix any broken references. All `[[...]]` references must resolve to existing artifacts.

### 4.3 Commit

Use `/commit` with `chore(gov): annotate source with governance artifact references`.

---

## PHASE 5: ESTABLISH BASELINE

Create work items for any in-progress work discovered in Phase 1.

### 5.1 Create Work Items

For each active task:

```bash
govctl work new --active "<task title>"
govctl work set <WI-ID> description "<what is being done>"
govctl work add <WI-ID> acceptance_criteria "add: <expected outcome>"
govctl work add <WI-ID> refs <related-ADR-or-RFC>
```

### 5.2 Final Validation

```bash
govctl check
govctl render
```

### 5.3 Final Commit

Use `/commit` with `chore(gov): establish govctl governance baseline`.

---

## PHASE 6: SUMMARY

Present the migration results:

```
=== MIGRATION COMPLETE ===

Project: <project name>

Artifacts created:
  ADRs: N (documenting existing architectural decisions)
  RFCs: N (codifying existing specifications)
  Work Items: N (tracking in-progress tasks)

Source annotations: N files annotated with [[...]] references

Validation: govctl check passes

Next steps:
  - Use /gov for all new work going forward
  - Use /discuss for new design decisions
  - Incrementally annotate more source files as you touch them
  - Run govctl check in CI to enforce governance going forward
```

---

## TIPS

### Prioritization

Not everything needs an ADR. Focus on decisions that:

- **Affect multiple developers** (framework choice, API conventions)
- **Are hard to reverse** (database choice, authentication strategy)
- **Generate recurring questions** ("Why do we use X instead of Y?")

Skip trivial decisions (indentation style, variable naming) — those belong in a linter config, not an ADR.

### Handling Uncertainty

When discovering decisions:

- If you can identify the decision but not the rationale → say so in the context. "The rationale is not documented; this ADR records the current state."
- If alternatives are unknown → omit `alternatives` and say in the ADR context that non-selected options were not recoverable.
- If alternatives are known → capture them first and let the decision prose summarize the conclusion, rather than skipping straight to the final answer.
- If consequences are unclear → document what you can observe. "The negative consequences of this decision have not been formally evaluated."

### Incremental Migration

It's fine to migrate in stages:

- **Week 1:** Scaffold + top 5 most important ADRs
- **Week 2:** Annotate the most-touched modules
- **Later:** Backfill as you encounter undocumented decisions during regular work

The `/gov` workflow naturally extends migration — every new decision gets an ADR, every new feature gets a work item.
