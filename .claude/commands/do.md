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
# VCS commit (detect once, use throughout)
VCS_CMD=$(jj --version >/dev/null 2>&1 && jj status >/dev/null 2>&1 && echo "jj commit -m" || echo "git add . && git commit -m")

# govctl commands
cargo run -- status                          # Show summary
cargo run -- list work pending               # List queue + active items
cargo run -- list rfc                        # List all RFCs
cargo run -- list adr                        # List all ADRs
cargo run -- new work "<title>"              # Create work item
cargo run -- move <file> <status>            # Transition work item (queue|active|done|cancelled)
cargo run -- new rfc <RFC-ID> "<title>"      # Create RFC
cargo run -- new adr "<title>"               # Create ADR
cargo run -- check                           # Validate everything
cargo run -- render                          # Render to markdown

# Multi-line input (see MULTI-LINE INPUT HANDLING section)
cargo run -- edit <clause-id> --stdin <<'EOF'
multi-line text here
EOF

# Checklist management
cargo run -- add <WI-ID> acceptance_criteria "Criterion text"
cargo run -- tick <WI-ID> acceptance_criteria "Criterion" done
```

---

## CRITICAL RULES

1. **All governance operations MUST use `govctl` CLI** — never edit governed files directly
2. **Never stop execution** unless you genuinely cannot proceed without human decision
3. **Record every step** using version control (see VCS_CMD above)
4. **Phase discipline is absolute** — follow `spec → impl → test → stable`
5. **RFC supremacy** — all behavior must be grounded in RFCs

---

## PHASE 0: INITIALIZATION

### 0.1 Validate Environment

```bash
# Check govctl is available
cargo run --quiet -- status

# Detect version control system (set VCS_CMD for entire workflow)
if jj --version >/dev/null 2>&1 && jj status >/dev/null 2>&1; then
    VCS_TYPE="jj"
    VCS_COMMIT="jj commit -m"
    echo "Using jujutsu for version control"
else
    VCS_TYPE="git"
    VCS_COMMIT="git add . && git commit -m"
    echo "Using git for version control"
fi
```

**Store VCS_TYPE and VCS_COMMIT for the entire workflow. Use consistently.**

### 0.2 Understand the Target

Parse `$ARGUMENTS` and determine:

1. **What** is being requested (feature, fix, refactor, documentation, etc.)
2. **Scope** estimation (trivial, small, medium, large)
3. **Domain** affected (governance, CLI, schema, docs, etc.)

Create a mental model of the target before proceeding.

---

## PHASE 1: WORK ITEM MANAGEMENT

### 1.1 Check Existing Work Items

```bash
# List all pending work items (queue + active)
cargo run -- list work pending
```

**Decision Tree:**

- IF an **active** work item matches the target → **Use that work item, proceed to Phase 2**
- IF a **queued** work item matches the target → **Move to active, proceed to Phase 2**
- IF no matching work item exists → **Create new work item**

### 1.2 Move Queued Item to Active (if applicable)

```bash
# Move from queue to active
cargo run -- move worklogs/items/<filename>.toml active
```

**Record:** `chore(work): activate work item <WI-ID> for <brief-description>`

### 1.3 Create New Work Item (if needed)

```bash
# Create new work item
cargo run -- new work "<concise-title-describing-target>"
```

Then immediately move to active:

```bash
cargo run -- move worklogs/items/<new-file>.toml active
```

**Record:** `chore(work): create and activate <WI-ID> for <brief-description>`

### 1.4 Update Work Item Description

Use `govctl set` to update the description with clear acceptance criteria:

```bash
cargo run -- set <WI-ID> content.description "<detailed-description-with-acceptance-criteria>"
```

---

## PHASE 2: GOVERNANCE ANALYSIS

### 2.1 Survey Current RFCs

```bash
# List all RFCs with their status and phase
cargo run -- list rfc
```

Read relevant RFC markdown files to understand existing specifications.

### 2.2 Survey Current ADRs

```bash
# List all ADRs
cargo run -- list adr
```

### 2.3 Determine Governance Requirements

**Decision Matrix:**

| Situation                                     | Action Required                         |
| --------------------------------------------- | --------------------------------------- |
| New feature not covered by any RFC            | Create new RFC (draft, spec phase)      |
| Existing RFC covers feature but is ambiguous  | Create ADR to document interpretation   |
| Existing RFC conflicts with requirement       | Create RFC amendment or superseding RFC |
| Architectural decision not documented         | Create new ADR                          |
| Pure implementation of existing normative RFC | Proceed directly to Phase 3             |

### 2.4 Create RFC if Required

```bash
# Create new RFC
cargo run -- new rfc <RFC-ID> "<title>"

# Add clauses as needed
cargo run -- new clause <RFC-ID>:<CLAUSE-ID> "<clause-title>" -s "Specification" -k normative

# Edit clause text
cargo run -- edit <RFC-ID>:<CLAUSE-ID> --text "<normative-text>"
```

### 2.5 Create ADR if Required

```bash
# Create new ADR
cargo run -- new adr "<title>"

