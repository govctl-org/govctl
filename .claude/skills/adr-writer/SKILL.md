---
name: adr-writer
description: "Write effective Architecture Decision Records. Use when: (1) Creating a new ADR, (2) Recording a design decision, (3) User mentions ADR, decision, trade-off, or alternatives"
---

# ADR Writer

Write ADRs that clearly capture context, decisions, and consequences.

## Quick Reference

```bash
govctl adr new "<title>"
govctl adr set <ADR-ID> context --stdin <<'EOF'
context text
EOF
govctl adr set <ADR-ID> decision --stdin <<'EOF'
decision text
EOF
govctl adr set <ADR-ID> consequences --stdin <<'EOF'
consequences text
EOF
govctl adr add <ADR-ID> alternatives "Option: Description"
govctl adr add <ADR-ID> refs RFC-NNNN
```

## ADR Structure

Every ADR has three required fields and two optional fields:

### 1. Context (required)

Explain the situation that requires a decision. Structure:

```markdown
## Context

[1-2 sentence summary of the situation]

### Problem Statement

What specific issue are we addressing?

### Constraints

What existing RFCs, ADRs, or technical limitations restrict our options?

### Options Considered

Brief overview (details go in the alternatives field).
```

**Key principle:** A reader 6 months from now must understand _why_ this decision was needed without asking anyone.

### 2. Decision (required)

State what was decided and why. Structure:

```markdown
## Decision

We will **[action]** because:

1. **Reason one:** Explanation
2. **Reason two:** Explanation

### Implementation Notes

Specific guidance for implementing this decision.
```

**Key principle:** Lead with the decision, then justify. Don't bury the answer.

### 3. Consequences (required)

Honest accounting of trade-offs. Structure:

```markdown
## Consequences

### Positive

- Benefit one
- Benefit two

### Negative

- Trade-off one (mitigation: ...)
- Trade-off two (mitigation: ...)

### Neutral

- Side effect that is neither positive nor negative
```

**Key principle:** Every decision has downsides. If your Negative section is empty, you haven't thought hard enough.

### 4. Alternatives (recommended)

Document options considered. Future readers need to know what was _not_ chosen and why.

**Extended structure per ADR-0027:**

    [[content.alternatives]]
    text = "Option A: Description"
    status = "rejected"
    pros = ["Advantage 1", "Advantage 2"]
    cons = ["Disadvantage 1"]
    rejection_reason = "Why this was not chosen"

**Field semantics:**

- `text` (required): Description of the alternative
- `status`: `considered` (default) | `accepted` | `rejected`
- `pros`: List of advantages
- `cons`: List of disadvantages
- `rejection_reason`: If rejected, explains why

**When to add pros/cons:**

- For significant decisions with multiple options
- When trade-offs are non-obvious
- To help future readers understand the evaluation process

### 5. References (recommended)

```bash
govctl adr add <ADR-ID> refs RFC-0001
govctl adr add <ADR-ID> refs ADR-0005
```

Link to artifacts that constrained or informed the decision. Use plain IDs (not `[[...]]` syntax) in the refs field.

## Writing Rules

### Quality Checklist

- **Context is complete.** Problem statement, constraints, and options are all present.
- **Decision is decisive.** Starts with "We will..." — not "We might..." or "We could...".
- **Consequences are honest.** Negative section is non-empty with mitigations.
- **Alternatives are documented.** At least one rejected option with reason.
- **References link to related artifacts.** Use `[[artifact-id]]` in content fields.

### Content Field Formatting

Use markdown within content fields. Wrap code/technical terms in backticks:

```
# Good
decision = "Use `HashMap<String, Vec<ClauseSpec>>` for clause storage"

# Bad — angle brackets break mdbook rendering
decision = "Use HashMap<String, Vec<ClauseSpec>> for clause storage"
```

## Common Mistakes

| Mistake                            | Fix                                                     |
| ---------------------------------- | ------------------------------------------------------- |
| Empty Negative section             | Every decision has trade-offs — document them           |
| No alternatives                    | Add at least one rejected option                        |
| Vague context: "We need to decide" | Specific: "RFC-0002 requires X but doesn't specify how" |
| Decision buried in prose           | Lead with "We will **action**"                          |
| Missing refs                       | Link to RFCs/ADRs that constrain the decision           |
