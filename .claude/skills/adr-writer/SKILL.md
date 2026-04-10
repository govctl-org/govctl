---
name: adr-writer
description: "Write effective Architecture Decision Records. Use when: (1) Creating a new ADR, (2) Recording a design decision, (3) User mentions ADR, decision, trade-off, or alternatives"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional ADR topic]
---

# ADR Writer

Write ADRs that clearly capture context, alternatives, decisions, and consequences.

## Invocation Mode

This helper skill may be used standalone or by `/discuss`, `/gov`, `/spec`, or `/migrate`.
It is responsible for ADR content structure and quality, not ADR lifecycle verbs. Use `/spec` or `/gov` for `govctl adr accept`, `reject`, or `supersede`.

## Authority

ADRs explain decisions: why one option was chosen over others, under what constraints, and with what consequences.
They are justificatory artifacts, not normative mini-RFCs and not work-item execution logs.

## Quick Reference

```bash
govctl adr new "<title>"
govctl adr set <ADR-ID> context --stdin <<'EOF'
context text
EOF
govctl adr add <ADR-ID> alternatives "Option: Description"
govctl adr add <ADR-ID> alternatives "Other option: Description" --reject-reason "Why it was not chosen"
govctl adr tick <ADR-ID> alternatives --at 1 -s rejected
govctl adr tick <ADR-ID> alternatives --at 0 -s accepted
govctl adr set <ADR-ID> decision --stdin <<'EOF'
decision text
EOF
govctl adr set <ADR-ID> consequences --stdin <<'EOF'
consequences text
EOF
govctl adr add <ADR-ID> refs RFC-NNNN
govctl adr add <ADR-ID> tags <tag>
```

## ADR Writing Order

Use this order unless you are doing a historical backfill:

1. Write `context`
2. Add `alternatives`
3. Expand `pros` / `cons`
4. Mark rejected options and record `rejection_reason`
5. Mark the chosen option `accepted`
6. Write `decision` as the conclusion of that discussion
7. Write `consequences`

The key rule is: **alternatives-first, decision-last**. Do not jump straight to a polished conclusion before the ADR shows what was actually considered.

Historical backfills are the exception: if alternatives are not recoverable, say so explicitly in `context` and write the best available `decision` from the surviving evidence.

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

### 2. Alternatives (recommended before writing `decision`)

Document options considered before you write the final decision. Future readers need to know what was _not_ chosen and why.

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
- `rejection_reason`: Required when rejected

**CLI commands:**

```bash
# Simple alternative
govctl adr add <ADR-ID> alternatives "Option A: Use PostgreSQL"

# With pros, cons, and rejection reason
govctl adr add <ADR-ID> alternatives "Option B: Use Redis" \
  --pro "Fast caching" --pro "Simple API" \
  --con "Additional infrastructure" \
  --reject-reason "Overkill for our scale"

# Update alternative state after discussion
govctl adr tick <ADR-ID> alternatives --at 0 -s rejected
govctl adr tick <ADR-ID> alternatives --at 1 -s accepted
govctl adr add <ADR-ID> alt[0].pros "New advantage"
govctl adr remove <ADR-ID> alt[0].cons "Outdated disadvantage"
```

**When to add pros/cons:**

- For significant decisions with multiple options
- When trade-offs are non-obvious
- To help future readers understand the evaluation process

**Key principle:** Let the alternatives show the discussion. The chosen option should emerge there before you write the final decision prose.

### 3. Decision (required)

State the conclusion of the alternatives discussion and why it won. Structure:

> **Do NOT include `## Decision` heading** — the renderer adds it automatically.

```markdown
We will **[action]** because:

1. **Reason one:** Explanation
2. **Reason two:** Explanation

### Implementation Notes

Specific guardrails for implementing this decision, not a task checklist.
```

**Key principle:** Write `decision` as the conclusion of the evaluated alternatives. Lead with the answer, but only after the ADR already shows what was considered.

### 4. Consequences (required)

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
- **Alternatives come first.** The discussion is visible in `alternatives` before the final decision prose settles the issue.
- **Decision is decisive.** Starts with "We will..." — not "We might..." or "We could...".
- **Consequences are honest.** Negative section is non-empty with mitigations.
- **Alternatives are documented.** For new decisions, show the discussion in `alternatives` first and include at least one rejected option with reason. For historical backfills, document rejected options when known; otherwise state that they were not recoverable.
- **References link to related artifacts.** Use `[[artifact-id]]` in content fields.
- **Tagged.** If the project has `[tags]` configured, tag the ADR with relevant domain tags.
- **Stay at the decision layer.** Capture the chosen approach and why, not full normative clause text or task-by-task execution detail.

### What Belongs in an ADR

- The problem that required a decision
- Constraints and decision drivers
- Alternatives considered and why they were accepted or rejected
- The chosen approach, after the alternatives have been evaluated
- Positive, negative, and neutral consequences

### What Does Not Belong in an ADR

- Full RFC-style obligation lists
- Private code structure or language-specific type definitions unless they are central to the design decision itself
- Work-item plans, journal entries, or implementation progress tracking

### Content Field Formatting

Use markdown within content fields. Wrap code/technical terms in backticks:

```
# Good
decision = "We will preserve clause insertion order to keep rendered output stable across runs."

# Bad — drifts into language-specific representation
decision = "Use `HashMap<String, Vec<ClauseSpec>>` for clause storage"
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
| Decision written before alternatives  | Add and evaluate alternatives first; then write decision |
| No alternatives for a new ADR         | Add at least one rejected option                         |
| Historical ADR lacks rejected options | State that alternatives were not recoverable             |
| Vague context: "We need to decide"    | Specific: "RFC-0002 requires X but doesn't specify how"  |
| Decision buried in prose              | Lead with "We will **action**"                           |
| Missing refs                          | Link to RFCs/ADRs that constrain the decision            |
| ADR turns into a mini-RFC             | Move obligation details into an RFC                      |
| ADR turns into a task plan            | Move execution detail into a work item                   |