# Edit ADR fields
cargo run -- set <ADR-ID> content.context "<context-text>"
cargo run -- set <ADR-ID> content.decision "<decision-text>"
cargo run -- set <ADR-ID> content.consequences "<consequences-text>"
```

### 2.6 Self-Review Loop for RFC/ADR

For each new/amended RFC or ADR, execute this review loop:

```
REPEAT until stable:
  1. Run validation: `cargo run -- check`
  2. Read the rendered output: `cargo run -- render`
  3. Evaluate against these criteria:
     - Is it unambiguous? (no room for interpretation)
     - Is it complete? (covers all edge cases)
     - Is it minimal? (no unnecessary complexity)
     - Is it testable? (clear conformance criteria)
  4. If any criterion fails → edit and repeat
  5. If all criteria pass → break loop
```

**Record (RFC):** `docs(rfc): draft <RFC-ID> for <feature-summary>`
**Record (ADR):** `docs(adr): propose <ADR-ID> for <decision-summary>`

### 2.7 Link Work Item to Governance Artifacts

```bash
cargo run -- add <WI-ID> refs <RFC-ID>
cargo run -- add <WI-ID> refs <ADR-ID>
```

---

## PHASE 3: IMPLEMENTATION

> **GATE CHECK:** Implementation MUST NOT begin until:
>
> - Work item is in `active` status
> - Any required RFC is at least `draft` status (for experimental work) or `normative` (for production features)
> - RFC is in `impl` phase or later

### 3.1 Verify Phase Gate

```bash
cargo run -- check
```

If RFC phase is still `spec`, advance it:

```bash
# Only after RFC is finalized as normative
cargo run -- finalize <RFC-ID> normative
cargo run -- advance <RFC-ID> impl
```

### 3.2 Implement Changes

Follow RFC specifications exactly. For each implementation unit:

1. **Write code** following RFC clauses
2. **Add inline comments** citing clause IDs where behavior is specified
3. **Keep changes focused** — one logical change per commit

**Record (feature):** `feat(<scope>): implement <clause-id> — <brief-description>`
**Record (fix):** `fix(<scope>): correct <issue> per <clause-id>`

### 3.3 Self-Verify Implementation

After implementation:

1. Run lints and format checks:

   ```bash
   just pre-commit
   ```

2. Run govctl validation:

   ```bash
   cargo run -- check
   ```

3. Verify no RFC deviations

---

## PHASE 4: TESTING

### 4.1 Advance to Test Phase

```bash
cargo run -- advance <RFC-ID> test
```

### 4.2 Write Tests

For each normative clause:

1. Create test case that verifies clause behavior
2. Include both positive and negative cases
3. Document which clause each test validates

### 4.3 Run Tests

```bash
cargo test
```

**Record:** `test(<scope>): add tests for <RFC-ID> conformance`

### 4.4 Iterate Until All Tests Pass

If tests fail:

1. Fix implementation (not the spec, unless spec is wrong)
2. Re-run tests
3. Repeat until green

---

## PHASE 5: STABILIZATION

### 5.1 Final Validation

```bash
cargo run -- check
just pre-commit
cargo test
```

### 5.2 Advance RFC to Stable (if applicable)

```bash
cargo run -- advance <RFC-ID> stable
```

### 5.3 Accept ADRs

```bash
cargo run -- accept <ADR-ID>
```

### 5.4 Update Documentation

Ensure all rendered documentation is up to date:

```bash
cargo run -- render
```

**Record:** `docs: finalize documentation for <feature>`

---

## PHASE 6: COMPLETION

### 6.1 Mark Work Item Done

```bash
cargo run -- move worklogs/items/<WI-file>.toml done
```

### 6.2 Final Record

**Record:** `chore(work): complete <WI-ID> — <target-summary>`

### 6.3 Summary Report

Output a summary:

```bash
=== WORKFLOW COMPLETE ===

Target: $ARGUMENTS

Work Item: <WI-ID>
Status: done

Governance Changes:
- RFC(s): <list or "none">
- ADR(s): <list or "none">

Implementation:
- Files modified: <count>
- Tests added: <count>

