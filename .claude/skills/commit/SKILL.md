---
name: commit
description: "Record a focused VCS commit with govctl validation and Work Item traceability"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional commit message hint]"
---

# Commit

Record the current coherent change without inventing governance work or
touching unrelated files.

## Detect The VCS

Run `jj root` first. If it succeeds, use only `jj`: a colocated repository also
contains `.git`, so probing Git first picks the wrong tool. Only if `jj root`
fails, use `git rev-parse --git-dir`.

Inspect changes with `jj status` and `jj diff`, or `git status --short`,
`git diff`, and `git diff --cached`. The repository is governed when `gov/`
exists.

## Hard Stops

- This is the only workflow that issues raw commit commands.
- Do not create or reactivate a Work Item just to make a commit.
- Do not perform RFC or ADR lifecycle transitions.
- Do not commit governed changes while `govctl check` fails.
- Do not absorb, revert, or reformat unrelated user changes.
- Stop when substantive implementation has no matching active or completed Work
  Item and is not spec-only governance maintenance.

## Traceability

In a governed repository, run `govctl check` and `govctl work list active`. Use
the Work Item already identified by context or the diff; query
`govctl work list done` only if none is. Spec-only changes need no Work Item.

For a matching active Work Item:

- tick only criteria the diff satisfies;
- move it to `done` only when all criteria and effective guards pass; and
- add a note only for a durable constraint or retry rule, never for progress,
  validation output, review state, or the commit itself.

An item may already be `done`; keep the closure and implementation in one
commit.

## Commit Boundary

Review the full diff, including generated and governance files. The commit is
one explainable outcome. If unrelated work shares the current jj change, stop
and split it (`jj split --help`) or ask the user. With Git, stop if the index
already holds unrelated staged changes.

Message format, with the type chosen from the outcome (`feat`, `fix`,
`refactor`, `docs`, `test`, `chore`):

```text
<type>(<area>): <short summary>
```

Record it without opening an editor; use stdin for multi-line messages:

```bash
jj describe -m "<type>(<area>): <summary>"
jj new
```

```bash
git add -- <reviewed-path>...
git commit -m "<type>(<area>): <summary>"
```

## Report

The commit ID and subject, the Work Item status if any, the `govctl check`
result, and whether the working copy is clean afterwards.
