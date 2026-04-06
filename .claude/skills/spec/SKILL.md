---
name: spec
description: "Maintain governance artifacts without implementation work. Use when: (1) Accepting or refining ADRs, (2) Clarifying or amending RFCs without code changes, (3) Governance-only docs/check/render updates"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: <artifact-maintenance-task>
---

# /spec - Governance Artifact Maintenance

Maintain governance artifacts for: `$ARGUMENTS`

Use this workflow for spec-only governance work: refine or accept ADRs, clarify or amend RFCs, update artifact references, and validate/render governance output without implementing code.

**Outputs:** Updated governance artifacts, completed artifact review, and validated rendered governance state.

**Artifact roles:** RFCs define obligations, ADRs explain decisions, and work items track execution. `/spec` only maintains the first two.

## Critical Rules

1. Artifact-only scope. Do not write implementation code in this workflow.
2. No work item by default. This workflow is for governance maintenance, not implementation tracking.
3. Ask permission before lifecycle-owned verbs: `govctl adr accept`, `govctl adr reject`, `govctl adr supersede`, `govctl rfc finalize`, and `govctl rfc bump`.
4. Do not advance RFC phase out of `spec` here. Hand off to `/gov` when implementation-bearing work begins.
5. Clarification-only RFC updates must not silently change behavior. If behavior changes, stop and route through `/discuss` and `/gov`.
6. Never edit governed files directly. Use `govctl` verbs only.
7. Validate with `govctl check`, and run `govctl render` when rendered docs should change.
8. Use `/commit` for raw VCS operations. This workflow defines what to record, not how to invoke VCS directly.
9. Do not let RFCs absorb implementation structure or let ADRs absorb work-item execution details. Preserve artifact roles while editing.

## Quick Reference

```bash
govctl status
govctl rfc list
govctl rfc show <RFC-ID>
govctl adr list
govctl adr show <ADR-ID>

govctl adr set <ADR-ID> context --stdin <<'EOF' ... EOF
govctl adr set <ADR-ID> decision --stdin <<'EOF' ... EOF
govctl adr set <ADR-ID> selected_option "Option A"
govctl adr edit <ADR-ID> content.consequences.positive --add "Benefit"
govctl adr edit <ADR-ID> content.consequences.negative --add "Trade-off"
govctl adr add <ADR-ID> alternatives "Rejected option"
govctl adr accept <ADR-ID>

govctl clause edit <RFC-ID>:C-<NAME> --stdin <<'EOF' ... EOF
govctl rfc bump <RFC-ID> --patch -m "Clarify clause wording"
govctl rfc finalize <RFC-ID> normative

govctl check
govctl render
```

## Workflow

### 1. Classify the task

Choose the narrowest fit:

- **ADR maintenance**: refine a proposed ADR, add alternatives/refs, or accept it
- **RFC clarification**: tighten wording or fill specification gaps without changing behavior
- **RFC amendment**: change normative requirements without implementing them yet
- **Governance-only cleanup**: fix artifact references, rendering output, or metadata

If the task requires implementation, testing implementation behavior, or phase advancement beyond `spec`, stop and use `/gov`.

### 2. Gather context

```bash
govctl status
govctl rfc list
govctl adr list
```

Then read the relevant artifacts:

```bash
govctl rfc show <RFC-ID>
govctl adr show <ADR-ID>
```

For artifact editing conventions, follow the appropriate writer skill:

- RFC changes -> **rfc-writer**
- ADR changes -> **adr-writer**

### 3. Edit the artifacts

Use `govctl` verbs only.

For ADR work:

- Refine `context`, `decision`, `consequences`, `alternatives`, and `refs`
- If the ADR is ready to become authoritative, ask permission before `govctl adr accept <ADR-ID>`

For RFC work:

- Edit clauses with `govctl clause edit`
- If the RFC is draft and ready to become normative, ask permission before `govctl rfc finalize <RFC-ID> normative`
- If amending an existing normative RFC, ask permission before `govctl rfc bump`

Semver guidance for RFC amendments:

- `--patch`: clarification or wording fix with no behavioral change
- `--minor`: additive requirement or newly specified behavior
- `--major`: breaking or incompatible requirement change

Every RFC bump must include a changelog summary via `-m`.

### 4. Review and validate

Run the appropriate reviewer before finalizing artifact state:

- RFC changes -> **rfc-reviewer**
- ADR changes -> **adr-reviewer**

Then validate:

```bash
govctl check
govctl render
```

Fix validation or reviewer issues before recording the result.

### 5. Record the result

Spec-only governance commits may be recorded without a work item.

Use commit types that reflect artifact maintenance:

- `docs(rfc)`: RFC drafting, clarification, or amendment
- `docs(adr)`: ADR drafting or acceptance preparation
- `chore(gov)`: governance metadata, refs, render output, or config cleanup

Use `/commit` to record those changes.

If the task grows into implementation work, stop here and hand off to `/gov`.

## Handoff Rules

- Use `/discuss` when the design itself is still unresolved
- Use `/gov` when code or tests must change
- Use `/quick` only for standalone non-behavioral cleanup outside governance artifacts

## Examples

### Accept a reviewed ADR

1. Read the ADR with `govctl adr show <ADR-ID>`
2. Run **adr-reviewer**
3. Fix issues
4. Ask permission, then run `govctl adr accept <ADR-ID>`
5. Run `govctl check`

### Clarify an RFC without changing behavior

1. Edit the clause text with `govctl clause edit`
2. Run **rfc-reviewer**
3. Ask permission, then run `govctl rfc bump <RFC-ID> --patch -m "Clarify wording"`
4. Run `govctl check` and `govctl render`

### Prepare a deprecation plan without implementation

1. Update the RFC language to mark the behavior deprecated
2. Record the rationale and migration guidance
3. Review and validate the artifact changes
4. Hand off to `/gov` for actual implementation or removal work
