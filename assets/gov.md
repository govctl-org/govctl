---
description: Execute governed workflow — work item, RFC/ADR, implement, test, done
allowed-tools: Read, Write, StrReplace, Shell, Glob, Grep, LS, SemanticSearch, TodoWrite
argument-hint: <what-to-do>
---

# /gov — Governed Workflow

Execute a complete, auditable workflow to do: `$ARGUMENTS`

---

## QUICK REFERENCE

```bash
# {{GOVCTL}} commands
{{GOVCTL}} status                             # Show summary
{{GOVCTL}} work list pending                  # List queue + active items
{{GOVCTL}} rfc list                           # List all RFCs
{{GOVCTL}} adr list                           # List all ADRs
{{GOVCTL}} work new --active "<title>"        # Create + activate work item
{{GOVCTL}} work move <WI-ID> <status>         # Transition (queue|active|done|cancelled)
{{GOVCTL}} rfc new "<title>"                  # Create RFC (auto-assigns ID)
{{GOVCTL}} adr new "<title>"                  # Create ADR
{{GOVCTL}} check                              # Validate everything
{{GOVCTL}} render                             # Render to markdown
{{GOVCTL}} render changelog                   # Generate CHANGELOG.md
{{GOVCTL}} release <version>                  # Cut a release (e.g., 1.0.0)

# Checklist management (with changelog category prefixes)
{{GOVCTL}} work add <WI-ID> acceptance_criteria "add: New feature"    # → Added section
{{GOVCTL}} work add <WI-ID> acceptance_criteria "fix: Bug fixed"      # → Fixed section
{{GOVCTL}} work add <WI-ID> acceptance_criteria "chore: Tests pass"   # → excluded from changelog
{{GOVCTL}} work tick <WI-ID> acceptance_criteria "pattern" -s done

# Multi-line input
{{GOVCTL}} clause edit <clause-id> --stdin <<'EOF'
multi-line text here
EOF
```

---

## CRITICAL RULES

1. **All governance operations MUST use `{{GOVCTL}}` CLI** — never edit governed files directly
2. **Proceed autonomously** unless you hit a blocking condition (see ERROR HANDLING)
3. **Phase discipline** — follow `spec → impl → test → stable` for RFC-governed work
4. **RFC supremacy** — behavioral changes must be grounded in RFCs
5. **RFC advancement requires permission** — see RFC ADVANCEMENT GATE below
6. **Reference format in code** — when referencing artifacts in source code comments, use `[[artifact-id]]` syntax (e.g., `// Implements [[RFC-0001:C-FOO]]`) to enable validation by `{{GOVCTL}} check`

---

## RFC ADVANCEMENT GATE

**Default behavior:** Ask for human permission before:

- `{{GOVCTL}} rfc finalize <RFC-ID> normative`
- `{{GOVCTL}} rfc advance <RFC-ID> <phase>`

**Override:** If `$ARGUMENTS` contains phrases like:

- "free", "autonomous", "all allowed", "no permission needed", "full authority"

Then RFC advancement may proceed without asking.

**Rationale:** RFC status/phase changes are significant governance actions. They should not happen silently unless explicitly authorized.

---

## PHASE 0: INITIALIZATION

### 0.1 Validate Environment

```bash
{{GOVCTL}} status
```

**Detect VCS:** Try `jj status` first. If it succeeds, use jujutsu. Otherwise use git. Error if neither works.

### 0.2 Read Project Configuration

Read the governance config to understand project-specific settings. Use the Read tool or your IDE to view `gov/config.toml`.

Key settings to note:

- `source_scan.pattern` — the `[[...]]` pattern used for inline artifact references
- Output directories for rendered artifacts
- Any project-specific overrides

**VCS commands (use detected VCS throughout):**

| Action        | jj                                              | git                                  |
| ------------- | ----------------------------------------------- | ------------------------------------ |
| Simple commit | `jj commit -m "<msg>"`                          | `git add . && git commit -m "<msg>"` |
| Multi-line    | `jj describe --stdin <<'EOF' ... EOF && jj new` | See CONVENTIONS section              |

### 0.3 Classify the Target

Parse `$ARGUMENTS` and classify:

| Type         | Examples                 | Workflow                 |
| ------------ | ------------------------ | ------------------------ |
| **Doc-only** | README, comments, typos  | Fast path (skip Phase 2) |
| **Bug fix**  | Existing behavior broken | May skip RFC creation    |
| **Feature**  | New capability           | Full workflow with RFC   |
| **Refactor** | Internal restructure     | ADR recommended          |

**Fast path for doc-only changes:** Skip to Phase 1, then directly to Phase 3 (implementation). No RFC/ADR required.

---

## PHASE 1: WORK ITEM MANAGEMENT

### 1.1 Check Existing Work Items

```bash
{{GOVCTL}} work list pending
```

**Decision:**

- Active item matches → use it, proceed to Phase 2
- Queued item matches → `{{GOVCTL}} work move <WI-ID> active`
- No match → create new

