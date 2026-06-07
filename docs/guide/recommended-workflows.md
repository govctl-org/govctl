# Recommended Workflows

govctl is a governance tool, not a single enforced process. The recommended
workflow is upstream-first: clarify requirements before design, design before
execution, and execution before release.

Use this page as the default operating model when you are working with an AI
agent or coordinating changes across a team.

## The Default Path

For non-trivial product work, prefer:

```text
RFC -> ADR -> Work Item -> Code -> Verification -> Release
```

Each layer has a different job:

| Artifact  | Role                        | Good content                                                               | Bad content                                                      |
| --------- | --------------------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| RFC       | Defines obligations         | Behavior, interfaces, lifecycle rules, compatibility, validation semantics | Private module layout, helper names, current implementation plan |
| ADR       | Explains decisions          | Alternatives, trade-offs, chosen approach, consequences                    | New product requirements, task checklists, progress logs         |
| Work Item | Tracks execution            | Task scope, acceptance criteria, dependencies, durable notes               | Normative behavior, design rationale, transient progress         |
| Loop      | Coordinates local execution | Round evidence, blockers, changed paths, next local action                 | Product requirements, durable design decisions                   |

When in doubt, ask what the text is trying to do:

- "What must be true?" belongs in an RFC.
- "Why this option?" belongs in an ADR.
- "What are we doing in this task?" belongs in a Work Item.
- "What happened in this round?" belongs in loop state or the final response.

## Small Changes

Not every change needs every artifact.

Use the smallest path that still leaves a useful record:

| Change                                 | Typical path                                                    |
| -------------------------------------- | --------------------------------------------------------------- |
| Typo, comment, small docs fix          | Edit directly, run `govctl check`                               |
| Bug fix for already specified behavior | Work Item -> Code -> Verification                               |
| New user-visible behavior              | RFC -> ADR if needed -> Work Item -> Code                       |
| New architecture or trade-off          | ADR -> Work Item -> Code                                        |
| Deprecation or removal                 | RFC amendment -> Work Item -> Code                              |
| Mechanical cleanup                     | No Work Item, or one coarse Work Item if the outcome is durable |

Do not create work items for helper extraction, file moves, fixture sharing,
formatting, or other low-level steps whose durable record is the diff.

## Discovery And Drafting

Use `/discuss` when the problem is still unclear. Good `/discuss` output is a
draft RFC, a proposed ADR, or a decision that no new artifact is needed.

During discovery:

- Keep RFCs focused on externally relevant obligations.
- Keep ADRs focused on trade-offs and decisions.
- Do not start implementation while the governing RFC is still ambiguous.
- Use reviewer agents before treating RFC or ADR drafts as settled.

RFC and ADR drafting should remain human-in-the-loop. Agents can produce strong
drafts, but semantic review is still necessary because `govctl check` validates
structure, not judgment.

## Upstream-First Refinement

RFCs and ADRs may change as you learn. The important rule is direction:

```text
Update upstream artifacts first, then implement downstream work.
```

If implementation reveals that a requirement is wrong, incomplete, or ambiguous,
do not silently make the code diverge. Amend the RFC or ADR first, then continue
the Work Item.

Avoid the reverse pattern:

```text
Code first -> patch RFC/ADR afterward to match what happened
```

That pattern turns governance into a log of implementation choices instead of a
source of authority.

## Work Item Execution

Use Work Items for non-trivial implementation even when no new RFC or ADR is
needed. A good Work Item says what will be completed and how closure is checked.

Before implementation:

```bash
govctl work new --active "Implement stale search index refresh"
govctl work add WI-YYYY-MM-DD-NNN refs RFC-0002
govctl work add WI-YYYY-MM-DD-NNN acceptance_criteria "changed: Search refreshes stale derived indexes before querying"
govctl work add WI-YYYY-MM-DD-NNN acceptance_criteria "chore: govctl check passes"
govctl check --has-active
```

During execution:

- Tick acceptance criteria as they become true.
- Add `notes` only for durable constraints or retry rules.
- Do not put progress updates, command output, review status, next actions, or
  temporary blockers in `notes`.

After execution:

```bash
govctl work tick WI-YYYY-MM-DD-NNN acceptance_criteria "Search refreshes" -s done
govctl work tick WI-YYYY-MM-DD-NNN acceptance_criteria "govctl check passes" -s done
govctl work move WI-YYYY-MM-DD-NNN done
```

Moving to `done` runs verification guards when verification is enabled.

## When To Use Loops

Use a loop when execution is bigger than one simple Work Item or when you need
resumable local round evidence.

