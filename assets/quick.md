---
description: Fast path for trivial changes — skip governance, minimal ceremony
allowed-tools: Read, Write, StrReplace, Shell, Glob, Grep, LS, SemanticSearch, TodoWrite
argument-hint: <what-to-do>
---

# /quick — Fast Path Workflow

Execute a lightweight workflow for trivial changes: `$ARGUMENTS`

**Use for:** Documentation fixes, typos, comments, small refactors, non-behavioral changes.

**Do NOT use for:** New features, behavioral changes, anything requiring RFC/ADR.

---

## WORKFLOW

### 1. Validate Environment

```bash
{{GOVCTL}} status

# Detect VCS
if jj status >/dev/null 2>&1; then VCS="jj"
elif git rev-parse --git-dir >/dev/null 2>&1; then VCS="git"
else echo "Error: not in a VCS repository" >&2; exit 1; fi
```

### 2. Create Work Item

```bash
{{GOVCTL}} work new --active "<concise-title>"
{{GOVCTL}} work add <WI-ID> acceptance_criteria "chore: Change completed"
```

### 3. Implement

Make the changes. If referencing governance artifacts in code comments, use `[[artifact-id]]` syntax:

```rust
// Implements [[RFC-0001:C-FOO]]
```

Run validations:

```bash
{{GOVCTL}} check
```

### 4. Record

```bash
# jj
jj commit -m "<type>(<scope>): <description>"

# git
git add . && git commit -m "<type>(<scope>): <description>"
```

### 5. Complete

```bash
{{GOVCTL}} work tick <WI-ID> acceptance_criteria "Change completed" -s done
{{GOVCTL}} work move <WI-ID> done
```

### 6. Final Record

```bash
# jj
jj commit -m "chore(work): complete <WI-ID>"

# git
git add . && git commit -m "chore(work): complete <WI-ID>"
```

---

## WHEN TO SWITCH TO /gov

If during implementation you discover:

- This requires behavioral changes → switch to `/gov`
- This needs RFC specification → switch to `/gov`
- This is an architectural decision → switch to `/gov`

**BEGIN EXECUTION NOW.**
