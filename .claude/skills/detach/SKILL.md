---
name: detach
description: "Remove project-local govctl integration through an ownership-aware dry run, archival plan, explicit confirmation, and recoverable edits"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional scope hint]"
---

# Detach Govctl

Stop using govctl in a project while preserving its governance history and
unrelated user configuration. Detachment is destructive integration cleanup,
not artifact deletion.

## Read-Only Inventory

Do not mutate anything during discovery. Inspect:

- `gov/config.toml` and the full `gov/` tree;
- configured documentation, source-scan, and agent-asset paths;
- project-local skills and reviewer agents attributable to govctl;
- hooks, plugin metadata, project instructions, and editor configuration that
  invoke or require govctl;
- `[[RFC-...]]`, `[[ADR-...]]`, `[[WI-...]]`, and other governed references in
  source and configuration; and
- the logical-to-resolved mapping for every candidate path, using symlink
  metadata to inspect each existing ancestor without assuming containment; and
- existing `gov.archived/`, working-copy changes, ignore rules, and destination
  collisions.

Use structured parsers for structured configuration. Use `rg` for reference and
text discovery. Treat generated projections under configured documentation
paths as historical output and preserve them by default.

Ownership must be established before removal. A matching filename or a mention
of `govctl` is evidence to inspect, not proof that an entire file or directory
is govctl-owned. When ownership is ambiguous, preserve the content and list it
for manual review.

## Hard Stops

- Present a file-level mutation plan and obtain explicit confirmation before
  changing anything. Approval applies only to that stated plan.
- Never delete the authoritative `gov/` tree; archive it.
- Stop on an existing archive destination instead of overwriting, merging, or
  inventing a new destination without user agreement.
- Require `gov/`, its archive destination, and their existing ancestors to be
  non-symlink paths contained by the project root.
- Do not remove a shared skills, agents, hooks, plugin, editor, or instruction
  directory as a unit unless the inventory proves the directory is wholly
  govctl-owned.
- Default all other mutations to resolved targets contained by the project
  root. A symlinked or external target requires separate exact authorization
  after showing its logical path, resolved target, and symlinked ancestors.
- Do not discard unrelated working-copy changes or non-govctl configuration.
- Do not strip plain historical prose merely because it mentions an RFC, ADR,
  Work Item, or govctl.
- After archiving `gov/`, do not invoke project-scoped govctl commands unless
  the archive is first restored.
- Stop on the first unexpected mutation failure and report completed and pending
  operations; do not continue into a less recoverable partial state.

## Mutation Plan

The dry-run report must identify:

- the exact logical and resolved archive source and proposed destination;
- every file to remove;
- every structured configuration entry or text span to edit;
- references that will be removed, retained as useful archival prose, or left
  for manual review;
- generated documentation and unrelated assets that will remain untouched;
- existing changes that overlap the plan;
- a recoverable baseline for every edited or removed file; and
- restoration steps derived from those baselines and the proposed operations.

Resolve destination collisions and ambiguous ownership before asking for final
confirmation. Re-run the inventory if the user changes the scope.

## Execution Policy

Apply the confirmed plan in a recoverable order:

1. Preserve the authoritative governance tree by renaming the confirmed,
   non-symlinked `gov/` path to the confirmed non-conflicting, non-symlinked
   archive path.
2. Remove only verified project-local govctl assets. In shared directories,
   remove individual owned entries and retain the container.
3. Edit hooks and structured configuration surgically, preserving unrelated
   entries and valid file structure.
4. Remove governed reference markup from source only where the remaining
   comment stays accurate. Delete a reference-only comment when it has no other
   meaning; preserve useful historical context as plain prose when appropriate.
5. Remove govctl-specific instruction sections without altering neighboring
   project policy.
6. Clean up a directory only when it is empty and was part of the confirmed
   plan.

Do not automatically rewrite ignore policy. Report archive tracking or ignore
implications and apply a change only when it was included in the confirmed
plan.

## Recovery

Keep an operation record in the final response, not in the archived governed
artifacts. If an edit fails, preserve the archive and successfully completed
changes, stop, and explain the exact restoration or continuation steps. Never
claim atomic detachment when the filesystem operations were not transactional.

Restoration starts by resolving any new `gov/` collision, moving the confirmed
archive back to `gov/`, and reinstalling project-local assets only where they
were removed. Derive the remaining restoration steps from the recorded plan
rather than promising a fixed one-command undo.

## Completion Evidence

Detachment is complete when:

- the governance tree exists at the confirmed archive path;
- no active project hook, instruction, or verified govctl-owned local asset
  still activates the integration;
- source edits preserve code behavior and meaningful comments;
- configured generated documentation and unrelated user assets remain
  untouched unless the user explicitly included them;
- a final scan reports remaining ambiguous or intentionally preserved mentions;
  and
- the final report lists archived, removed, edited, preserved, and unresolved
  paths plus restoration guidance.

Use `commit` if the user wants the confirmed detachment recorded in VCS.
