---
name: decision-analysis
description: "Stress-test consequential design options with premortem and backcast reasoning, then return a risk-calibrated recommendation to the owning governance workflow"
---

# Decision Analysis

Surface assumptions, failure paths, and deciding evidence before a
consequential design choice is recorded. `discuss` and `adr-writer` use this
skill. It only analyzes: no artifacts, lifecycle changes, code, Work Items, or
VCS operations.

## When To Use

Go deep when trade-offs are non-obvious, the choice is expensive to reverse, or
a wrong assumption risks normative, compatibility, data, security, or
operational damage. Use a short comparison for reversible, well-understood
choices. Skip it when a normative RFC already decides the answer or the choice
has no durable consequence.

## Hard Stops

- Invented scenarios, unsupported probabilities, and model confidence are not
  evidence.
- Do not recommend behavior that conflicts with a normative RFC or make an ADR
  the source of new obligations.
- Run no implementation experiment before the lifecycle allows implementation.
- Ask when an unknown could reverse the ranking or cross an authority boundary;
  state smaller assumptions explicitly.

## Method

1. **Frame.** Name the decision, real options (status quo only if viable),
   constraints, success criteria, horizon, and evidence. Separate facts from
   assumptions and note what evidence would change the ranking.
2. **Premortem.** For each option, imagine a concrete failure. Name the few
   material causes, early signals, impact, and prevention. Consider
   specification drift, hidden coupling, migration or data loss, verification
   blind spots, operational burden, and governance cost where relevant.
3. **Backcast.** Imagine it succeeding and work back to the prerequisites,
   actions, dependencies, and signals that made it work, to test for a credible
   path.
4. **Recommend.** Rank by constraints, downside, reversibility, evidence
   quality, cost of learning, and prerequisites. Use numbers only when data
   supports them.

Return `Go`, `No-Go`, or `Conditional Go` with the preferred option and why,
conditions that must hold, pivot triggers, and the smallest lifecycle-valid
step that reduces the biggest uncertainty, such as a sketch or isolated
prototype outside governed behavior. Match depth to risk.

## Where Results Go

| Output                                                | Destination                         |
| ----------------------------------------------------- | ----------------------------------- |
| Observable obligations, compatibility, validation     | RFC                                 |
| Context, alternatives, chosen direction, consequences | ADR                                 |
| Approved scope, acceptance evidence, mitigation work  | Work Item                           |
| Stable, repeatable executable checks                  | Verification Guard                  |
| Assumptions, scenario detail, temporary evidence      | Discussion record or final response |

Put only durable rationale in the ADR, shaped by `adr-writer`; never paste the
full analysis or checklists into its decision. A pivot that could change the
decision or a requirement returns to `discuss` and the owning artifact before
execution resumes.
