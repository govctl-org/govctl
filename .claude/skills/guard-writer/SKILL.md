---
name: guard-writer
description: "Define reusable, non-interactive verification guards with stable commands and risk-matched scope"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional guard topic]"
---

# Guard Writer

Define reusable executable completion checks under [[RFC-0000:C-GUARD-DEF]].
This helper owns guard quality; the invoking workflow decides which Work Items
use a guard.

## Discovery

```bash
govctl guard list
govctl guard show <GUARD-ID>
govctl work show <WI-ID>
```

Check `gov/config.toml` for project defaults. Use `govctl <resource> --help`
for current syntax.

## Hard Stops

- Do not create a guard for a one-off diagnostic.
- Do not add a heavy or domain-specific guard to project defaults just because
  it is reusable.
- Do not delete a guard without explicit user authorization; hand deletion to
  the invoking workflow.
- No interactive commands, prompts, or TTY-dependent behavior.
- Stop when the command gives no stable evidence for a named risk.

## Writing A Guard

A guard has `id`, `title`, optional `refs`, a `command`, optional
`timeout_secs`, and an optional output `pattern`. Let `govctl guard` handle
serialization and field validation.

The command should:

- run non-interactively from the project root;
- check one named risk with a stable exit status;
- use the narrowest reliable test, lint, schema, or validation target;
- set a timeout that fits its expected cost; and
- use a `pattern` only when exit status cannot prove the condition.

Keep commands simple and portable. Reference the RFC or ADR the guard helps
verify. For a reusable scenario with a Conformance Case, add the guard to that
Case's `guards` field; never put `CONF-*` IDs in guard `refs`.

## Placement

- `verification.default_guards` are the intersection of checks every Work Item
  needs: fast, stable, environment-independent, and relevant even to
  documentation-only work.
- A Work Item's `verification.required_guards` add reusable checks chosen from
  its changed surface, references, and criteria.
- Full suites belong on cross-cutting items, release checks, or changes no
  narrower guard can cover.
- A waiver needs a specific reason and applies to the effective default plus
  item guard set. Repeated waivers mean the guard's scope is wrong; fix the
  scope instead of waiving again.

## Checklist

- The ID and metadata are clear and unique, and no existing guard already
  serves the same purpose.
- Timeout and pattern are present only when needed.
- Placement as default or required matches actual scope.
- `govctl check` passes; it validates schema, identity, references, and
  pattern syntax.
- The command has been run once when the guard is new or materially changed.

Hand VCS work to the `commit` skill.