### 1.2 Create New Work Item

```bash
# Create and activate in one command
{{GOVCTL}} work new --active "<concise-title>"
```

### 1.3 Add Acceptance Criteria

**Important:** Work items cannot be marked done without acceptance criteria.

**Category prefixes** (for changelog generation per ADR-0012/ADR-0013):

| Prefix        | Changelog Section | Notes                        |
| ------------- | ----------------- | ---------------------------- |
| `add:`        | Added             | New features                 |
| `changed:`    | Changed           |                              |
| `deprecated:` | Deprecated        |                              |
| `removed:`    | Removed           |                              |
| `fix:`        | Fixed             |                              |
| `security:`   | Security          |                              |
| `chore:`      | _(excluded)_      | Internal tasks, test passing |

**Note:** Explicit category is **required**. Use prefix syntax (`add: ...`) or `--category` option.

```bash
{{GOVCTL}} work add <WI-ID> acceptance_criteria "add: Implement feature X"
{{GOVCTL}} work add <WI-ID> acceptance_criteria "fix: Memory leak resolved"
{{GOVCTL}} work add <WI-ID> acceptance_criteria "chore: All tests pass"  # won't appear in changelog
```

### 1.4 Record

```bash
# jj
jj commit -m "chore(work): activate <WI-ID> for <brief-description>"

# git
git add . && git commit -m "chore(work): activate <WI-ID> for <brief-description>"
```

---

## PHASE 2: GOVERNANCE ANALYSIS

> **Skip this phase** for doc-only changes (README, comments, typos).

### 2.1 Survey Existing Governance

```bash
{{GOVCTL}} rfc list
{{GOVCTL}} adr list
```

### 2.2 Determine Requirements

| Situation                           | Action             |
| ----------------------------------- | ------------------ |
| New feature not covered by RFC      | Create RFC         |
| Ambiguous RFC interpretation        | Create ADR         |
| Architectural decision              | Create ADR         |
| Pure implementation of existing RFC | Proceed to Phase 3 |

### 2.3 Create RFC (if needed)

```bash
# Create RFC (auto-assigns next ID, or use --id RFC-NNNN)
{{GOVCTL}} rfc new "<title>"

# Add clauses
{{GOVCTL}} clause new <RFC-ID>:<CLAUSE-ID> "<title>" -s "Specification" -k normative

# Edit clause text via stdin
{{GOVCTL}} clause edit <RFC-ID>:<CLAUSE-ID> --stdin <<'EOF'
The system MUST...
EOF
```

### 2.4 Create ADR (if needed)

```bash
{{GOVCTL}} adr new "<title>"
```

### 2.5 Link to Work Item

```bash
{{GOVCTL}} work add <WI-ID> refs <RFC-ID>
```

### 2.6 Record

```bash
# jj
jj commit -m "docs(rfc): draft <RFC-ID> for <summary>"

# git
git add . && git commit -m "docs(rfc): draft <RFC-ID> for <summary>"
```

---

## PHASE 3: IMPLEMENTATION

### 3.1 Gate Check (for RFC-governed work)

Before implementation, verify:

- RFC **status** is `normative` (required for production features)
- RFC **phase** is `impl` or later

```bash
# Check current state
{{GOVCTL}} rfc list
```

**Gate conditions:**

| RFC Status | RFC Phase | Action                                              |
| ---------- | --------- | --------------------------------------------------- |
| draft      | spec      | **ASK PERMISSION** → Finalize → advance → implement |
| normative  | spec      | **ASK PERMISSION** → Advance → implement            |
| normative  | impl+     | Proceed directly                                    |
| deprecated | any       | ❌ No new implementation allowed                    |

**If permission granted (or override in $ARGUMENTS):**

```bash
{{GOVCTL}} rfc finalize <RFC-ID> normative  # if draft
{{GOVCTL}} rfc advance <RFC-ID> impl        # if spec phase
```

**Amending normative RFCs during implementation:**

Per [[ADR-0016]], normative RFCs MAY be amended during implementation. Amendments MUST bump version and add changelog entry:

```bash
# Edit clause content
{{GOVCTL}} clause edit <RFC-ID>:<CLAUSE-ID> --stdin <<'EOF'
Updated specification text.
EOF
```

### 3.2 Implement

1. Write code following RFC clauses (if applicable)
2. **Reference artifacts in comments** — use `[[artifact-id]]` syntax:
   ```rust
   // Implements [[RFC-0001:C-VALIDATION]]
   fn validate() { ... }
   ```
3. Keep changes focused — one logical change per commit
4. Run validations after substantive changes:
   ```bash
   # Run your project's lint/format checks
   {{GOVCTL}} check
   ```

### 3.3 Record

```bash
# jj
jj commit -m "feat(<scope>): <description>"

# git
git add . && git commit -m "feat(<scope>): <description>"
```

---

## PHASE 4: TESTING

> **For doc-only changes:** Run tests to verify no regressions, but skip RFC phase advancement.

