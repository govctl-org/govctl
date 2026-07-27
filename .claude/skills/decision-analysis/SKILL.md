---
name: decision-analysis
description: "Stress-test consequential design options with premortem and backcast reasoning, then return a risk-calibrated recommendation to the owning governance workflow"
---

# Decision Analysis

Expose assumptions, failure paths, success conditions, and option-changing
evidence before a consequential design choice is recorded.

This is a reference skill used by `discuss` and `adr-writer`. It analyzes; it
does not create artifacts, mutate lifecycle state, write implementation, create
Work Items, or perform VCS operations.

## When Analysis Helps

Use deeper analysis when plausible options have non-obvious trade-offs, the
choice is expensive to reverse, consequences cross important boundaries, or a
wrong assumption could create normative, compatibility, data, security, or
operational risk.

Use a compact comparison for reversible, well-understood choices. Skip this
skill when an existing normative RFC already determines the answer or when the
task is a routine implementation choice with no durable architectural
consequence.

## Hard Stops

- Do not treat invented scenarios, unsupported probabilities, or model
  confidence as evidence.
- Do not recommend behavior that conflicts with a normative RFC.
- Do not turn an ADR into the source of new product obligations.
- Do not perform an implementation experiment before the governing lifecycle
  permits implementation work.
- Stop and clarify when an unknown could materially reverse the option ranking
  or violate an authority boundary. State lesser assumptions explicitly.

## Analysis Policy

### Frame The Choice

Identify the decision, viable options, governing constraints, success criteria,
relevant horizon, and available evidence. Include the status quo only when it is
a real option. Separate facts from assumptions and note which evidence would
change the ranking.

### Premortem

For each viable option, imagine a concrete failure at the relevant horizon.
Identify the smallest set of material internal and external causes, their early
signals, likely impact, and feasible prevention or response. Include
specification drift, hidden coupling, migration or data loss, verification
blind spots, operational burden, and governance cost only when relevant.

### Backcast

Imagine the same option succeeding. Work backward to the prerequisites,
controllable actions, external dependencies, and measurable signals that made
success possible. This tests whether the option has a credible path, not merely
an attractive outcome.

### Compare And Recommend

Rank options against governing constraints, downside severity, reversibility,
evidence quality, cost of learning, and success prerequisites. Use quantitative
probability or impact values only when supported by data; otherwise use clear
relative judgments with confidence and rationale.

Return `Go`, `No-Go`, or `Conditional Go`, the preferred option, why it wins,
conditions that must hold, evidence that would trigger a pivot, and the
smallest lifecycle-valid step that reduces the most important uncertainty.
Prefer analysis, interface sketches, prototypes outside governed behavior, or
isolated experiments that do not cross an RFC phase boundary.

Scale the depth to decision risk. Do not generate a large matrix when a short
comparison exposes the decisive trade-off.

## Authority Mapping

Analysis output is input to governed artifacts, not a new source of authority:

| Output                                                                           | Destination                         |
| -------------------------------------------------------------------------------- | ----------------------------------- |
| Observable obligations, compatibility rules, validation behavior                 | RFC                                 |
| Context, evaluated alternatives, chosen direction, consequences                  | ADR                                 |
| Approved delivery scope, acceptance evidence, dependencies, mitigation follow-up | Work Item                           |
| Stable repeatable executable checks                                              | Verification Guard                  |
| Assumptions, scenario detail, temporary evidence                                 | Discussion record or final response |

Summarize only durable rationale in the ADR. Do not paste the full analysis,
execution gates, or task checklist into its decision field. Use `adr-writer` to
shape the selected content and preserve structured alternative ownership.
A pivot that could change the selected decision or a requirement returns to
`discuss` and the owning ADR or RFC before execution resumes.

## Completion Evidence

A useful analysis:

- states the decision, viable options, constraints, evidence, and assumptions;
- gives each serious option a credible failure and success path;
- identifies signals and mitigations that can change action;
- makes a decisive, risk-calibrated recommendation without false precision;
- proposes a lifecycle-valid uncertainty-reduction step; and
- routes obligations, rationale, execution, and verification to their
  authoritative destinations.
