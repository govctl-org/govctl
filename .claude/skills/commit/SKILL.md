---
name: commit
description: "Commit changes with govctl integration — check work item status, preserve durable notes only when needed, and run govctl check"
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, TodoWrite
argument-hint: "[optional commit message hint]"
---

# /commit — Commit with Govctl Integration

Commit changes using the project's version control system, with govctl-aware checks.

**Outputs:** Recorded VCS commit, updated work-item memory when applicable, and a clean post-commit working copy.

---

## WORKFLOW

**CRITICAL: Steps MUST be executed in exact order. Do NOT skip ahead.**

1. This is the only workflow that should issue raw `jj` or `git` commit commands.
2. Do not perform RFC/ADR lifecycle verbs here, except work-item tick/move updates and rare durable notes that belong to commit bookkeeping.
3. Implementation-bearing commits should remain traceable to the matching work item, whether it is active or done. Never create or reactivate a work item solely to make a commit.
4. Spec-only governance commits may proceed without a work item only when the diff is limited to governance artifacts, rendered governance docs, embedded skill/agent templates, or related metadata.

### Step 1: Detect VCS and Governance

**1a. Detect VCS.** Run `jj root` first. If it succeeds, use **Jujutsu** — do NOT also check git. A jj-git colocated repo has both `.jj/` and `.git/`, so checking git would also succeed and cause you to use the wrong VCS. Only if `jj root` fails, run `git rev-parse --git-dir`. If that succeeds, use **Git**. If both fail, stop and inform user.

**CRITICAL:** Do NOT run `jj root` and `git rev-parse` in parallel. Run `jj root` first, and only proceed to git detection if jj is not found.

**1b. Detect governance.** Check whether the current repository is governed by govctl:

```bash
test -f gov/config.toml || test -d gov
```

- **If governed** (exit 0): continue with Steps 2–7 as written below.
- **If not governed** (exit 1): skip all govctl-specific steps (Steps 2–3 and 7). Proceed directly to Step 4 (Inspect Changes), then Step 5 (Compose Message), then Step 6 (Execute Commit) using plain VCS commands. Omit work-item tracking and `govctl check` entirely. Mention in your summary that no govctl governance was detected.

### Step 2: Govctl Pre-Commit Checks

Before committing, run govctl checks:

```bash
govctl check
```

If check fails, stop. Show the diagnostics, fix the issue, and rerun `govctl check` before committing.

### Step 3: Work Item Traceability Check

Check for queued and active work items, then inspect any changed or already-known completed work item that may match the diff:

```bash
govctl work list pending
```

If no matching pending item is found and the completed item is not already known from the changed files or current task context, inspect completed items:

```bash
govctl work list done
```

**If an active work item exists:**

1. Determine whether the active work item actually matches this diff.
2. If the diff is spec-only governance maintenance unrelated to the active work item, the commit may remain work-item-free even while another work item is active.
3. If the active work item applies, then:
   - Ask whether any closure-worthy durable `notes` should be recorded; skip notes for progress, validation output, review status, next actions, temporary blockers, or TODOs
   - Check whether any acceptance criteria can be ticked:
     ```bash
     govctl work show <WI-ID>
     ```
     If criteria match the completed work, suggest ticking them:
     ```bash
     govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done
     ```
4. If the active work item does not apply and the diff is not spec-only, look for the active or done work item that governed the implementation

**If no active work item exists:**

- If the changes are spec-only governance maintenance, a work item is optional. Typical examples:
  - `gov/**` artifact files
  - rendered governance docs under `docs/rfc/**` or `docs/adr/**`
  - embedded skill or agent templates under `.claude/**`
  - supporting metadata such as `CLAUDE.md`, `build.rs`, or `src/cmd/new.rs` that only wire governance assets
- If a matching done work item governed the implementation, keep the commit traceable to it; the final implementation commit may include that work item's closure
- Otherwise, report that the implementation was performed outside tracked work and ask how the user wants it recorded
- Never create or reactivate a work item solely to make the commit. If substantive implementation remains, establish an active work item before continuing that implementation

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

### Step 6: Finalize Work Item

Before invoking the commit command, if a matching work item exists:

1. **Add notes only when there is a closure-worthy durable constraint, retry rule, or learning to preserve after the work item is done**:

   ```bash
   govctl work add <WI-ID> notes "Do not retry parser path X; it cannot preserve normalized arrays"
   ```

   Do not add notes for commands run, tests passed, review findings addressed, current plans, next actions, or temporary blockers.

2. **Tick acceptance criteria** when the completed work satisfies them.
3. **Move the work item to done only when all criteria are satisfied**. Otherwise, keep it active.

Include these finalization changes in the same commit as the implementation. Do not update the work item after committing solely to record closure.

### Step 7: Execute Commit

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

---

## QUICK REFERENCE

```bash
# Govctl checks
govctl check                    # Validate all artifacts
govctl work list pending        # List queued and active work items
govctl work show <WI-ID>        # Show work item details
govctl work tick <WI-ID> acceptance_criteria "<pattern>" -s done

# VCS commands
jj status                       # Jujutsu
jj diff --stat
git status                      # Git
git diff --stat
```

---

## COMMON SCENARIOS

### Scenario 1: Matching Work Item

```text
1. Detect the matching active or done WI-XXXX
2. govctl check → passes
3. Confirm the WI actually matches this diff
4. If yes: ask about closure-worthy durable notes and criterion ticks
5. If no, but diff is spec-only: proceed without attaching the commit to that WI
6. If all criteria are satisfied, move the WI to done before the final implementation commit
7. Commit the implementation and WI closure together
```

### Scenario 2: No Active Work Item

```text
1. No pending work items
2. govctl check → passes
3. Check whether a matching done work item governed the implementation
   - If yes: keep the commit traceable to it and include any closure changes in the same commit
4. Otherwise, check whether the diff is spec-only governance maintenance
   - If yes: proceed with a spec-only commit and mention no WI was used
   - If no: report the untracked implementation; do not create or reactivate a WI solely for the commit
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
