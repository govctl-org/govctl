---
name: commit
description: "Record a focused VCS commit with govctl validation and Work Item traceability"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional commit message hint]"
---

# Commit

Record the current coherent change without inventing governance work or
rewriting unrelated files.

## Operational Baseline

### Discovery

Detect Jujutsu first:

```bash
jj root
```

If it succeeds, use only `jj`. A colocated repository also contains `.git`, so
probing Git would select the wrong VCS. Only when `jj root` fails, use
`git rev-parse --git-dir`.

Treat the repository as governed when `gov/config.toml` or `gov/` exists. In the
govctl repository itself, use `cargo run --quiet --` for governance commands.
Inspect current changes with `jj status` and `jj diff`, or with the corresponding
Git commands after Git has been selected:

```bash
git status --short
git diff
git diff --cached
```

Use the selected VCS `--help` when additional syntax is needed.

### Hard Stops

- This skill is the only workflow that issues raw commit commands.
- Do not create or reactivate a Work Item solely to make a commit.
- Do not perform RFC or ADR lifecycle transitions here.
- Do not commit governed changes while `govctl check` fails.
- Do not absorb, revert, or reformat unrelated user changes.
- Stop when substantive implementation has no matching active or completed Work
  Item and is not limited to spec-only governance maintenance.

## Decision Policy

### Establish Traceability

For governed repositories, run `govctl check` and inspect active Work Items with
`govctl work list active`. Use the matching active or completed item already
visible in task context or the diff; query `govctl work list done` only when that
context does not identify it. Spec-only changes may be committed without a Work
Item.

Before committing a matching active Work Item:

- tick only criteria actually satisfied by the diff;
- move it to `done` only when all criteria and effective guards pass; and
- add a note only for a durable constraint or retry rule, never for progress,
  validation output, review state, or the fact that a commit was made.

An implementation Work Item may already be `done` before its final commit. Keep
the closure change and implementation in the same coherent commit.

### Define The Commit Boundary

Review the full diff, including generated and governance files. The commit must
represent one explainable outcome. Leave unrelated work untouched. If unrelated
work shares the current Jujutsu change, stop before describing it and use
`jj split --help` or user guidance to establish a coherent change boundary.
For Git, inspect the index before committing and stop if it already contains
unrelated staged changes; staging reviewed paths does not remove them.

Use:

```text
<type>(<area>): <short summary>
```

Choose `feat`, `fix`, `refactor`, `docs`, `test`, or `chore` from the delivered
outcome, not from individual file types.

### Record

For Jujutsu, describe the current change and then create a new empty change:

```bash
jj describe -m "<type>(<area>): <summary>"
jj new
```

For Git, stage the reviewed change and commit it:

```bash
git add -- <reviewed-path>...
git commit -m "<type>(<area>): <summary>"
```

Use stdin for a multi-line commit message. Do not open an interactive VCS
editor.

## Completion Evidence

Report:

- commit ID and subject;
- matching Work Item status, when applicable;
- `govctl check` result for governed repositories; and
- whether the post-commit working copy is clean.
