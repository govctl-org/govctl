---
name: detach
description: "Remove project-local govctl integration through an ownership-aware dry run, archival plan, explicit confirmation, and recoverable edits"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional scope hint]"
---

# Detach Govctl

Remove govctl integration from a project while archiving its governance history
and keeping unrelated configuration.

## Inventory (Read-Only)

Change nothing yet. Inspect `gov/` and its config; configured docs, source-scan,
and agent-asset paths; local skills and agents; hooks, plugin metadata,
instructions, and editor config that invoke govctl; `[[...]]` references in
source and config; existing `gov.archived/`, working-copy changes, ignore rules,
and destination collisions. For every candidate path, record its logical and
resolved path and check each existing ancestor for symlinks.

Use parsers for structured config and `rg` for text. Keep generated docs by
default. A filename match or mention of govctl is a lead, not proof of
ownership; when ownership is unclear, keep the content and list it for manual
review.

## Hard Stops

- Get explicit confirmation of a file-level plan before any change. Approval
  covers only that plan.
- Archive `gov/`; never delete it. If the destination exists, stop rather than
  overwrite, merge, or choose another path.
- `gov/`, the archive destination, and their ancestors must be non-symlink
  paths inside the project root.
- Other changes must target resolved paths inside the project root. A symlinked
  or external target needs its own approval after showing its logical path,
  resolved target, and symlinked ancestors.
- Never remove a shared skills, agents, hooks, plugin, editor, or instruction
  directory unless govctl provably owns all of it.
- Keep unrelated working-copy changes, non-govctl config, and plain historical
  prose that merely mentions governance.
- After archiving, run no project-scoped govctl command until the archive is
  restored.
- Stop at the first unexpected failure and report what completed and what is
  pending.

## Dry-Run Plan

List the archive source and destination (logical and resolved), every file to
remove, every config entry or text span to edit, references to remove or keep
or leave for review, what stays untouched, overlapping existing changes, a
recoverable baseline for each touched file, and restoration steps. Resolve
collisions and unclear ownership before asking for confirmation; redo the
inventory if scope changes.

## Execution

1. Rename `gov/` to the confirmed archive path.
2. Remove only verified govctl assets; in shared directories remove the owned
   entries and keep the directory.
3. Edit hooks and config surgically, keeping unrelated entries valid.
4. Remove reference markup only where the remaining comment stays accurate;
   delete reference-only comments and keep useful history as plain prose.
5. Remove govctl instruction sections without touching neighboring policy.
6. Delete a directory only if it is empty and in the plan.

Change ignore rules only if the plan includes it; otherwise just report the
implications.

## Recovery

Record operations in the final response, not in the archive. On failure, keep
the archive and completed changes, stop, and give exact restore or continue
steps. The operations are not atomic; do not claim they are. To restore,
resolve any new `gov/` collision, move the archive back, reinstall removed
assets, and follow the remaining steps from the plan.

## Done When

- `gov/` exists at the confirmed archive path;
- no hook, instruction, or verified govctl asset still activates the
  integration;
- source edits keep behavior and meaningful comments, and unrelated assets are
  untouched;
- a final scan lists remaining unclear or intentionally kept mentions; and
- the report lists archived, removed, edited, kept, and unresolved paths plus
  restoration steps.

Use `commit` if the user wants the detach recorded.
