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
{{GOVCTL}} status                             # Project overview
{{GOVCTL}} rfc list                           # List all RFCs
{{GOVCTL}} adr list                           # List all ADRs

# RFC drafting
{{GOVCTL}} rfc new "<title>"                  # Create RFC (auto-assigns ID)
{{GOVCTL}} clause new <RFC-ID>:C-<NAME> "<title>" -s "<section>" -k <kind>
{{GOVCTL}} clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
clause text here
EOF

# ADR drafting
{{GOVCTL}} adr new "<title>"                  # Create ADR
{{GOVCTL}} adr set <ADR-ID> context "..." --stdin
{{GOVCTL}} adr set <ADR-ID> decision "..." --stdin
{{GOVCTL}} adr set <ADR-ID> consequences "..." --stdin
{{GOVCTL}} adr add <ADR-ID> alternatives "Option: Description"
{{GOVCTL}} adr add <ADR-ID> refs RFC-0001

# Validation
{{GOVCTL}} check                              # Validate all artifacts
```

---

## CRITICAL RULES

1. **Discussion-first** — Understand the problem before proposing solutions
2. **Research existing context** — Check what RFCs/ADRs already exist and reference them
3. **Draft only** — Never finalize RFCs or accept ADRs in this workflow
4. **No work items** — This is spec phase; work items come later with `/gov`
5. **Ask when unclear** — If requirements are ambiguous, ask clarifying questions
6. **Quality over speed** — Produce complete, well-structured drafts

---

## PHASE 0: CONTEXT DISCOVERY

### 0.1 Survey Existing Governance

Before discussing, understand what already exists:

```bash
{{GOVCTL}} status
{{GOVCTL}} rfc list
{{GOVCTL}} adr list
```

### 0.2 Identify Relevant Artifacts

Based on `$ARGUMENTS`, identify RFCs and ADRs that might be relevant:

- **Related RFCs:** Specifications that touch the same domain
- **Related ADRs:** Previous decisions that constrain options
- **Superseded artifacts:** Old decisions that may need updating

Read relevant artifacts to understand existing constraints and decisions.

### 0.3 Note Project Configuration

```bash
cat gov/config.toml
```

Understand project-specific settings that may affect the design.

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

### 2.1 RFC Drafting (for new capabilities/specifications)

**When to create an RFC:**

- New feature or capability
- Behavioral contract that code must follow
- API or interface specification
- Cross-cutting concern (e.g., error handling, logging)

**RFC Structure:**

```bash
# Create the RFC
{{GOVCTL}} rfc new "<descriptive-title>"

# Add Summary clause (informative)
{{GOVCTL}} clause new <RFC-ID>:C-SUMMARY "Summary" -s "Summary" -k informative
{{GOVCTL}} clause edit <RFC-ID>:C-SUMMARY --stdin <<'EOF'
Brief overview of what this RFC specifies and why.

**Scope:** What this RFC covers and does not cover.

**Rationale:** Why this specification is needed.

*Since: v0.1.0*
EOF

# Add Specification clauses (normative)
{{GOVCTL}} clause new <RFC-ID>:C-<NAME> "<Title>" -s "Specification" -k normative
{{GOVCTL}} clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
The system MUST...
The system SHOULD...
The system MAY...

**Rationale:**
Why this requirement exists.

*Since: v0.1.0*
EOF
```

**RFC Writing Guidelines:**

1. **Use RFC 2119 keywords:** MUST, SHOULD, MAY (all caps)
2. **Be specific:** Avoid ambiguous terms like "appropriate" or "reasonable"
3. **Include rationale:** Explain why, not just what
4. **Reference existing artifacts:** Use `[[RFC-NNNN]]` or `[[ADR-NNNN]]` syntax
5. **Add `*Since: vX.Y.Z*`:** Track when each clause was introduced

### 2.2 ADR Drafting (for design decisions)

**When to create an ADR:**

- Choosing between alternatives (library, pattern, approach)
- Interpreting an ambiguous RFC requirement
- Recording architectural constraints
- Documenting why we're NOT doing something

**ADR Structure:**

```bash
# Create the ADR
{{GOVCTL}} adr new "<decision-title>"

