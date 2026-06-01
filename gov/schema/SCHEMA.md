# govctl Schema Specification

This document defines the unified data model for all govctl artifacts.

## Design Principles

1. **Consistency**: Same patterns across artifact types
2. **Minimalism**: Only essential fields; no speculative features
3. **Explicit lifecycle**: Each artifact type has a defined status progression
4. **Cross-referencing**: All artifacts can reference others via `refs`

---

## Identifier Formats

| Artifact  | Format              | Example             |
| --------- | ------------------- | ------------------- |
| RFC       | `RFC-NNNN`          | `RFC-0001`          |
| Clause    | `C-NAME`            | `C-PHASE-ORDER`     |
| ADR       | `ADR-NNNN`          | `ADR-0001`          |
| Work Item | (see ID strategies) | `WI-2026-01-17-001` |

**Full references** combine artifact IDs:

- Clause in RFC: `RFC-0001:C-PHASE-ORDER`
- Standalone: `ADR-0001`, `WI-2026-01-17-001`

### Work Item ID Strategies

Work item ID format is configurable via `gov/config.toml` to support multi-person collaboration. See [[ADR-0020]].

| Strategy               | Format                      | Example                  | Use Case           |
| ---------------------- | --------------------------- | ------------------------ | ------------------ |
| `sequential` (default) | `WI-YYYY-MM-DD-NNN`         | `WI-2026-01-17-001`      | Solo projects      |
| `author-hash`          | `WI-YYYY-MM-DD-{hash4}-NNN` | `WI-2026-01-17-a7f3-001` | Multi-person teams |
| `random`               | `WI-YYYY-MM-DD-{rand4}`     | `WI-2026-01-17-b2c9`     | Simple uniqueness  |

**Configuration:**

```toml
# gov/config.toml
[work_item]
id_strategy = "author-hash"  # or "sequential" (default), "random"
```

- `sequential`: Original behavior. May cause ID collisions in parallel branches.
- `author-hash`: Uses first 4 chars of sha256(git user.email) for namespace isolation. Recommended for teams.
- `random`: Generates random 4-char hex suffix. No sequence number.

---

## Lifecycle States

### RFC Status

```
draft → normative → deprecated
```

| Status       | Meaning                                                 |
| ------------ | ------------------------------------------------------- |
| `draft`      | Under discussion. Implementation MUST NOT depend on it. |
| `normative`  | Frozen. Implementation MUST conform.                    |
| `deprecated` | Obsolete. Implementation SHOULD migrate away.           |

### RFC Phase

```
spec → impl → test → stable
```

| Phase    | Meaning                                   |
| -------- | ----------------------------------------- |
| `spec`   | Defining requirements. No implementation. |
| `impl`   | Building per specification.               |
| `test`   | Verifying implementation.                 |
| `stable` | Released for production use.              |

**Invariant Rules:**

1. **Phase gate rule**: Phase MUST NOT advance past `spec` until status is `normative`.
2. **Stability rule**: Status `draft` + phase `stable` is FORBIDDEN.
3. **Deprecation rule**: Status `deprecated` + phase `impl` or `test` is FORBIDDEN.

These rules ensure specifications are locked before implementation begins, and deprecated specs receive no new development work.

### Clause Status

```
active → superseded
       → deprecated
```

| Status       | Meaning                     |
| ------------ | --------------------------- |
| `active`     | In effect.                  |
| `superseded` | Replaced by another clause. |
| `deprecated` | Obsolete, no replacement.   |

### ADR Status

```
proposed → accepted → superseded
         → rejected
```

| Status       | Meaning                       |
| ------------ | ----------------------------- |
| `proposed`   | Under consideration.          |
| `accepted`   | Ratified decision.            |
| `rejected`   | Declined after consideration. |
| `superseded` | Replaced by newer ADR.        |

### Work Item Status

```
queue → active → done
    ↘        ↘ cancelled
```

| Status      | Meaning                           |
| ----------- | --------------------------------- |
| `queue`     | Planned, not started.             |
| `active`    | In progress.                      |
| `done`      | Completed successfully.           |
| `cancelled` | Abandoned (from queue or active). |

---

## Schema Definitions

### RFC (TOML)

```toml
#:schema ../../schema/rfc.schema.json

[govctl]
id = "RFC-0001"
title = "Example RFC"
version = "1.0.0"
status = "draft"
phase = "spec"
owners = ["@owner"]
created = "2026-01-17"
updated = "2026-01-17"
supersedes = "RFC-0000"

[[sections]]
title = "Section Name"
clauses = ["clauses/C-EXAMPLE.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-17"
notes = "Initial release"
```

