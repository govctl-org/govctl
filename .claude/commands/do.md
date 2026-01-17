---
description: Execute governed workflow — work item, RFC/ADR, implement, test, done
allowed-tools: Read, Write, StrReplace, Shell, Glob, Grep, LS, SemanticSearch, TodoWrite
argument-hint: <what-to-do>
---

# /do — Governed Workflow

Execute a complete, auditable workflow to do: `$ARGUMENTS`

---

## QUICK REFERENCE

```bash
# govctl commands
cargo run --quiet -- status                   # Show summary
cargo run --quiet -- list work pending        # List queue + active items
cargo run --quiet -- list rfc                 # List all RFCs
cargo run --quiet -- list adr                 # List all ADRs
cargo run --quiet -- new work --active "<title>"  # Create + activate work item
cargo run --quiet -- mv <WI-ID> <status>      # Transition (queue|active|done|cancelled)
cargo run --quiet -- new rfc "<title>"        # Create RFC (auto-assigns ID)
cargo run --quiet -- new adr "<title>"        # Create ADR
cargo run --quiet -- check                    # Validate everything
cargo run --quiet -- render                   # Render to markdown

# Checklist management
cargo run --quiet -- add <WI-ID> acceptance_criteria "Criterion text"
cargo run --quiet -- tick <WI-ID> acceptance_criteria "pattern" -s done

# Multi-line input
cargo run --quiet -- edit <clause-id> --stdin <<'EOF'
multi-line text here
EOF
```

---

## CRITICAL RULES

1. **All governance operations MUST use `govctl` CLI** — never edit governed files directly
2. **Proceed autonomously** unless you hit a blocking condition (see ERROR HANDLING)
3. **Phase discipline** — follow `spec → impl → test → stable` for RFC-governed work
4. **RFC supremacy** — behavioral changes must be grounded in RFCs
5. **RFC advancement requires permission** — see RFC ADVANCEMENT GATE below

---

## RFC ADVANCEMENT GATE

**Default behavior:** Ask for human permission before:
- `govctl finalize <RFC-ID> normative`
- `govctl advance <RFC-ID> <phase>`

**Override:** If `$ARGUMENTS` contains phrases like:
- "free", "autonomous", "all allowed", "no permission needed", "full authority"

Then RFC advancement may proceed without asking.

**Rationale:** RFC status/phase changes are significant governance actions. They should not happen silently unless explicitly authorized.

---

## PHASE 0: INITIALIZATION

### 0.1 Validate Environment

```bash
cargo run --quiet -- status

# Detect VCS (run once, use throughout)
if jj status >/dev/null 2>&1; then
    VCS="jj"
    echo "Using jujutsu"
else
    VCS="git"
    echo "Using git"
fi
```

**VCS commands (use detected VCS throughout):**

| Action | jj | git |
|--------|-----|-----|
| Simple commit | `jj commit -m "<msg>"` | `git add . && git commit -m "<msg>"` |
| Multi-line | `jj describe --stdin <<'EOF' ... EOF && jj new` | See CONVENTIONS section |

### 0.2 Classify the Target

Parse `$ARGUMENTS` and classify:

| Type | Examples | Workflow |
|------|----------|----------|
| **Doc-only** | README, comments, typos | Fast path (skip Phase 2) |
| **Bug fix** | Existing behavior broken | May skip RFC creation |
| **Feature** | New capability | Full workflow with RFC |
| **Refactor** | Internal restructure | ADR recommended |

**Fast path for doc-only changes:** Skip to Phase 1, then directly to Phase 3 (implementation). No RFC/ADR required.

---

## PHASE 1: WORK ITEM MANAGEMENT

### 1.1 Check Existing Work Items

```bash
cargo run --quiet -- list work pending
```

**Decision:**
- Active item matches → use it, proceed to Phase 2
- Queued item matches → `cargo run --quiet -- mv <WI-ID> active`
- No match → create new

### 1.2 Create New Work Item

```bash
# Create and activate in one command
cargo run --quiet -- new work --active "<concise-title>"
```

### 1.3 Add Acceptance Criteria

**Important:** Work items cannot be marked done without acceptance criteria.

```bash
cargo run --quiet -- add <WI-ID> acceptance_criteria "First criterion"
cargo run --quiet -- add <WI-ID> acceptance_criteria "Second criterion"
```

### 1.4 Record

```bash
jj commit -m "chore(work): activate <WI-ID> for <brief-description>"
```

---

## PHASE 2: GOVERNANCE ANALYSIS

> **Skip this phase** for doc-only changes (README, comments, typos).

### 2.1 Survey Existing Governance

```bash
cargo run --quiet -- list rfc
cargo run --quiet -- list adr
```

### 2.2 Determine Requirements

| Situation | Action |
|-----------|--------|
| New feature not covered by RFC | Create RFC |
| Ambiguous RFC interpretation | Create ADR |
| Architectural decision | Create ADR |
| Pure implementation of existing RFC | Proceed to Phase 3 |

### 2.3 Create RFC (if needed)

```bash
# Create RFC (auto-assigns next ID, or use --id RFC-NNNN)
cargo run --quiet -- new rfc "<title>"

# Add clauses
cargo run --quiet -- new clause <RFC-ID>:<CLAUSE-ID> "<title>" -s "Specification" -k normative

# Edit clause text via stdin
cargo run --quiet -- edit <RFC-ID>:<CLAUSE-ID> --stdin <<'EOF'
The system MUST...
EOF
```

