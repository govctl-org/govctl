---
name: guard-writer
description: "Write well-structured Verification Guards. Use when: (1) Creating a new guard, (2) Editing guard check commands or patterns, (3) User mentions guard, verification, or check"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: [optional guard topic]
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
govctl guard set GUARD-ID command "new command"
govctl guard set GUARD-ID timeout_secs 600
govctl guard set GUARD-ID pattern "regex pattern"
govctl guard add GUARD-ID refs RFC-NNNN
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

### Basic test guard

```toml
#:schema ../schema/guard.schema.json

[govctl]
id = "GUARD-CARGO-TEST"
title = "cargo test passes"
refs = ["RFC-0000", "RFC-0001"]

[check]
command = "cargo test"
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

## Integration with Work Items

Guards can be required by work items and by project-level config:

```toml
# In gov/config.toml — applies to all work items
[verification]
enabled = true
default_guards = ["GUARD-GOVCTL-CHECK", "GUARD-CARGO-TEST"]

# In a work item — additional guards for that item
[verification]
required_guards = ["GUARD-CLIPPY"]
```

Work items can waive guards with a reason:

```toml
[[verification.waivers]]
guard = "GUARD-CARGO-TEST"
reason = "Documentation-only change, no code modified"
```

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