| Field               | Required | Type   | Description                            |
| ------------------- | -------- | ------ | -------------------------------------- |
| `govctl.id`         | yes      | string | Unique identifier `RFC-NNNN`           |
| `govctl.title`      | yes      | string | Human-readable title                   |
| `govctl.version`    | yes      | string | Semantic version `X.Y.Z`               |
| `govctl.status`     | yes      | enum   | `draft` \| `normative` \| `deprecated` |
| `govctl.phase`      | yes      | enum   | `spec` \| `impl` \| `test` \| `stable` |
| `govctl.owners`     | yes      | array  | List of responsible parties            |
| `govctl.created`    | yes      | date   | Creation date                          |
| `govctl.updated`    | no       | date   | Last modification date                 |
| `govctl.supersedes` | no       | string | RFC ID this replaces                   |
| `sections`          | yes      | array  | Ordered sections with clause refs      |
| `changelog`         | no       | array  | Version history                        |

### Clause (TOML)

```toml
#:schema ../../../schema/clause.schema.json

[govctl]
id = "C-EXAMPLE"
title = "Example Clause"
kind = "normative"
status = "active"
since = "1.0.0"
anchors = []

[content]
text = "The system MUST do X."
```

| Field                  | Required | Type   | Description                                                  |
| ---------------------- | -------- | ------ | ------------------------------------------------------------ |
| `govctl.id`            | yes      | string | Unique within RFC `C-NAME`                                   |
| `govctl.title`         | yes      | string | Human-readable title                                         |
| `govctl.kind`          | yes      | enum   | `normative` \| `informative`                                 |
| `govctl.status`        | no       | enum   | `active` \| `superseded` \| `deprecated` (default: `active`) |
| `content.text`         | yes      | string | Clause content (Markdown)                                    |
| `govctl.since`         | no       | string | Version introduced                                           |
| `govctl.superseded_by` | no       | string | Clause ID that replaces this                                 |
| `govctl.anchors`       | no       | array  | Cross-reference targets                                      |

### ADR (TOML)

```toml
[govctl]
id = "ADR-0001"
title = "Example Decision"
status = "proposed"
date = "2026-01-17"
superseded_by = "ADR-0002"
refs = ["RFC-0001:C-EXAMPLE"]

[content]
context = """
Background and problem description.
"""

decision = """
What was decided and why.
"""

consequences = """
Impact of the decision.
"""

[[content.alternatives]]
text = "Option A"
status = "rejected"
rejection_reason = "Does not meet performance requirements"
cons = ["Slow", "Hard to maintain"]

[[content.alternatives]]
text = "Option B"
status = "accepted"
pros = ["Fast", "Simple"]
cons = ["Requires new dependency"]
```

| Field                                     | Required | Type   | Description                                            |
| ----------------------------------------- | -------- | ------ | ------------------------------------------------------ |
| `govctl.id`                               | yes      | string | Unique identifier `ADR-NNNN`                           |
| `govctl.title`                            | yes      | string | Decision title                                         |
| `govctl.status`                           | yes      | enum   | `proposed` \| `accepted` \| `rejected` \| `superseded` |
| `govctl.date`                             | yes      | date   | Decision date                                          |
| `govctl.superseded_by`                    | no       | string | ADR ID that replaces this                              |
| `govctl.refs`                             | no       | array  | Cross-references                                       |
| `content.context`                         | yes      | string | Problem description                                    |
| `content.decision`                        | yes      | string | Decision and rationale                                 |
| `content.consequences`                    | yes      | string | Impact analysis                                        |
| `content.alternatives`                    | no       | array  | Options considered                                     |
| `content.alternatives[].text`             | yes      | string | Option description                                     |
| `content.alternatives[].status`           | no       | enum   | `considered` \| `rejected` \| `accepted`               |
| `content.alternatives[].pros`             | no       | array  | Advantages (per [[ADR-0027]])                          |
| `content.alternatives[].cons`             | no       | array  | Disadvantages (per [[ADR-0027]])                       |
| `content.alternatives[].rejection_reason` | no       | string | Why rejected (per [[ADR-0027]])                        |

### Work Item (TOML)

```toml
[govctl]
id = "WI-2026-01-17-001"
title = "Example Work Item"
status = "active"
created = "2026-01-17"
started = "2026-01-17"
completed = "2026-01-18"
refs = ["RFC-0001"]
depends_on = ["WI-2026-01-16-001"]

[content]
description = """
What needs to be done.
"""

notes = ["Key observation", "Remember edge case"]

[[content.acceptance_criteria]]
text = "First criterion"
status = "done"
category = "added"

[[content.acceptance_criteria]]
text = "Second criterion"
status = "pending"
category = "chore"
```

