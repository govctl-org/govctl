---
name: decision-analysis
description: "Stress-test a design decision with premortem/backcast analysis, then produce a risk-calibrated recommendation that maps to ADR fields. Use when: (1) Multiple competing architecture options, (2) Irreversible or high-risk design choices, (3) /discuss identifies a complex trade-off"
---

# Decision Analysis

Stress-test a design decision using premortem (simulated failure) and backcasting (simulated success), then produce a structured recommendation that feeds directly into ADR artifacts.

This skill is a **reference skill** — called by `/discuss` during design exploration, not invoked standalone.

## When to Use

Use this analysis when the decision meets **any** of:

- 2+ competing options with non-obvious trade-offs
- Irreversible or hard-to-reverse architectural change
- Cross-cutting impact (touches 3+ modules or affects external interfaces)
- Normative RFC finalization with significant design risk

**Do not use** for: single-option bug fixes, doc-only changes, trivial refactors, or decisions already locked by existing normative RFCs.

## Workflow

### Step 1: Normalize the Decision

Extract from the discussion context:

- **Decision statement:** One sentence describing what must be decided
- **Options:** List of alternatives (always include "do nothing / status quo" if relevant)
- **Constraints:** Existing RFCs, ADRs, phase requirements, technical limitations
- **Success criteria:** What "this worked" looks like in measurable terms
- **Timeframe:** Delivery window; infer a reasonable default if unstated

If any unresolved unknown would materially affect option ranking, reversibility, or constraints, ask clarifying questions before proceeding.
If the remaining unknowns are minor, proceed with explicit assumptions.
Ask at most 2 clarifying questions in one round.

### Step 2: Define Scenario Anchors

Write two one-sentence anchors from the end of the timeframe:

- **Failure anchor:** What "this went badly" looks like (measurable: regressions, compliance failures, blocked phases)
- **Success anchor:** What "this worked well" looks like (measurable: tests green, phase advanced, no spec drift)

Note whether the decision is **reversible** or **irreversible**, and whether external dependence is low/medium/high.

### Step 3: Premortem (Simulated Failure)

For the top 2-3 viable options, imagine each path fully failed. Generate:

- Up to 5 **internal causes** (controllable: implementation choices, spec gaps, testing blind spots)
- Up to 5 **external causes** (uncontrollable: dependency changes, upstream breakage, ecosystem shifts)

Make each cause specific and falsifiable. Include govctl-relevant failure modes:

- Silent deviation from normative RFC (Law 2 violation)
- Phase discipline breach (impl work during spec phase)
- Hidden coupling across RFC boundaries
- Migration or data integrity risks
- Operational burden or governance overhead

### Step 4: Backcast (Simulated Success)

For the same options, imagine each path succeeded strongly. Generate:

- Up to 5 **internal causes** (controllable advantages)
- Up to 5 **external causes** (tailwinds, timing, ecosystem support)

Include govctl-relevant success drivers:

- Clean RFC-to-implementation traceability
- Phase gates caught issues early
- Strong clause coverage in tests
- ADR alternatives documented well enough to avoid revisiting

### Step 5: Decision Exploration Table

For each cause from Steps 3-4, assign:

| Category | Control | Reason | Probability (%) | Confidence | Impact (1-5) | Early Signal | Countermeasure |
|----------|---------|--------|-----------------|------------|---------------|--------------|----------------|

Calibration guide:

- 10-19%: Unlikely but possible
- 20-39%: Somewhat unlikely
- 40-59%: Moderate likelihood
- 60-79%: Likely
- 80-95%: Highly probable

Use numbers, not vague terms.

### Step 6: Recommend

Compare the options using the evidence above, rank them, and produce:

- **Verdict:** `Go`, `No-Go`, or `Conditional Go`
- **Top option** with 1-sentence rationale
- **Must-pass gates** before commitment (e.g., `govctl check` clean, load test threshold, security review)
- **Pivot triggers:** conditions that would change the recommendation
- **Smallest validating step:** the smallest non-production validation that reduces uncertainty.
  Prefer design spike, interface sketch, decision matrix, or isolated experiment.
  Do not implement governed behavior before the RFC is normative and in `impl` phase.

For `Conditional Go`, specify exactly what conditions must be met.

### Step 7: Mitigation Strategies

2-4 bullet points for each category:

- **Precommitments:** Decisions locked now to prevent drift (e.g., "RFC clause X forbids approach Y")
- **Hedging:** Feature flags, staged rollout, dual-write, rollback plan
- **Slow-sabotage prevention:** Rules that prevent gradual scope creep or silent deviation
- **Setback response:** What to do when things go wrong (not "try harder" — concrete next steps)

### Step 8: Review Checkpoints

Align with govctl phase gates:

| Checkpoint | Phase Gate | What to Measure | Kill Criteria | Scale Criteria |
|------------|-----------|-----------------|---------------|----------------|
| Early (before commitment) | Before `finalize normative` | Design evidence, assumptions, interface feasibility | Core assumptions falsified | Option remains viable |
| Mid (implementation) | Before `advance test` | `govctl check` clean, clause coverage | Spec drift detected | All clauses implemented |
| Late (release readiness) | Before `advance stable` | Full test suite, no regressions | Compliance failures | Ready for stable |

## Output → ADR Mapping

When the analysis is complete, map outputs to ADR fields using the **adr-writer** skill:

| Analysis Output | ADR Field | How to Map |
|----------------|-----------|------------|
| Decision statement + constraints | `context` | Problem statement and constraints section |
| Premortem + backcast summaries | `consequences` | Positive from backcast, Negative from premortem (with mitigations) |
| Options ranking | `alternatives` | Each option with pros/cons from the exploration table |
| Top option + rationale | `decision` | "We will **X** because..." |
| Must-pass gates | `decision` | Add under `### Implementation Notes` |
| Related RFCs/ADRs discovered | `refs` | Link as references |

## Compact Mode

If the decision is moderate-risk (only 2 options, single-module scope, reversible), produce a **compact analysis**:

1. One-paragraph context with anchors
2. Abbreviated table (top 3 risks, top 3 levers only)
3. Verdict with rationale
4. Key mitigations (2-3 bullets)

Skip Steps 7-8 in compact mode.
