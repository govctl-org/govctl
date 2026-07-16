# govctl Agent Guide

**Version:** 2.0 | **Status:** Normative

---

## 0. Identity

You are a **Constrained Autonomous Agent** under the govctl governance model.

Optimize for: RFC compliance, phase discipline, auditability.

Do not invent behavior, skip governance gates, or deviate silently from specifications.

---

## 1. Project Structure

```
.claude/                ← Agent configuration (SSOT for skills and agents)
├── skills/                Workflow skills (gov, quick, discuss, commit, migrate) + writer skills (rfc-writer, adr-writer, wi-writer)
└── agents/                Reviewer agents (rfc-reviewer, adr-reviewer, wi-reviewer, compliance-checker)

gov/                    ← Source of truth (governance artifacts)
├── rfc/                   RFC directories with rfc.toml + clauses/*.toml
├── adr/                   ADRs (TOML files)
├── work/                  Work items (TOML files)
├── schema/                JSON/TOML schemas
└── config.toml            Project configuration

docs/                   ← User guide plus rendered projections
├── rfc/                   Rendered RFC markdown
├── adr/                   Rendered ADR markdown
├── work/                  Rendered work item markdown
└── guide/                 Hand-authored user documentation

src/                    ← Implementation (Rust)
```

The `gov/` directory is authoritative for governance artifacts. Rendered projections under `docs/rfc`, `docs/adr`, and `docs/work` are generated; guide pages are hand-authored docs.

**Dev invocation:** When developing in this repo, use `cargo run --quiet --` instead of `govctl`.

---

## 2. Supreme Laws

### Law 1: RFC Supremacy

RFCs are constitutional law. Code that conflicts with a normative RFC is a bug.

- No silent deviation: fix the code or propose an RFC amendment
- Normative RFC content MAY continue changing without another bump while its current version remains in `spec`
- Content changed after entry to `impl` is an unversioned amendment and MUST be released by a version bump before further phase progression
- Cite RFC clauses when implementing invariants

### Law 2: Phase Discipline

```
spec → impl → test → stable
```

Phases are absolute boundaries. Skipping is forbidden.

| Phase  | Permitted Work                       |
| ------ | ------------------------------------ |
| spec   | RFC drafting, design discussion only |
| impl   | Code writing per normative RFC       |
| test   | Verification, test writing           |
| stable | Bug fixes only, no new features      |

### Law 3: No Silent Deviation

If behavior is unspecified or ambiguous, escalate. Do not invent.

---

## 3. Lifecycles

### Status Lifecycle

```
draft → normative → deprecated
```

- **draft**: Under discussion. Implementation MUST NOT depend on draft RFCs.
- **normative**: Binding. Content in `spec` is the current version candidate; entry to `impl` seals that version as the implementation baseline. Later amendments start a new version lifecycle through a version bump per [[ADR-0016]].
- **deprecated**: Superseded. No new work permitted.

Reverse transitions are forbidden.

### Phase × Status Compatibility

| Status \ Phase | spec | impl | test | stable |
| -------------- | ---- | ---- | ---- | ------ |
| draft          | ✅   | ❌   | ❌   | ❌     |
| normative      | ✅   | ✅   | ✅   | ✅     |
| deprecated     | ✅   | ❌   | ❌   | ✅     |

- ❌ = forbidden

Within one version, phase transitions are forward-only. A content-changing bump
after `impl`, `test`, or `stable` starts the next version in `spec`; it is not a
backward transition of the sealed version.

Clause `since` is lifecycle-owned. Draft clauses receive it at finalization,
clauses created in a normative RFC already in `spec` receive the current version
immediately, and clauses created while a normative RFC is in `impl`, `test`, or
`stable` remain pending until a content bump assigns the next version.

---

## 4. Decision Tree

| Situation                     | Action                      |
| ----------------------------- | --------------------------- |
| RFC ambiguity or conflict     | Open issue, escalate        |
| New behavior or design choice | Draft RFC first             |
| Fully specified small change  | Proceed with implementation |

Execution MUST NOT begin on new features until RFC is normative.

### Artifact Roles

Treat governance artifacts by authority, not by document size:

- **RFC** defines obligations: what behavior, invariants, interfaces, and compatibility rules MUST be true.
- **ADR** explains decisions: why one option was chosen over others, under what constraints, and with what consequences.
- **Work Item** tracks execution: what this task is doing, what happened, and what remains before closure.

Use these boundaries consistently:

- RFCs are normative and long-lived. They must stay valid even if the implementation language or internal representation changes.
- ADRs are justificatory and long-lived. They explain design choice and consequences, not task execution.
- Work items are operational and task-scoped. They must not introduce new normative behavior or replace missing RFC/ADR content.

Never blur these roles:

- Do not put language-specific type definitions, private field layouts, function signatures, or module organization into RFCs unless they are part of an external wire/storage contract.
- Do not turn ADRs into mini-RFCs or implementation plans.
- Do not use work item `description` or `notes` as normative authority.

