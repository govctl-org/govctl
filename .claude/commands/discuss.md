---
description: Facilitate design discussion — research context, clarify requirements, draft RFC/ADR
allowed-tools: Read, Write, StrReplace, Shell, Glob, Grep, LS, SemanticSearch, TodoWrite
argument-hint: <topic-or-question>
---

# /discuss — Design Discussion Workflow

Facilitate a design discussion about: `$ARGUMENTS`

**Purpose:** Understand a design problem, research existing governance context, and produce draft RFC or ADR artifacts. This workflow is for the **spec phase** — no implementation, no work items.

**Outputs:** Draft RFC and/or proposed ADR, then handoff to implementation workflow.

---

## QUICK REFERENCE

```bash
# Context discovery
govctl status                             # Project overview
govctl rfc list                           # List all RFCs
govctl adr list                           # List all ADRs

# RFC drafting
govctl rfc new "<title>"                  # Create RFC (auto-assigns ID)
govctl clause new <RFC-ID>:C-<NAME> "<title>" -s "<section>" -k <kind>
govctl clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
clause text here
EOF

# ADR drafting
govctl adr new "<title>"                  # Create ADR
govctl adr set <ADR-ID> context "..." --stdin
govctl adr set <ADR-ID> decision "..." --stdin
govctl adr set <ADR-ID> consequences "..." --stdin
govctl adr add <ADR-ID> alternatives "Option: Description"
govctl adr add <ADR-ID> refs RFC-0001

# Validation
govctl check                              # Validate all artifacts
```

---

## CRITICAL RULES

1. **Discussion-first** — Understand the problem before proposing solutions
2. **Research existing context** — Check what RFCs/ADRs already exist and reference them
3. **Draft only** — Never finalize RFCs or accept ADRs in this workflow
4. **No work items** — This is spec phase; work items come later with `/gov`
5. **Ask when unclear** — If requirements are ambiguous, ask clarifying questions
6. **Quality over speed** — Produce complete, well-structured drafts
7. **Reference format** — Always use `[[artifact-id]]` syntax when referencing artifacts (e.g., `[[RFC-0001]]`, `[[RFC-0001:C-FOO]]`) — in content fields AND code comments

---

## PHASE 0: CONTEXT DISCOVERY

### 0.1 Survey Existing Governance

Before discussing, understand what already exists:

```bash
govctl status
govctl rfc list
govctl adr list
```

### 0.2 Identify Relevant Artifacts

Based on `$ARGUMENTS`, identify RFCs and ADRs that might be relevant:

- **Related RFCs:** Specifications that touch the same domain
- **Related ADRs:** Previous decisions that constrain options
- **Superseded artifacts:** Old decisions that may need updating

Read relevant artifacts to understand existing constraints and decisions.

### 0.3 Note Project Configuration

Read `gov/config.toml` to understand project-specific settings that may affect the design.

---

## PHASE 1: CLASSIFICATION & DISCUSSION

### 1.1 Classify the Topic

Parse `$ARGUMENTS` and classify:

| Type               | Indicator                                    | Output                        |
| ------------------ | -------------------------------------------- | ----------------------------- |
| **New capability** | "How should X work?", "Design Y feature"     | RFC                           |
| **Design choice**  | "Should we use A or B?", "Decide between..." | ADR                           |
| **Clarification**  | "What does RFC-NNNN mean by...?"             | Discussion only (no artifact) |
| **Amendment**      | "RFC-NNNN should change because..."          | RFC version bump              |
| **Both**           | Complex feature with architectural decisions | RFC + ADR(s)                  |

### 1.2 Discussion Phase

**If requirements are clear:** Proceed to Phase 2.

**If requirements are ambiguous:** Ask clarifying questions before proceeding.

Questions to consider:

- What problem are we solving?
- Who are the users/consumers?
- What are the constraints (performance, compatibility, complexity)?
- What are the trade-offs we're willing to make?
- Are there existing patterns we should follow or deviate from?

**Do not invent requirements.** If something is unspecified, ask.

### 1.3 Design Exploration

For complex topics, explore the design space:

1. **Identify options:** What are the possible approaches?
2. **Analyze trade-offs:** What does each option make easier/harder?
3. **Check constraints:** What do existing RFCs/ADRs require or prohibit?
4. **Recommend:** Which option best fits the project's needs?

Document this exploration — it becomes the ADR context/alternatives or RFC rationale.

---

## PHASE 2: DRAFT ARTIFACTS

