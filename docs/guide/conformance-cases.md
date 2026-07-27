# Conformance Cases

Conformance Cases connect a project-owned acceptance scenario to one or more
versioned RFC Clause requirements and reusable Verification Guards:

```text
RFC Clause requirement -> Conformance Case -> Verification Guard
normative authority       derived scenario    execution entrypoint
```

A Case is traceability metadata, not a test runner or a source of normative
requirements. When a Case and an RFC disagree, the RFC governs.

## Create A Case

The scenario path must already exist, stay inside the repository, and remain
outside `gov/`. The selector is project-defined; use `*` for the complete file.

```bash
govctl conformance new "Cache invalidation behavior" \
  --path tests/conformance/cache.toml \
  --selector cache-expiry \
  --requirement RFC-0012:C-CACHE-EXPIRY@1.2.0 \
  --guard GUARD-CACHE-CONFORMANCE
```

Requirement values always use `<CLAUSE-ID>@<RFC-VERSION>`. A Case may bind
multiple requirements and Guards.

## Inspect And Edit

```bash
govctl conformance list
govctl conformance show CONF-CACHE-INVALIDATION-BEHAVIOR
govctl conformance get CONF-CACHE-INVALIDATION-BEHAVIOR requirements

govctl conformance edit CONF-CACHE-INVALIDATION-BEHAVIOR \
  requirements --add RFC-0012:C-CACHE-FALLBACK@1.2.0
govctl conformance edit CONF-CACHE-INVALIDATION-BEHAVIOR \
  requirements[0].version --set 1.3.0
govctl conformance edit CONF-CACHE-INVALIDATION-BEHAVIOR \
  requirements[1] --remove
```

Case mutations validate the complete prospective trace graph before writing.
Removing the final requirement is rejected.

## Query Traceability

```bash
govctl conformance trace
govctl conformance trace RFC-0012
govctl conformance trace RFC-0012:C-CACHE-EXPIRY
govctl conformance trace CONF-CACHE-INVALIDATION-BEHAVIOR --output json
govctl conformance trace GUARD-CACHE-CONFORMANCE
```

Trace output derives `provisional`, `candidate`, `current`, or `stale`
applicability from the referenced RFC version and Clause lifecycle. These values
describe the declared relationship only; they do not mean that a scenario has
run or passed.

Cases also participate in `govctl search`, controlled-vocabulary tags,
`govctl status`, and `govctl check`.

## Schema Migration

Conformance Cases require project schema version 4. Upgrade an existing project
with:

```bash
govctl migrate
govctl check
```

Migration preserves valid prospective Case files and rejects an invalid Case
graph without partially changing the repository.
