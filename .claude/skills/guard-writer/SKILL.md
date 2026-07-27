---
name: guard-writer
description: "Define reusable, non-interactive verification guards with stable commands and risk-matched scope"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional guard topic]"
---

# Guard Writer

Define reusable executable completion checks under
[[RFC-0000:C-GUARD-DEF]]. This helper owns guard quality; the invoking workflow
decides when a guard applies to a Work Item.

## Discovery

Inspect existing guards and effective project policy before adding another:

```bash
govctl guard list
govctl guard show <GUARD-ID>
govctl work show <WI-ID>
```

Use `govctl guard --help` and subcommand help for current creation, editing, and
deletion syntax. Inspect `gov/config.toml` for project defaults.

## Hard Stops

- Do not create a guard for a one-off diagnostic.
- Do not add a heavyweight or domain-specific guard to project defaults merely
  because it is reusable.
- Do not permanently delete a guard without explicit user authorization; hand
  the destructive operation to the invoking workflow.
- Do not use interactive commands, prompts, or TTY-dependent behavior.
- Do not use repeated waivers to conceal an over-broad default.
- Stop when the command does not provide stable evidence for a named risk.

## Definition Policy

A guard has metadata (`id`, `title`, optional artifact `refs`) and an executable
check (`command`, optional `timeout_secs`, optional output `pattern`). Let
`govctl guard` own serialization and field validation.

Choose a command that:

- runs non-interactively from the project root;
- verifies one named risk domain with a stable exit status;
- uses the narrowest reliable test, lint, schema, or validation target;
- has an intentional timeout for its expected cost; and
- needs an output pattern only when exit status cannot prove the condition.

Keep commands simple and portable within the repository's supported
environment. Reference the RFC or ADR whose requirement or decision the guard
helps verify.

When a reusable scenario has a Conformance Case, add the Guard to that Case's
`guards` field. Do not add `CONF-*` IDs to Guard `refs`; Cases own the derived
Case-to-Guard edge.

## Scope Policy

Project `verification.default_guards` are the intersection of checks required
by every Work Item. A default should be fast, stable, environment-independent,
and relevant even to documentation-only work.

Use Work Item `verification.required_guards` for reusable checks selected from
that item's changed surface, governing references, and acceptance criteria.
Full suites belong on cross-cutting items, release checks, or changes whose
blast radius cannot be covered by narrower guards.

Waivers need a specific reason and apply to the effective default plus
Work-Item guard set. Repeated waivers are evidence that the guard's scope should
be corrected.

## Quality Tests

A guard is ready when:

- its ID and metadata are clear and unique;
- its command is non-interactive, deterministic enough for a completion gate,
  and scoped to a named risk;
- timeout and pattern settings have an explicit need;
- references connect it to relevant governance;
- its placement as a default or Work Item requirement matches actual scope; and
- it does not duplicate an existing guard without a distinct purpose.

## Completion Evidence

Run `govctl check` after creating or editing a guard. This validates schema,
identity, references, and pattern syntax. Exercise the command itself when the
guard is new or materially changed, then hand VCS work to the `commit` skill.