### 2.4 Create ADR (if needed)

```bash
cargo run --quiet -- new adr "<title>"
```

### 2.5 Link to Work Item

```bash
cargo run --quiet -- add <WI-ID> refs <RFC-ID>
```

### 2.6 Record

```bash
jj commit -m "docs(rfc): draft <RFC-ID> for <summary>"
```

---

## PHASE 3: IMPLEMENTATION

### 3.1 Gate Check (for RFC-governed work)

Before implementation, verify:
- RFC **status** is `normative` (required for production features)
- RFC **phase** is `impl` or later

```bash
# Check current state
cargo run --quiet -- list rfc
```

**Gate conditions per RFC-0001:**

| RFC Status | RFC Phase | Action |
|------------|-----------|--------|
| draft | spec | **ASK PERMISSION** → Finalize → advance → implement |
| normative | spec | **ASK PERMISSION** → Advance → implement |
| normative | impl+ | Proceed directly |
| deprecated | any | ❌ No new implementation allowed |

**If permission granted (or override in $ARGUMENTS):**
```bash
cargo run --quiet -- finalize <RFC-ID> normative  # if draft
cargo run --quiet -- advance <RFC-ID> impl        # if spec phase
```

### 3.2 Implement

1. Write code following RFC clauses (if applicable)
2. Keep changes focused — one logical change per commit
3. Run validations after substantive changes:
   ```bash
   just pre-commit
   cargo run --quiet -- check
   ```

### 3.3 Record

```bash
jj commit -m "feat(<scope>): <description>"
```

---

## PHASE 4: TESTING

> **For doc-only changes:** Run `cargo test` to verify no regressions, but skip RFC phase advancement.

### 4.1 Advance Phase (if RFC exists)

**ASK PERMISSION** before advancing (unless override in $ARGUMENTS):
```bash
cargo run --quiet -- advance <RFC-ID> test
```

### 4.2 Run Tests

```bash
cargo test
```

If tests fail, fix implementation and re-run. Do not proceed until green.

### 4.3 Record

```bash
jj commit -m "test(<scope>): add tests for <feature>"
```

---

## PHASE 5: COMPLETION

### 5.1 Final Validation

```bash
just pre-commit
cargo run --quiet -- check
cargo test
```

### 5.2 Advance RFC to Stable (if applicable)

If RFC exists and all tests pass, **ASK PERMISSION** before advancing (unless override in $ARGUMENTS):
```bash
cargo run --quiet -- advance <RFC-ID> stable
```

### 5.3 Tick Acceptance Criteria

**Pre-flight:** Verify acceptance criteria were added in Phase 1. If missing, add now:
```bash
cargo run --quiet -- add <WI-ID> acceptance_criteria "criterion"
```

Then tick each completed criterion:
```bash
cargo run --quiet -- tick <WI-ID> acceptance_criteria "criterion" -s done
```

### 5.4 Mark Work Item Done

```bash
cargo run --quiet -- mv <WI-ID> done
```

### 5.5 Record

```bash
jj commit -m "chore(work): complete <WI-ID> — <summary>"
```

### 5.6 Summary Report

```
=== WORKFLOW COMPLETE ===

Target: $ARGUMENTS
Work Item: <WI-ID>
Status: done

Governance: <RFC/ADR list or "none">
Files modified: <count>

All validations passed.
```

---

## ERROR HANDLING

### When to Stop and Ask

1. **Ambiguous requirements** — cannot determine actionable items
2. **RFC conflict** — implementation conflicts with normative RFC
3. **Breaking change** — would break existing behavior
4. **Security concern** — credentials, secrets, sensitive data
5. **Scope explosion** — task grew beyond reasonable bounds

For all other errors: **fix and continue**.

### Recovery

| Error | Recovery |
|-------|----------|
| `check` fails | Read diagnostics, fix, retry |
| `cargo test` fails | Debug, fix, retry |
| `pre-commit` fails | Usually auto-fixes; re-run |
| `mv done` rejected | Add/tick acceptance criteria first |

---

## CONVENTIONS

### Commit Messages

| Prefix | Usage |
|--------|-------|
| `feat(scope)` | New feature |
| `fix(scope)` | Bug fix |
| `docs(scope)` | Documentation |
| `test(scope)` | Tests |
| `refactor(scope)` | Restructuring |
| `chore(scope)` | Maintenance |

### Multi-line Input

**govctl:** Use `--stdin` with heredoc:

```bash
cargo run --quiet -- edit <clause-id> --stdin <<'EOF'
Multi-line content here.
EOF
```

### Multi-line Commits

**jujutsu:** Use `jj describe` then `jj new`:

```bash
# Describe current change, then create new empty change
jj describe --stdin <<'EOF'
feat(scope): summary

- Detail one
- Detail two

Refs: RFC-0010
EOF
jj new
```

**git:** Must use `cat` heredoc (no native stdin support):

```bash
git add . && git commit -m "$(cat <<'EOF'
feat(scope): summary

- Detail one
- Detail two
EOF
)"
```

**Key:** Always use `<<'EOF'` (quoted) to prevent variable expansion.

---

## EXECUTION CHECKLIST

- [ ] Environment validated, VCS detected
- [ ] Work item active with acceptance criteria
- [ ] Governance analysis (skip for doc-only)
- [ ] Implementation complete
- [ ] Tests passing
- [ ] Acceptance criteria ticked
- [ ] Work item marked done
- [ ] Summary reported

**BEGIN EXECUTION NOW.**
