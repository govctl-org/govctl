---
name: rfc-writer
description: "Write well-structured RFCs with normative clauses. Use when: (1) Creating a new RFC, (2) Adding or editing RFC clauses, (3) User mentions RFC, specification, or normative requirements"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional RFC topic]"
---

# RFC Writer

Write RFCs that are precise, complete, and follow govctl conventions.

## Invocation Mode

This helper skill may be used standalone or by `/discuss`, `/gov`, `/spec`, or `/migrate`.
It is responsible for RFC content and clause quality, not RFC lifecycle verbs. Use `/spec` or `/gov` for `govctl rfc finalize`, `bump`, or `advance`.

## Authority

RFCs define obligations: what behavior, invariants, interfaces, and compatibility rules MUST be true.
They are normative artifacts, not design diaries, code sketches, or task plans.

## Quick Reference

```bash
govctl rfc new "<title>"
govctl clause new <RFC-ID>:C-<NAME> "<title>" -s "<section>" -k <kind>
govctl clause edit <RFC-ID>:C-<NAME> text --stdin <<'EOF'
clause text
EOF
govctl rfc add <RFC-ID> tags <tag>
```

## RFC Structure

Every RFC should have:

1. **Summary clause** (informative) — what this RFC covers and why
2. **Specification clauses** (normative) — the actual requirements
3. **Rationale sections** within clauses — why each requirement exists

### Summary Clause Template

```bash
govctl clause new <RFC-ID>:C-SUMMARY "Summary" -s "Summary" -k informative
govctl clause edit <RFC-ID>:C-SUMMARY text --stdin <<'EOF'
Brief overview of what this RFC specifies and why.

**Scope:** What is covered and what is not.

**Rationale:** Why this specification is needed.
EOF
```

### Normative Clause Template

```bash
govctl clause new <RFC-ID>:C-<NAME> "<Title>" -s "Specification" -k normative
govctl clause edit <RFC-ID>:C-<NAME> text --stdin <<'EOF'
The system MUST ...
The system SHOULD ...
The system MAY ...

**Rationale:**
Why this requirement exists.
EOF
```

## Writing Rules

### Authority Test

Before adding a requirement to an RFC clause, apply these checks:

- **Observable contract:** A user, script, stored artifact, integration point, or validator can observe whether the sentence is true.
- **Implementation independence:** The sentence remains valid if the implementation language, source module layout, helper names, or private data structures change.
- **Right destination:** Design rationale belongs in an ADR; task scope belongs in a Work Item; transient execution evidence belongs in loop state, round artifacts, or the final response.
- **External representation only:** CLI syntax, storage paths, schemas, and wire formats belong in RFCs only when they are part of the public or persisted contract.

Examples:

| Statement                                                                                     | Destination                       |
| --------------------------------------------------------------------------------------------- | --------------------------------- |
| `The search command MUST refresh stale derived indexes before querying.`                      | RFC                               |
| `We will use SQLite FTS5 for the derived search index because it avoids an external service.` | ADR                               |
| `changed: Update the search command to refresh stale indexes before returning results`        | Work Item                         |
| `Implement this in sync_search_index with manifest rows.`                                     | Work Item or code review, not RFC |

### RFC 2119 Keywords

Use these keywords in ALL CAPS in normative clauses:

| Keyword    | Meaning                        |
| ---------- | ------------------------------ |
| MUST       | Absolute requirement           |
| MUST NOT   | Absolute prohibition           |
| SHOULD     | Recommended but not required   |
| SHOULD NOT | Discouraged but not prohibited |
| MAY        | Optional                       |

### Quality Checklist

- **Be specific.** Avoid vague terms: "appropriate", "reasonable", "as needed". Say exactly what.
- **Include rationale.** Every normative clause should explain _why_, not just _what_.
- **One requirement per sentence.** Don't chain MUST/SHOULD in a single sentence.
- **Reference existing artifacts.** Use `[[RFC-NNNN]]` or `[[ADR-NNNN]]` syntax.
- **Testable.** Each MUST/SHOULD should be verifiable — if you can't test it, rewrite it.
- **Tagged.** If the project has `[tags]` configured, tag the RFC with relevant domain tags.
- **Stay implementation-agnostic.** Describe externally relevant behavior or constraints, not language-specific type layouts or private code structure.

### What Belongs in an RFC