### 2.1 RFC Drafting (if needed)

For structure, templates, and quality guidelines, follow the **rfc-writer** skill.

```bash
govctl rfc new "<title>"
govctl clause new <RFC-ID>:C-<NAME> "<title>" -s "<section>" -k <kind>
govctl clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
clause text
EOF
```

### 2.2 ADR Drafting (if needed)

For structure, templates, and quality guidelines, follow the **adr-writer** skill.

```bash
govctl adr new "<title>"
govctl adr set <ADR-ID> context --stdin <<'EOF' ... EOF
govctl adr set <ADR-ID> decision --stdin <<'EOF' ... EOF
govctl adr set <ADR-ID> consequences --stdin <<'EOF' ... EOF
govctl adr add <ADR-ID> alternatives "Option: Description"
govctl adr add <ADR-ID> refs RFC-NNNN
```

### 2.3 RFC Amendment (for changes to existing specs)

**When existing RFC needs modification:**

```bash
# Edit the clause content
govctl clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
Updated specification text.
EOF

# The RFC version will need bumping during /gov workflow
# Do NOT bump version in /discuss — that happens at implementation time
```

**Note:** Amendments to normative RFCs require careful consideration. Document the rationale for the change.

### 2.4 Validate Drafts

After creating artifacts:

```bash
govctl check
```

Fix any validation errors before proceeding.

### 2.5 Record (Optional)

If you want to save progress:

```bash
# jj
jj commit -m "docs(rfc): draft <RFC-ID> for <summary>"

# git
git add . && git commit -m "docs(rfc): draft <RFC-ID> for <summary>"
```

---

## PHASE 3: HANDOFF

### 3.1 Summary Report

Present the discussion results:

```
=== DISCUSSION COMPLETE ===

Topic: $ARGUMENTS

Artifacts created:
  - RFC-NNNN: <title> (draft, spec phase)
  - ADR-NNNN: <title> (proposed)

Key decisions:
  - <summary of main design choices>

Open questions:
  - <any unresolved issues>

Related artifacts referenced:
  - RFC-XXXX: <title>
  - ADR-YYYY: <title>
```

### 3.2 Next Steps

Prompt the user for next action:

```
Ready to proceed?

Options:
  1. /gov "<summary>" — Start governed implementation workflow
     - Creates work item
     - Finalizes RFC (with permission)
     - Implements, tests, completes

  2. /quick "<summary>" — Fast path for trivial implementation
     - Use if implementation is straightforward
     - Skips RFC finalization ceremony

  3. Continue discussing — Refine the drafts further
     - Ask follow-up questions
     - Add more clauses or detail

  4. Pause — Save drafts, return later
     - Drafts are committed and can be resumed
```

---

## ERROR HANDLING

### When to Stop and Ask

1. **Conflicting requirements** — existing RFCs/ADRs contradict each other
2. **Scope unclear** — cannot determine what's in/out of scope
3. **Missing context** — need information not available in codebase
4. **Breaking change** — proposal would break existing normative behavior

### When to Proceed

1. **Minor ambiguity** — make reasonable assumption, document it
2. **Style questions** — follow existing patterns in the codebase
3. **Optional details** — defer to implementation phase

---

## CONVENTIONS

### Artifact References

Use `[[artifact-id]]` syntax for inline references in content fields:

```
# Good - expands to clickable link when rendered
context = "Per [[RFC-0001]], all RFCs must have a summary clause."

# Also good for clauses
decision = "Follow [[RFC-0001:C-SUMMARY]] structure."

# Bad - plain text, not linked
context = "Per RFC-0001, all RFCs must have a summary clause."
```

**Also use in source code comments** for implementation traceability:

```rust
// Implements [[RFC-0001:C-VALIDATION]]
fn validate() { ... }

// Per [[ADR-0005]], we chose X over Y
```

This enables `govctl check` to validate all references exist and are not deprecated.

For RFC 2119 keywords and clause conventions, see the **rfc-writer** skill.
For ADR structure and field conventions, see the **adr-writer** skill.

---

## DISCUSSION CHECKLIST

- [ ] Existing RFCs/ADRs surveyed
- [ ] Topic classified (RFC, ADR, both, or neither)
- [ ] Requirements clarified (asked questions if needed)
- [ ] Design options explored
- [ ] Draft artifact(s) created with complete structure
- [ ] Validation passed (`govctl check`)
- [ ] Summary presented
- [ ] Next steps offered

**BEGIN DISCUSSION NOW.**