Good loop use cases:

- A batch has multiple independently meaningful Work Items.
- Work Items have `depends_on` edges and need ready-item planning.
- You expect several implementation/review/verification rounds.
- You need local evidence for changed paths, blockers, note candidates, or
  validation results without polluting Work Item fields.

Do not use a loop to justify over-splitting one task into mechanical Work Items.
Do not create a separate loop for every tiny cleanup.

Typical loop flow:

```bash
govctl loop list open
govctl loop start WI-YYYY-MM-DD-001 WI-YYYY-MM-DD-002
govctl loop run LOOP-YYYY-MM-DD-NNN
# implement, verify, and fill the opened round evidence
govctl loop run LOOP-YYYY-MM-DD-NNN
```

Important boundaries:

- `loop run` advances local round state only.
- It does not implement code.
- It does not tick acceptance criteria.
- It does not add Work Item notes.
- It does not move Work Items to `done`.

If batch scope changes, keep the same loop identity:

```bash
govctl loop add LOOP-YYYY-MM-DD-NNN work WI-YYYY-MM-DD-003
govctl loop remove LOOP-YYYY-MM-DD-NNN work WI-YYYY-MM-DD-002
govctl loop replan LOOP-YYYY-MM-DD-NNN
```

Loop state is local execution memory under `.govctl/loops/`. Work Items remain
the durable task record.

## Agent Goals And Loops

Some agent runtimes provide session-level goal features, such as `/goal`. These
work well with batched Work Items in a loop when the boundaries stay clear:

| Layer      | Role                                                        |
| ---------- | ----------------------------------------------------------- |
| Work Item  | Durable outcome and acceptance criteria                     |
| Loop       | Batch coordination, ready-work planning, and round evidence |
| Agent goal | Current session focus, budget, and resume target            |

For a batched loop, set the agent goal to the current loop or round instead of
duplicating the Work Item list:

```text
Goal: Complete the current round for LOOP-YYYY-MM-DD-NNN.
```

That goal gives the agent a narrow execution target while the loop remains the
source of truth for ready Work Items, blockers, changed paths, validation
evidence, and note candidates.

Use a simpler goal for simple work:

- For one small Work Item, a goal may point directly at that Work Item.
- For a multi-Work-Item batch, prefer one goal for the active loop or current
  round.
- Do not create one goal per mechanical substep.

Do not store agent goals in RFCs, ADRs, or Work Item notes. Goals are runtime
focus. Loop state is local execution memory. Work Items are the durable task
record.

## Review And Verification

Use both structural and semantic checks:

```bash
govctl check
govctl verify
```

`govctl check` validates schemas, references, lifecycle rules, tags, source
annotations, and other deterministic constraints. It does not decide whether an
RFC is too implementation-specific or whether an ADR is intellectually honest.

Use reviewer agents for semantic checks:

- `rfc-reviewer` catches vague requirements and implementation details inside
  RFCs.
- `adr-reviewer` catches missing alternatives, dishonest consequences, and ADRs
  that invent requirements.
- `wi-reviewer` catches vague criteria, transient notes, local requirements, and
  over-split Work Items.
- `compliance-checker` audits implementation against normative RFC clauses and
  accepted ADR decisions.

Treat reviewer findings as design feedback, not just formatting feedback.

## Project-Specific Guards

Domain projects often need extra checks. For example:

- Requirement ID tracing from RFC clauses to tests and source comments.
- Generated-code or assembler inspection for low-level performance work.
- Protocol conformance suites.
- Language-specific lint or style checks.

Model these as verification guards when they can be automated:

```bash
govctl guard new "Requirement trace"
govctl guard edit GUARD-REQUIREMENT-TRACE check.command --set "cargo run --bin trace-check"
govctl work edit WI-YYYY-MM-DD-NNN verification.required_guards --add GUARD-REQUIREMENT-TRACE
```

Keep the guard domain-specific. govctl provides the governance structure; your
project owns the language, framework, and protocol-specific checks.

## Common Pitfalls

- Writing implementation details into RFCs because they are easy to describe.
- Writing new requirements into ADRs because the decision needs justification.
- Writing current plans or validation output into Work Item notes.
- Creating many tiny Work Items for one coherent refactor.
- Treating loops as mandatory ceremony instead of local execution coordination.
- Treating an agent goal as durable governance state.
- Treating `govctl check` as a substitute for human or agent semantic review.

The goal is not to maximize artifact count. The goal is to keep authority,
decision rationale, execution state, and verification evidence in the right
places.