All phase gates passed. Ready for review.
```

---

## ERROR HANDLING

### When to Ask for Human Input

STOP and request human decision ONLY when:

1. **Ambiguous requirements** — the target cannot be parsed into actionable items
2. **RFC conflict** — implementation requirements conflict with normative RFC
3. **Breaking change** — proposed change would break existing userspace behavior
4. **Security concern** — action might expose sensitive data or credentials
5. **Irreversible action** — action cannot be undone (e.g., data deletion)
6. **Scope explosion** — task has grown beyond reasonable bounds

For all other situations: **proceed autonomously**.

### Recovery Procedures

| Error                         | Recovery                                |
| ----------------------------- | --------------------------------------- |
| `cargo run -- check` fails    | Read diagnostics, fix issues, retry     |
| `cargo test` fails            | Debug failure, fix code or test, retry  |
| `just pre-commit` fails       | Fix lints/format, retry                 |
| Work item state invalid       | Use `govctl move` to correct state      |
| RFC phase transition rejected | Check prerequisites, fulfill them first |

---

## COMMIT MESSAGE CONVENTIONS

Use conventional commits:

| Prefix            | Usage                                |
| ----------------- | ------------------------------------ |
| `feat(scope)`     | New feature implementation           |
| `fix(scope)`      | Bug fix                              |
| `docs(scope)`     | Documentation changes                |
| `test(scope)`     | Test additions/changes               |
| `refactor(scope)` | Code restructuring                   |
| `chore(scope)`    | Maintenance tasks, work item updates |

Always include relevant artifact IDs in the message body when applicable.

---

## MULTI-LINE INPUT HANDLING

Many operations require multi-line text. Use these patterns:

### govctl: Clause Text (--stdin)

```bash
# Preferred: pipe via stdin
cargo run -- edit <RFC-ID>:<CLAUSE-ID> --stdin <<'EOF'
An RFC (Request for Comments) is a formal specification document
that defines normative behavior for the system.

It MUST contain:
- A unique identifier
- Clear conformance criteria
EOF
```

### govctl: Set Field with Heredoc

```bash
# For multi-line field values
cargo run -- set <WI-ID> content.description "$(cat <<'EOF'
Implement the new validation layer.

Acceptance criteria:
1. All inputs validated against schema
2. Error messages include field path
3. No performance regression > 5%
EOF
)"
```

### jujutsu: Multi-line Commit Message

**Option 1: Using --stdin with jj describe (preferred)**

```bash
# First commit with any message, then update description via stdin
jj commit -m "placeholder"
jj describe @- --stdin <<'EOF'
feat(validator): implement schema validation

- Add JSON Schema validation for all inputs
- Include field path in error messages
- Optimize hot path for common cases

Refs: RFC-0010:C-VALIDATION
EOF
```

**Option 2: Using heredoc with -m**

```bash
jj commit -m "$(cat <<'EOF'
feat(validator): implement schema validation

- Add JSON Schema validation for all inputs
- Include field path in error messages
- Optimize hot path for common cases

Refs: RFC-0010:C-VALIDATION
EOF
)"
```

**Note:** `jj describe` has native `--stdin` support, which is cleaner than heredoc wrapping.

### git: Multi-line Commit Message

```bash
# Using heredoc
git add . && git commit -m "$(cat <<'EOF'
feat(validator): implement schema validation

- Add JSON Schema validation for all inputs
- Include field path in error messages
- Optimize hot path for common cases

Refs: RFC-0010:C-VALIDATION
EOF
)"
```

### Alternative: Temporary File

For very long content, write to a temp file first:

```bash
# Write content to temp file
cat > /tmp/clause-text.txt <<'EOF'
Long clause text here...
Multiple paragraphs...
EOF

# Use --text-file
cargo run -- edit <RFC-ID>:<CLAUSE-ID> --text-file /tmp/clause-text.txt
```

### jujutsu: Update Current Change Description

```bash
# Update description of current working copy (@) via stdin
jj describe --stdin <<'EOF'
Work in progress: implementing validation layer

TODO:
- [ ] Schema validation
- [ ] Error messages
EOF
```

**Key patterns:**

- Always use `<<'EOF'` (quoted) to prevent variable expansion in heredoc
- `jj describe --stdin` is native and preferred over heredoc wrapping with `-m`
- `govctl edit --stdin` and `govctl set --stdin` mirror this pattern

---

## VERSION CONTROL USAGE

Detected in Phase 0, use throughout:

| VCS_TYPE | Command for Recording a Step                                        |
| -------- | ------------------------------------------------------------------- |
| `jj`     | `jj commit -m "<message>"` (commits current change, starts new one) |
| `git`    | `git add . && git commit -m "<message>"`                            |

**jujutsu workflow details:**

- `jj commit -m` commits the current working-copy change with the message
- After commit, jujutsu automatically creates a new empty working-copy change
- Use `jj desc -m` only to update the description of the _current_ uncommitted change
- For fine-grained history, prefer `jj commit -m` at each logical step

**git workflow details:**

- Always stage all changes before committing
- Each `git commit` creates a new commit on the current branch

When documentation says "Record:", execute:

- **jj**: `jj commit -m "<message>"`
- **git**: `git add . && git commit -m "<message>"`

---

## EXECUTION CHECKLIST

- [ ] Environment validated
- [ ] Work item identified/created and active
- [ ] Governance analysis complete
- [ ] RFC/ADR created if needed
- [ ] RFC/ADR self-reviewed until stable
- [ ] Implementation complete per spec
- [ ] Tests written and passing
- [ ] All validations green
- [ ] Work item marked done
- [ ] Final summary reported

**BEGIN EXECUTION NOW.**