- Externally observable behavior
- Validation and error semantics
- Lifecycle and compatibility rules
- External schemas, protocol fields, storage formats, or CLI surface that users/scripts depend on

### What Does Not Belong in an RFC

- Rust/TypeScript/Python type declarations
- Private struct field lists or enum variant names
- Function signatures, module layout, helper names
- Work-item execution plans, step sequencing, or progress-log notes

Only include representation details when they are themselves the external contract.

### Clause Naming

- Use `C-` prefix followed by a descriptive uppercase name with hyphens
- Good: `C-VALIDATION`, `C-ERROR-FORMAT`, `C-WORK-DEF`
- Bad: `C-1`, `C-Misc`, `C-stuff`

### Section Types

| Section       | Clause Kind | Content                      |
| ------------- | ----------- | ---------------------------- |
| Summary       | informative | Overview, scope, rationale   |
| Specification | normative   | MUST/SHOULD/MAY requirements |
| Rationale     | informative | Extended explanation         |

## Phase-Aware Authoring

These rules summarize [[RFC-0000:C-STATUS-LIFECYCLE]],
[[RFC-0000:C-PHASE-LIFECYCLE]], and [[RFC-0002:C-LIFECYCLE-VERBS]].

Before editing an RFC, inspect its status and phase:

- A draft RFC is published initially through finalization; do not version-bump it.
- In `spec`, RFC and Clause edits refine the current version candidate and do not require another version bump.
- In `impl`, `test`, or `stable`, edits that change RFC or Clause content create an unversioned amendment. Lifecycle-owned metadata updates performed by dedicated verbs and current-version changelog-only corrections are exempt. The `/spec` or `/gov` workflow must release qualifying amendments with a version bump before further phase progression.
- Entry from `spec` to `impl` seals the final RFC and Clause content as that version's implementation baseline.
- A deprecated RFC cannot start another version lifecycle.

This helper writes specification content but does not perform lifecycle verbs.
Use `/spec` or `/gov` for bump and advance decisions.

## Clause Version Assignment

Clause version assignment follows [[RFC-0000:C-CLAUSE-DEF]] and
[[RFC-0002:C-LIFECYCLE-VERBS]].

Clause `since` is lifecycle-owned; do not write it into clause text or try to set
it through the generic edit surface:

- A Clause created in a draft RFC remains pending until RFC finalization assigns the current version.
- A Clause created in a normative RFC already in `spec` receives the current RFC version immediately.
- A Clause created in `impl`, `test`, or `stable` remains pending until a content-changing RFC bump assigns the next version.
- A deprecated RFC does not accept new Clauses.

## Rendering Rules

The renderer auto-generates structural elements. **Do NOT include these in clause `text`:**

- Clause heading (`### [RFC-XXXX:C-NAME] Title`) — auto-generated from metadata
- `*Since: vX.Y.Z*` — auto-generated from the `since` field
- `> **Superseded by:** ...` — auto-generated from the `superseded_by` field
- `Amended: ...` — **does not exist**; do not hallucinate this

Clause text should contain only the specification prose, rationale, and `[[...]]` references.

## Common Mistakes

| Mistake                                        | Fix                                                                   |
| ---------------------------------------------- | --------------------------------------------------------------------- |
| Vague MUST: "MUST handle errors appropriately" | Specific: "MUST return a descriptive validation error to the caller"  |
| No rationale                                   | Add `**Rationale:**` section explaining why                           |
| Untestable requirement                         | Rewrite so it can be verified programmatically                        |
| Missing cross-references                       | Add `[[RFC-NNNN]]` or `[[ADR-NNNN]]` links                            |
| Rust/TS type layout in the clause              | Rewrite it as semantic requirements or an external contract           |
| Work-plan language in the clause               | Move execution details to a work item                                 |
| Design rationale in the clause                 | Move the "why this approach" discussion to an ADR                     |
| Current progress or validation output          | Move transient evidence to loop state, round artifacts, or response   |
| Including `Since:` in clause text              | Don't — the renderer adds it from the `since` field automatically     |
| Including clause heading in text               | Don't — the renderer generates `### [RFC:C-NAME] Title` from metadata |

## Validation and Handoff

- Run `govctl check` after substantive RFC edits
- Use `rfc-reviewer` before lifecycle handoff
- Use `/spec` for clarification-only or artifact-only RFC maintenance
- Use `/gov` for implementation-bearing RFC amendments
