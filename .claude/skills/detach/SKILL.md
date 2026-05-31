---
name: detach
description: "Remove govctl governance from a project. Archives artifacts, removes skills/agents, strips code references. Use when: (1) User wants to stop using govctl, (2) User mentions detach, opt-out, remove governance, or uninstall"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional scope hint]
---

# /detach — Remove govctl Governance

**Always dry run first.** Scan everything, present a report, get explicit confirmation before changing anything.

## Rules

1. Archive `gov/` → `gov.archived/` (never delete).
2. Leave `docs/rfc/`, `docs/adr/`, `docs/work/` untouched.
3. Strip `[[RFC-...]]`, `[[ADR-...]]`, `[[WI-...]]` references from source code and agent configs.
4. Do not run `govctl` commands after detaching.

---

## Phase 0: Dry Run

Scan and report:

1. **Structure** — check for `gov/`, `.claude/skills/`, `.claude/agents/`, `.claude/hooks/hooks.json`, `.claude-plugin/`, `.claude/.claude-plugin/`
2. **Source refs** — grep for `\[\[(RFC-\d{4}|ADR-\d{4}|WI-\d{4}-)` in source files (use `source_scan.include` from `gov/config.toml` if available, otherwise scan common source dirs)
3. **Agent configs** — grep `CLAUDE.md`, `AGENTS.md`, `.cursorrules` for `govctl`, `RFC-`, `ADR-`, `WI-`

Present a summary with counts and file-level details. List what will happen. **Wait for user confirmation.**

---

## Phase 1: Archive

```bash
mv gov gov.archived
```

Update any `gov/` entries in `.gitignore` to `gov.archived/` (or ask user).

---

## Phase 2: Remove Agent Integrations

Remove govctl-managed skills from `.claude/skills/`:

```
gov quick discuss spec commit migrate init detach
rfc-writer adr-writer wi-writer guard-writer decision-analysis
```

Remove govctl agents from `.claude/agents/`:

```
rfc-reviewer.md adr-reviewer.md wi-reviewer.md compliance-checker.md
```

Strip `govctl`-referencing hooks from `.claude/hooks/hooks.json` (remove the file if it becomes empty).

Remove `.claude/.claude-plugin/` and `.claude-plugin/`.

Clean up empty directories.

---

## Phase 3: Strip Source References

For each `[[...]]` reference found in source code, pick the best option:

- **Has surrounding context:** Remove the `[[...]]` only, keep the rest of the comment
- **Reference-only line:** Remove the entire comment line
- **Important cross-reference:** Convert to plain text, e.g. `See ADR-0010 (archived in gov.archived/adr/)`

When in doubt, preserve context and just remove the brackets.

Verify no `[[RFC-`, `[[ADR-`, or `[[WI-` references remain.

---

## Phase 4: Strip Agent Configs

Remove govctl-specific sections from `CLAUDE.md`, `AGENTS.md`, `.cursorrules`:

- Project rules describing govctl workflows
- `[[...]]` artifact references
- Skill invocation instructions (`/gov`, `/quick`, `/discuss`, etc.)
- govctl CLI command examples

Preserve all non-govctl content. Verify no references remain.

---

## Phase 5: Summary

Report what was archived, removed, and stripped. Suggest:

```bash
/commit chore(gov): detach from govctl governance
```

To restore: `mv gov.archived gov && govctl init-skills`