### 4.1 Advance Phase (if RFC exists)

**ASK PERMISSION** before advancing (unless override in $ARGUMENTS):

```bash
{{GOVCTL}} rfc advance <RFC-ID> test
```

### 4.2 Run Tests

```bash
# Run your project's test command
```

If tests fail, fix implementation and re-run. Do not proceed until green.

### 4.3 Record

```bash
# jj
jj commit -m "test(<scope>): add tests for <feature>"

# git
git add . && git commit -m "test(<scope>): add tests for <feature>"
```

---

## PHASE 5: COMPLETION

### 5.1 Final Validation

```bash
# Run your project's full validation suite
{{GOVCTL}} check
```

### 5.2 Advance RFC to Stable (if applicable)

If RFC exists and all tests pass, **ASK PERMISSION** before advancing (unless override in $ARGUMENTS):

```bash
{{GOVCTL}} rfc advance <RFC-ID> stable
```

### 5.3 Tick Acceptance Criteria

**Pre-flight:** Verify acceptance criteria were added in Phase 1. If missing, add now:

```bash
{{GOVCTL}} work add <WI-ID> acceptance_criteria "add: Feature implemented"
```

Then tick each completed criterion:

```bash
{{GOVCTL}} work tick <WI-ID> acceptance_criteria "Feature implemented" -s done
```

### 5.4 Mark Work Item Done

```bash
{{GOVCTL}} work move <WI-ID> done
```

### 5.5 Record

```bash
# jj
jj commit -m "chore(work): complete <WI-ID> — <summary>"

# git
git add . && git commit -m "chore(work): complete <WI-ID> — <summary>"
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

| Error                    | Recovery                           |
| ------------------------ | ---------------------------------- |
| `{{GOVCTL}} check` fails | Read diagnostics, fix, retry       |
| Tests fail               | Debug, fix, retry                  |
| Lint/format fails        | Usually auto-fixes; re-run         |
| `mv done` rejected       | Add/tick acceptance criteria first |

---

## CONVENTIONS

### Content Field Formatting

When editing content fields (description, context, decision, consequences, notes), use proper markdown:

**Code and technical terms:** Wrap in backticks

```
# Good
description = "Add `expand_inline_refs()` function using `Vec<String>`"

# Bad - will cause mdbook warnings
description = "Add expand_inline_refs() function using Vec<String>"
```

**Artifact references:** Use `[[artifact-id]]` syntax for inline references

```
# Good - expands to clickable link when rendered
context = "Per [[RFC-0000]], all work items must have acceptance criteria."

# Also good for clauses
decision = "Follow [[RFC-0000:C-WORK-DEF]] requirements."

# Bad - plain text, not linked
context = "Per RFC-0000, all work items must have acceptance criteria."
```

The `[[...]]` pattern is automatically expanded to markdown links during `{{GOVCTL}} render`.

**Source code comments:** Use `[[artifact-id]]` in code comments to enable validation:

```rust
// Implements [[RFC-0001:C-VALIDATION]]
fn validate_input(data: &str) -> Result<()> { ... }

// Per [[ADR-0005]], we use streaming instead of buffering
fn process_stream() { ... }
```

`{{GOVCTL}} check` validates these references exist and warns if they're deprecated/superseded.

**Note:** The `refs` field uses plain artifact IDs (not `[[...]]` syntax):

```bash
{{GOVCTL}} work add <WI-ID> refs RFC-0000      # Correct
{{GOVCTL}} work add <WI-ID> refs "[[RFC-0000]]" # Wrong
```

### Commit Messages

| Prefix            | Usage         |
| ----------------- | ------------- |
| `feat(scope)`     | New feature   |
| `fix(scope)`      | Bug fix       |
| `docs(scope)`     | Documentation |
| `test(scope)`     | Tests         |
| `refactor(scope)` | Restructuring |
| `chore(scope)`    | Maintenance   |

### Multi-line Input

**{{GOVCTL}}:** Use `--stdin` with input redirection.

**bash/zsh (macOS/Linux):**

```bash
{{GOVCTL}} clause edit <clause-id> --stdin <<'EOF'
Multi-line content here.
EOF
```

**PowerShell (Windows):**

```powershell
@"
Multi-line content here.
"@ | {{GOVCTL}} clause edit <clause-id> --stdin
```

### Multi-line Commits

**jujutsu (bash/zsh):**

```bash
jj describe --stdin <<'EOF'
feat(scope): summary

- Detail one
- Detail two
EOF
jj new
```

**jujutsu (PowerShell):**

```powershell
@"
feat(scope): summary

- Detail one
- Detail two
"@ | jj describe --stdin
jj new
```

**git (bash/zsh):**

```bash
git add . && git commit -m "$(cat <<'EOF'
feat(scope): summary

- Detail one
- Detail two
EOF
)"
```

**git (PowerShell):**

```powershell
git add .
git commit -m @"
feat(scope): summary

- Detail one
- Detail two
"@
```

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
