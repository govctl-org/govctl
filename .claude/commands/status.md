---
description: Show governance status — read-only summary of RFCs, ADRs, and work items
allowed-tools: Read, Shell, Glob, Grep, LS
argument-hint: [focus-area]
---

# /status — Governance Status

Display the current governance state. Read-only, no mutations.

---

## OVERVIEW

```bash
govctl status
```

---

## DETAILED VIEWS

### RFCs

```bash
govctl rfc list
```

### ADRs

```bash
govctl adr list
```

### Work Items

```bash
# All pending (queue + active)
govctl work list pending

# All work items
govctl work list
```

---

## FOCUS AREAS

If `$ARGUMENTS` specifies a focus:

| Argument              | Action                  |
| --------------------- | ----------------------- |
| `rfc` or `rfcs`       | Show RFC list only      |
| `adr` or `adrs`       | Show ADR list only      |
| `work` or `tasks`     | Show work items only    |
| `pending` or `active` | Show pending work items |
| `<RFC-ID>`            | Read specific RFC       |
| `<ADR-ID>`            | Read specific ADR       |
| `<WI-ID>`             | Read specific work item |

---

## VALIDATION

Check for governance issues:

```bash
govctl check
```

---

## OUTPUT FORMAT

Provide a structured summary:

```
=== GOVERNANCE STATUS ===

RFCs: <count> (<normative>/<draft>/<deprecated>)
ADRs: <count> (<accepted>/<proposed>/<deprecated>)
Work Items: <active>/<queue>/<done>

Active Work:
- <WI-ID>: <title>

Recent Activity:
- <summary of recent changes>
```

**This is a read-only command. No files will be modified.**
