---
name: guard-writer
description: "Write well-structured Verification Guards. Use when: (1) Creating a new guard, (2) Editing guard check commands or patterns, (3) User mentions guard, verification, or check"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional guard topic]"
---

# Guard Writer

Write Verification Guards — reusable executable completion checks per [[RFC-0000:C-GUARD-DEF]].

## Invocation Mode

This helper skill may be used standalone or by `/gov` or `/spec`.
It is responsible for guard definition and validation, not for deciding which workflow should execute the guard in day-to-day implementation.

## Quick Reference

```bash
govctl guard new "<title>"
govctl guard list
govctl guard show GUARD-ID
govctl guard edit GUARD-ID command --set "new command"
govctl guard edit GUARD-ID timeout_secs --set 600
govctl guard edit GUARD-ID pattern --set "regex pattern"
govctl guard edit GUARD-ID refs --add RFC-NNNN
govctl guard delete GUARD-ID
```

## Guard Structure

Every guard is a TOML file under `gov/guard/` with two sections:

### `[govctl]` — Metadata

| Field   | Required | Description                          |
| ------- | -------- | ------------------------------------ |
| `id`    | yes      | Unique ID, format `GUARD-UPPER-CASE` |
| `title` | yes      | Human-readable description           |
| `refs`  | no       | Array of artifact references         |

### `[check]` — Execution

| Field          | Required | Description                                     |
| -------------- | -------- | ----------------------------------------------- |
| `command`      | yes      | Shell command, runs from project root           |
| `timeout_secs` | no       | Max seconds before failure (default: 300)       |
| `pattern`      | no       | Regex matched case-insensitively against output |

## Examples

### Scoped test guard

```toml
#:schema ../schema/guard.schema.json

[govctl]
id = "GUARD-LIFECYCLE-TESTS"
title = "lifecycle tests pass"
refs = ["RFC-0000", "RFC-0001"]

[check]
command = "cargo test --test lifecycle_tests"
timeout_secs = 300
```

### Guard with output pattern

```toml
#:schema ../schema/guard.schema.json

[govctl]
id = "GUARD-NO-FIXME"
title = "No FIXME comments in source"

[check]
command = "! grep -r FIXME src/"
pattern = "^$"
```

### Lint guard

```toml
#:schema ../schema/guard.schema.json

[govctl]
id = "GUARD-CLIPPY"
title = "clippy passes with no warnings"
refs = ["RFC-0000"]

[check]
command = "cargo clippy --all-targets -- -D warnings"
timeout_secs = 300
```

## Writing Guidelines

1. **ID format**: `GUARD-` prefix followed by uppercase alphanumeric with hyphens
2. **Commands must be non-interactive**: No prompts, no TTY requirements
3. **Commands run from project root**: Use relative paths accordingly
4. **Keep commands simple**: Prefer single commands; use `bash -c '...'` for pipelines
5. **Set timeouts intentionally**: Long builds may need more than the 300s default
6. **Use `pattern` sparingly**: Only when exit code alone is insufficient
7. **Add `refs`**: Link guards to the RFCs/ADRs they verify
8. **Verify one risk domain**: Prefer the narrowest stable command that proves a
   named concern, such as lifecycle tests, schema tests, or CLI parsing tests
9. **Keep aggregate suites available but opt-in**: A full test or lint suite is
   appropriate for cross-cutting Work Items, release checks, and CI; its
   reusability alone does not make it a project default

## Choosing Scope

Treat project defaults as the intersection of checks required by every Work
Item, not the union of every check the project can run.

A guard belongs in `verification.default_guards` only when all of these are
true:

- Every Work Item, including documentation-only work, needs the check.
- The command is fast enough to run at every completion gate.
- The result is stable and independent of optional services or environments.
- A narrower Work Item selection would not preserve useful time.

Put other reusable checks on affected Work Items through
`verification.required_guards`. Select guards from the Work Item's changed
surface, governing references, and acceptance criteria. Use a full-suite guard
only for shared infrastructure, cross-domain behavior, or another change whose
blast radius cannot be covered by narrower guards.

Do not create a guard for a one-off diagnostic command. Run that command during
implementation instead. Create a guard when the check is stable and likely to
be reused as a completion requirement.

## Integration with Work Items

Guards can be required by work items and by project-level config:

```toml
# In gov/config.toml — only checks required by every work item
[verification]
enabled = true
default_guards = ["GUARD-GOVCTL-CHECK"]

# In a cross-cutting work item — additional checks selected for its risk
[verification]
required_guards = ["GUARD-CARGO-TEST"]
```

Work items can waive guards with a reason:

```toml
[[verification.waivers]]
guard = "GUARD-CARGO-TEST"
reason = "Temporarily unavailable runner dependency; tracked in issue #123"
```

Do not use repeated waivers to compensate for an over-broad project default.
Remove that guard from `default_guards` and require it only on affected Work
Items.

## Validation

After creating or editing a guard, validate:

```bash
govctl check
```

This verifies:

- Guard schema conformance
- Unique guard IDs
- Valid regex patterns
- All referenced guard IDs in config and work items resolve

If the guard should be committed, hand off to `/commit`.
