---
name: commit
description: "Commit changes with govctl integration — check work item status, update journal or notes, and run govctl check"
allowed-tools: Read, Write, StrReplace, Shell, Glob, Grep, LS
argument-hint: [optional commit message hint]
---

# /commit — Commit with Govctl Integration

Commit changes using the project's version control system, with govctl-aware checks.

---

## WORKFLOW

**CRITICAL: Steps MUST be executed in exact order. Do NOT skip ahead.**

### Step 1: Detect VCS

Run `jj root` first. If succeeds, use **Jujutsu**. If fails, run `git rev-parse --git-dir`. If succeeds, use **Git**. If both fail, stop and inform user.

### Step 2: Govctl Pre-Commit Checks

Before committing, run govctl checks:

```bash
govctl check
```

If check fails, stop. Show the diagnostics, fix the issue, and rerun `govctl check` before committing.

### Step 3: Work Item Status Check

Check for active work items:

```bash
govctl work list pending
```

**If active work item exists:**

1. **Journal update**: Ask user if they want to add a journal entry documenting what was done and what happened
2. **Notes update**: Ask if any learnings, constraints, or retry rules should be recorded
3. **Acceptance criteria**: Check if any criteria can be ticked:
   ```bash
   govctl work show <WI-ID>
   ```
   If criteria match the completed work, suggest ticking them:
   ```bash
   govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done
   ```

**If no active work item:**

- If the changes are spec-only governance maintenance (artifact files, rendered governance docs, or related metadata only), a work item is optional
- Otherwise, ask whether these changes should be attached to an existing work item or a new one
- In the standard govctl workflow, create or activate a work item before committing
- If the user explicitly says this commit is outside tracked work, record that exception in your summary

### Step 4: Inspect Changes

**If Jujutsu:**

```bash
jj status
jj diff --stat
```

**If Git:**

```bash
git status
git diff --stat
```

### Step 5: Compose Message

Format (mandatory):

```
<type>(<area>): <short summary>

<body (optional)>
```

| Type       | When to use        |
| ---------- | ------------------ |
| `feat`     | New feature        |
| `fix`      | Bug fix            |
| `refactor` | Code restructuring |
| `docs`     | Documentation      |
| `test`     | Tests              |
| `chore`    | Maintenance        |

If `$ARGUMENTS` provided, use as basis. Otherwise derive from diff.

### Step 6: Execute Commit

#### Jujutsu

Single-line:

```bash
jj describe -m "<type>(<area>): <summary>"
jj new
```

Multi-line:

```bash
jj describe --stdin <<'EOF'
<type>(<area>): <short summary>

<body>
EOF
jj new
```

#### Git

```bash
git add -A
git commit -m "<type>(<area>): <summary>"
```

### Step 7: Post-Commit Work Item Update

After successful commit, if work item exists:

1. **Add journal entry** (if user confirmed in Step 3):

   ```bash
   govctl work add <WI-ID> journal "Committed <summary>; govctl check passes"
   govctl work add <WI-ID> journal --scope <scope> --stdin <<'EOF'
   <multi-line progress summary>
   EOF
   ```

   Add notes only when there is a durable constraint, retry rule, or learning to preserve:

   ```bash
   govctl work add <WI-ID> notes "Do not retry X; it fails because Y"
   ```

2. **Tick acceptance criteria** (if applicable)

3. **Check if work item should be marked done**:
   - All criteria ticked? Suggest: `govctl work move <WI-ID> done`
   - Still in progress? Keep active

---

## QUICK REFERENCE

```bash
# Govctl checks
govctl check                    # Validate all artifacts
govctl work list pending        # List active work items
govctl work show <WI-ID>        # Show work item details
govctl work add <WI-ID> journal "Progress update"
govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done

# VCS commands
jj status                       # Jujutsu
jj diff --stat
git status                      # Git
git diff --stat
```

---

## COMMON SCENARIOS

### Scenario 1: Active Work Item with Progress

```
1. Detect active WI-XXXX
2. govctl check → passes
3. Ask: "Add journal entry for this progress?"
4. User confirms → add journal entry
5. Check criteria → suggest ticking completed ones
6. Commit changes
7. Ask: "Mark work item as done?" (if all criteria ticked)
```

### Scenario 2: No Active Work Item

```
1. No pending work items
2. govctl check → passes
3. Check whether the diff is spec-only governance maintenance
   - If yes: proceed with a spec-only commit and mention no WI was used
   - Otherwise: ask "Which work item should this commit belong to?"
4. If implementation work applies:
   - If existing work applies: activate or use it
   - Otherwise: create WI with `govctl work new --active`
5. Commit changes
```

### Scenario 3: Govctl Check Fails

```
1. govctl check → fails
2. Show diagnostics
3. Fix the issue
4. Rerun `govctl check`
5. Commit only after it passes
```

---

## OUTPUT

Report:

1. Commit subject line
2. Work item status updates (if any)
3. govctl check result

**BEGIN EXECUTION NOW.**
