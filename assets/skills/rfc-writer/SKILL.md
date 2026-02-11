---
name: rfc-writer
description: "Write well-structured RFCs with normative clauses. Use when: (1) Creating a new RFC, (2) Adding or editing RFC clauses, (3) User mentions RFC, specification, or normative requirements"
---

# RFC Writer

Write RFCs that are precise, complete, and follow govctl conventions.

## Quick Reference

```bash
{{GOVCTL}} rfc new "<title>"
{{GOVCTL}} clause new <RFC-ID>:C-<NAME> "<title>" -s "<section>" -k <kind>
{{GOVCTL}} clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
clause text
EOF
```

## RFC Structure

Every RFC should have:

1. **Summary clause** (informative) — what this RFC covers and why
2. **Specification clauses** (normative) — the actual requirements
3. **Rationale sections** within clauses — why each requirement exists

### Summary Clause Template

```bash
{{GOVCTL}} clause new <RFC-ID>:C-SUMMARY "Summary" -s "Summary" -k informative
{{GOVCTL}} clause edit <RFC-ID>:C-SUMMARY --stdin <<'EOF'
Brief overview of what this RFC specifies and why.

**Scope:** What is covered and what is not.

**Rationale:** Why this specification is needed.
EOF
```

### Normative Clause Template

```bash
{{GOVCTL}} clause new <RFC-ID>:C-<NAME> "<Title>" -s "Specification" -k normative
{{GOVCTL}} clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF'
The system MUST ...
The system SHOULD ...
The system MAY ...

**Rationale:**
Why this requirement exists.
EOF
```

## Writing Rules

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

## Common Mistakes

| Mistake                                        | Fix                                                                         |
| ---------------------------------------------- | --------------------------------------------------------------------------- |
| Vague MUST: "MUST handle errors appropriately" | Specific: "MUST return `Result<T, E>` with descriptive error message"       |
| No rationale                                   | Add `**Rationale:**` section explaining why                                 |
| Untestable requirement                         | Rewrite so it can be verified programmatically                              |
| Missing cross-references                       | Add `[[RFC-NNNN]]` or `[[ADR-NNNN]]` links                                  |
| `since` field left empty                       | It's OK — `govctl rfc bump` or `govctl rfc finalize` fills it automatically |
