---
name: adr-writer
description: "Write effective Architecture Decision Records. Use when: (1) Creating a new ADR, (2) Recording a design decision, (3) User mentions ADR, decision, trade-off, or alternatives"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional ADR topic]
---

# ADR Writer

Write ADRs that clearly capture context, decisions, and consequences.

## Invocation Mode

This helper skill may be used standalone or by `/discuss`, `/gov`, `/spec`, or `/migrate`.
It is responsible for ADR content structure and quality, not ADR lifecycle verbs. Use `/spec` or `/gov` for `govctl adr accept`, `reject`, or `supersede`.

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

> **Do NOT include `## Context` heading** — the renderer adds it automatically.

```markdown
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

> **Do NOT include `## Decision` heading** — the renderer adds it automatically.

```markdown
We will **[action]** because:

1. **Reason one:** Explanation
2. **Reason two:** Explanation

### Implementation Notes

Specific guidance for implementing this decision.
```

**Key principle:** Lead with the decision, then justify. Don't bury the answer.

### 3. Consequences (required)

Honest accounting of trade-offs. Structure:

> **Do NOT include `## Consequences` heading** — the renderer adds it automatically.

```markdown
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

**CLI commands:**

```bash
# Simple alternative
govctl adr add <ADR-ID> alternatives "Option A: Use PostgreSQL"

# With pros, cons, and rejection reason
govctl adr add <ADR-ID> alternatives "Option B: Use Redis" \
  --pro "Fast caching" --pro "Simple API" \
  --con "Additional infrastructure" \
  --reject-reason "Overkill for our scale"

# Edit nested fields after creation
govctl adr tick <ADR-ID> alternatives --at 0 -s rejected
govctl adr add <ADR-ID> alt[0].pros "New advantage"
govctl adr remove <ADR-ID> alt[0].cons "Outdated disadvantage"
```

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

## Validation and Handoff

- Run `govctl check` after substantive ADR edits
- Use `adr-reviewer` before acceptance or handoff
- Use `/spec` for ADR acceptance without implementation
- Use `/gov` when the ADR accompanies implementation-bearing work

## Writing Rules

### Quality Checklist

- **Context is complete.** Problem statement, constraints, and options are all present.
- **Decision is decisive.** Starts with "We will..." — not "We might..." or "We could...".
- **Consequences are honest.** Negative section is non-empty with mitigations.
- **Alternatives are documented.** For new decisions, include at least one rejected option with reason. For historical backfills, document rejected options when known; otherwise state that they were not recoverable.
- **References link to related artifacts.** Use `[[artifact-id]]` in content fields.

### Content Field Formatting

Use markdown within content fields. Wrap code/technical terms in backticks:

```
# Good
decision = "Use `HashMap<String, Vec<ClauseSpec>>` for clause storage"

# Bad — angle brackets break mdbook rendering
decision = "Use HashMap<String, Vec<ClauseSpec>> for clause storage"
```

## Rendering Rules

The renderer auto-generates structural elements from TOML metadata. **Do NOT include these in content fields:**

- `## Context`, `## Decision`, `## Consequences` headings — auto-generated for each section
- `## Alternatives Considered` heading — auto-generated if alternatives exist
- `### Option Name (status)` headings — auto-generated from `alternatives[].text` + `status`
- `- **Pros:**`, `- **Cons:**`, `- **Rejected because:**` — auto-generated from structured fields
- ADR title (`# ADR-NNNN: Title`) — auto-generated from metadata

Content fields should contain only the body prose and `[[...]]` references.

## Common Mistakes

| Mistake                               | Fix                                                      |
| ------------------------------------- | -------------------------------------------------------- |
| `## Context` in content field         | Don't — the renderer adds section headings automatically |
| Empty Negative section                | Every decision has trade-offs — document them            |
| No alternatives for a new ADR         | Add at least one rejected option                         |
| Historical ADR lacks rejected options | State that alternatives were not recoverable             |
| Vague context: "We need to decide"    | Specific: "RFC-0002 requires X but doesn't specify how"  |
| Decision buried in prose              | Lead with "We will **action**"                           |
| Missing refs                          | Link to RFCs/ADRs that constrain the decision            |