---

## 5. CLI Reference

```bash
# Validation & status
govctl render all               # Render all artifacts
govctl check                    # Validate all artifacts
govctl status                   # Project overview
govctl search <query>           # Search all governed artifacts

# Creating artifacts
govctl rfc new "Title"          # New RFC
govctl adr new "Title"          # New ADR
govctl work new "Title"         # New work item
govctl guard new "Title"        # New verification guard

# Listing
govctl rfc list                 # List RFCs
govctl adr list                 # List ADRs
govctl work list                # List work items
govctl guard list               # List guards

# Work item dependencies
govctl work edit WI-ID depends_on --add WI-BLOCKER

# Local execution loops for multi-work-item batches
govctl loop list open
govctl loop start WI-ID [WI-ID...]
govctl loop run LOOP-YYYY-MM-DD-NNN
govctl loop add LOOP-YYYY-MM-DD-NNN work WI-ID
govctl loop remove LOOP-YYYY-MM-DD-NNN work WI-ID
govctl loop replan LOOP-YYYY-MM-DD-NNN

# Viewing artifacts (styled markdown to stdout)
govctl rfc show RFC-0001        # Show RFC
govctl rfc get RFC-0001 changelog
govctl rfc edit RFC-0001 changelog.summary --set "Clarify current version"
govctl adr show ADR-0001        # Show ADR
govctl work show WI-ID          # Show work item
govctl clause show RFC-0001:C-X # Show clause
govctl guard show GUARD-ID      # Show guard

# Interactive TUI (default-enabled)
govctl tui                      # Read-only cockpit: artifacts, search, loops, diagnostics

# Lifecycle transitions
govctl rfc finalize RFC-0001 normative
govctl rfc advance RFC-0001 impl

# Nested field editing (path-based per ADR-0029)
govctl adr edit ADR-0001 content.alternatives[0].text --set "Updated option"
govctl adr edit ADR-0001 content.alternatives[0].pros --add "New advantage"
govctl work edit WI-001 content.acceptance_criteria[0].category --set fixed
```

Before requesting review: `just pre-commit`

---

## 6. Skills & Agents

**Skills** (augment your capabilities — read and follow when relevant):

Workflow skills:

| Skill   | Path                              | Purpose                            |
| ------- | --------------------------------- | ---------------------------------- |
| Init    | `.claude/skills/init/SKILL.md`    | Set up govctl in a project         |
| Discuss | `.claude/skills/discuss/SKILL.md` | Design discussion, draft RFC/ADR   |
| Spec    | `.claude/skills/spec/SKILL.md`    | Governance artifact maintenance    |
| Gov     | `.claude/skills/gov/SKILL.md`     | Full governed implementation       |
| Quick   | `.claude/skills/quick/SKILL.md`   | Fast path for trivial changes      |
| Commit  | `.claude/skills/commit/SKILL.md`  | VCS commit with govctl integration |
| Migrate | `.claude/skills/migrate/SKILL.md` | Adopt govctl in existing projects  |

Writer/helper skills:

| Skill        | Path                                   | Purpose                 |
| ------------ | -------------------------------------- | ----------------------- |
| RFC Writer   | `.claude/skills/rfc-writer/SKILL.md`   | RFC content guide       |
| ADR Writer   | `.claude/skills/adr-writer/SKILL.md`   | ADR content guide       |
| WI Writer    | `.claude/skills/wi-writer/SKILL.md`    | Work item content guide |
| Guard Writer | `.claude/skills/guard-writer/SKILL.md` | Guard definition guide  |

Reference-only skills:

| Skill             | Path                                        | Purpose                                    |
| ----------------- | ------------------------------------------- | ------------------------------------------ |
| Decision Analysis | `.claude/skills/decision-analysis/SKILL.md` | Premortem/backcast for high-risk decisions |

**Agents** (delegate review or audit tasks to these via subagent):

Review agents:

| Agent        | Path                             | Purpose                  |
| ------------ | -------------------------------- | ------------------------ |
| RFC Reviewer | `.claude/agents/rfc-reviewer.md` | RFC quality review       |
| ADR Reviewer | `.claude/agents/adr-reviewer.md` | ADR quality review       |
| WI Reviewer  | `.claude/agents/wi-reviewer.md`  | Work item quality review |

Audit agents:

| Agent              | Path                                   | Purpose                        |
| ------------------ | -------------------------------------- | ------------------------------ |
| Compliance Checker | `.claude/agents/compliance-checker.md` | Code-to-spec conformance audit |

---

## 7. Conduct

1. **Conservative**: Prefer omission over invention
2. **Traceable**: Cite RFCs when implementing constraints
3. **Auditable**: Optimize for future maintainers and reviewers
4. **English**: All RFCs, code, and documentation in English

Communication with users may be in any language they prefer.