| Field                                    | Required | Type   | Description                                         |
| ---------------------------------------- | -------- | ------ | --------------------------------------------------- |
| `govctl.id`                              | yes      | string | Unique identifier `WI-YYYY-MM-DD-NNN`               |
| `govctl.title`                           | yes      | string | Work item title                                     |
| `govctl.status`                          | yes      | enum   | `queue` \| `active` \| `done` \| `cancelled`        |
| `govctl.created`                         | yes      | date   | Creation date                                       |
| `govctl.started`                         | no       | date   | When work began                                     |
| `govctl.completed`                       | no       | date   | When work finished                                  |
| `govctl.refs`                            | no       | array  | Cross-references                                    |
| `govctl.depends_on`                      | no       | array  | Blocking dependencies on other work items           |
| `content.description`                    | yes      | string | Work description                                    |
| `content.notes`                          | no       | array  | Ad-hoc key points (string array)                    |
| `content.acceptance_criteria`            | no       | array  | Completion checklist                                |
| `content.acceptance_criteria[].text`     | yes      | string | Criterion text                                      |
| `content.acceptance_criteria[].status`   | no       | enum   | `pending` \| `done` \| `cancelled`                  |
| `content.acceptance_criteria[].category` | no       | enum   | Changelog category (`add` \| `fix` \| `chore` etc.) |

### Local Loop State (TOML)

Loop execution state is local runtime data under `.govctl/loops/<loop-id>/`, not a governed work item field. `loop-state.schema.json` defines `state.toml`; `loop-round.schema.json` defines per-round records under `rounds/<WI-ID>/round-NNN.toml`.

```toml
[loop]
id = "LOOP-2026-01-17-001"
state = "active"
work = ["WI-2026-01-17-002"]
resolved = ["WI-2026-01-17-001", "WI-2026-01-17-002"]

[dependencies]
WI-2026-01-17-001 = []
WI-2026-01-17-002 = ["WI-2026-01-17-001"]

[items.WI-2026-01-17-001]
status = "done"
round_count = 1

[items.WI-2026-01-17-002]
status = "active"
round_count = 1
```

```toml
loop_id = "LOOP-2026-01-17-001"
work_item_id = "WI-2026-01-17-002"
round_number = 1
max_rounds = 2
item_status_before = "pending"
item_status_after = "active"
work_status_before = "queue"
work_status_after = "active"
action = "evaluated acceptance criteria"
outcome = "active"
reason = "pending acceptance criteria remain; max rounds not reached"
```

| Field                | Required | Type    | Description                                      |
| -------------------- | -------- | ------- | ------------------------------------------------ |
| `loop.id`            | yes      | string  | Local loop identifier                            |
| `loop.state`         | yes      | enum    | `pending` \| `active` \| `paused` \| `completed` \| `failed` |
| `loop.work`         | yes      | array   | Editable work item IDs requested by the user      |
| `loop.resolved`     | yes      | array   | Dependency-closed work item IDs                  |
| `dependencies`       | yes      | table   | Work item dependency adjacency map               |
| `items.<WI-ID>.status` | yes    | enum    | `pending` \| `active` \| `done` \| `failed` \| `blocked` \| `cancelled` |
| `items.<WI-ID>.round_count` | yes | integer | Number of executed rounds for the work item      |
| `round_number`       | yes      | integer | One-based round number for a work item record    |
| `max_rounds`         | yes      | integer | Round limit used for that invocation             |
| `action`             | yes      | string  | Summary of what the runner attempted             |
| `outcome`            | yes      | enum    | Resulting loop item status for the round         |
| `reason`             | no       | string  | Pending or failure explanation                   |

---

## Cross-Reference Format

References use artifact IDs, optionally scoped to clauses:

```
refs = [
  "RFC-0001",           # Reference to entire RFC
  "RFC-0001:C-EXAMPLE", # Reference to specific clause
  "ADR-0001",           # Reference to ADR
  "WI-2026-01-17-001"   # Reference to work item
]
```

---

## Rendered Markdown Signatures

Per ADR-0003, all rendered markdown files include a deterministic hash signature for tampering detection.

### Format

```markdown
<!-- GENERATED: do not edit. Source: RFC-0000 -->
<!-- SIGNATURE: sha256:64-character-hex-string -->
```

### Purpose

Rendered markdown files are **read-only projections** of the authoritative JSON/TOML sources. The signature ensures:

1. **SSOT enforcement**: Edits to markdown are detected and rejected
2. **Tamper detection**: Any modification breaks the signature
3. **Reproducibility**: Same source always produces same hash

### Computation

The signature is computed as follows:

1. Collect source content (RFC + all clauses, or ADR/Work Item TOML)
2. Convert to JSON and canonicalize:
   - Object keys sorted alphabetically at all nesting levels
   - Arrays preserve element order
   - Compact format (no extra whitespace)
3. For RFCs: sort clauses by clause ID before hashing
4. Prepend signature version header
5. Compute SHA-256 hash
6. Encode as 64-character lowercase hex string

### Verification

`govctl check` verifies that:

- Rendered markdown files have a signature comment
- The signature matches the recomputed hash from current sources

Mismatches are reported as errors. Run `govctl render` to regenerate.

---

## Schema Version

All TOML artifacts include a schema version for forward compatibility:

```toml
[govctl]
schema = 1  # Increment when breaking changes occur
```

Current version: **1**