# Set context (problem statement)
{{GOVCTL}} adr set <ADR-ID> context --stdin <<'EOF'
## Context

Describe the situation that requires a decision.

### Problem Statement
What is the issue we're addressing?

### Constraints
What existing requirements or decisions limit our options?

### Options Considered
Brief overview of alternatives (detailed in alternatives field).
EOF

# Set decision (the choice and rationale)
{{GOVCTL}} adr set <ADR-ID> decision --stdin <<'EOF'
## Decision

We will **<action>** because:

1. **Reason one:** Explanation
2. **Reason two:** Explanation
3. **Reason three:** Explanation

### Implementation Notes
Any specific guidance for implementing this decision.
EOF

# Set consequences (trade-offs)
{{GOVCTL}} adr set <ADR-ID> consequences --stdin <<'EOF'
## Consequences

### Positive
- Benefit one
- Benefit two

### Negative
- Trade-off one (mitigation: ...)
- Trade-off two (mitigation: ...)

### Neutral
- Side effect that is neither positive nor negative
EOF

# Add alternatives considered
{{GOVCTL}} adr add <ADR-ID> alternatives "Option A: Description of alternative"
{{GOVCTL}} adr add <ADR-ID> alternatives "Option B: Description of alternative"

# Add references to related artifacts
{{GOVCTL}} adr add <ADR-ID> refs RFC-0001
{{GOVCTL}} adr add <ADR-ID> refs ADR-0005
```

**ADR Writing Guidelines:**

1. **Context is crucial:** Future readers need to understand why this decision was made
2. **Be explicit about trade-offs:** What are we giving up?
3. **Document alternatives:** Even rejected options provide valuable context
4. **Reference constraints:** Link to RFCs/ADRs that influenced the decision
5. **Keep it focused:** One decision per ADR

### 2.3 RFC Amendment (for changes to existing specs)

**When existing RFC needs modification:**

```bash
# Edit the clause content
{{GOVCTL}} clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
Updated specification text.
EOF

# The RFC version will need bumping during /gov workflow
# Do NOT bump version in /discuss — that happens at implementation time
```

**Note:** Amendments to normative RFCs require careful consideration. Document the rationale for the change.

### 2.4 Validate Drafts

After creating artifacts:

```bash
{{GOVCTL}} check
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

### RFC 2119 Keywords

In normative clauses, use these keywords (all caps):

| Keyword    | Meaning                        |
| ---------- | ------------------------------ |
| MUST       | Absolute requirement           |
| MUST NOT   | Absolute prohibition           |
| SHOULD     | Recommended but not required   |
| SHOULD NOT | Discouraged but not prohibited |
| MAY        | Optional                       |

### Section Names

**For RFCs:**

- "Summary" — Overview and rationale (informative)
- "Specification" — Requirements (normative)
- "Rationale" — Extended explanation (informative)

**For ADRs:**

- Context, Decision, Consequences are the standard fields
- Use markdown headers within these fields for structure

### Multi-line Input

Use `--stdin` with heredoc:

```bash
{{GOVCTL}} clause edit <clause-id> --stdin <<'EOF'
Multi-line content here.
Second line.
EOF
```

**Key:** Always use `<<'EOF'` (quoted) to prevent variable expansion.

---

## DISCUSSION CHECKLIST

- [ ] Existing RFCs/ADRs surveyed
- [ ] Topic classified (RFC, ADR, both, or neither)
- [ ] Requirements clarified (asked questions if needed)
- [ ] Design options explored
- [ ] Draft artifact(s) created with complete structure
- [ ] Validation passed (`{{GOVCTL}} check`)
- [ ] Summary presented
- [ ] Next steps offered

**BEGIN DISCUSSION NOW.**
