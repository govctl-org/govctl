# Working with RFCs

RFCs (Requests for Comments) are the normative specifications in govctl. They define what will be built before implementation begins.

## Creating RFCs

```bash
# Auto-assign next available ID
govctl rfc new "Feature Title"

# Specify ID manually
govctl rfc new "Feature Title" --id RFC-0010
```

## RFC Structure

An RFC consists of:

- **Metadata** (`rfc.toml`) — ID, title, status, phase, version, owners
- **Clauses** (`clauses/*.toml`) — Atomic units of specification

The TOML files use `#:schema` headers and a `[govctl]` + `[content]` layout:

```toml
#:schema ../../schema/rfc.schema.json

[govctl]
id = "RFC-0010"
title = "Feature Title"
version = "0.1.0"
status = "draft"
phase = "spec"
owners = ["@you"]
created = "2026-03-17"
refs = []

[content]
summary = "Brief summary of this RFC."
```

## Tagging RFCs

Once tags are registered in the project vocabulary, apply them to RFCs:

```bash
govctl rfc edit RFC-0010 tags --add caching
govctl rfc edit RFC-0010 tags --add api
```

Filter lists by tag:

```bash
govctl rfc list --tag caching
govctl rfc list --tag caching,api
```

## Canonical Edit Surface

Editable RFC and Clause fields use path-based operations. Lifecycle-owned fields
such as RFC versions, changelog dates, and Clause `since` values use dedicated
commands or automatic assignment instead:

```bash
# Lifecycle-managed fields use dedicated verbs
govctl rfc finalize RFC-0010 normative
govctl rfc bump RFC-0010 --minor -m "Add new clause for edge case"

# Correct metadata for the current RFC version only
govctl rfc get RFC-0010 changelog
govctl rfc edit RFC-0010 changelog.summary --set "Clarify edge-case behavior"
govctl rfc edit RFC-0010 changelog.fixed --add "Correct timeout wording"
govctl rfc edit RFC-0010 changelog.fixed[0] --remove

# Add to array fields
govctl rfc edit RFC-0010 refs --add RFC-0001
govctl rfc edit RFC-0010 owners --add "@co-maintainer"

# Remove by index or pattern
govctl rfc edit RFC-0010 refs --at 0 --remove
govctl rfc edit RFC-0010 owners --remove "@old-owner" --exact

# Edit clause text
govctl clause edit RFC-0010:C-SCOPE text --stdin <<'EOF'
New clause text here
EOF
```

Finalization is the initial publication boundary for a draft RFC. A
version-changing bump opens the next candidate from a normative RFC in `impl`,
`test`, or `stable`; draft, deprecated, and already-open `spec` RFCs are rejected.
Changelog-only corrections do not change the version and remain available
through either `rfc edit ... changelog` or `rfc bump --change`.

## Working with Clauses

### Create a Clause

```bash
govctl clause new RFC-0010:C-SCOPE "Scope" -s "Specification" -k normative
```

Options:

- `-s, --section` — Section name (e.g., "Specification", "Rationale")
- `-k, --kind` — `normative` (binding) or `informative` (explanatory)

Clause files use the same `[govctl]` + `[content]` layout:

```toml
#:schema ../../../schema/clause.schema.json

[govctl]
id = "C-SCOPE"
title = "Scope"
kind = "normative"
status = "active"
since = "0.1.0"

[content]
text = """
The system MUST validate all inputs."""
```

`since` is assigned when the target version is known. Clauses created in a
draft RFC remain pending until finalization. Clauses created in a normative RFC
already in `spec` receive the current RFC version immediately. Clauses created
in `impl`, `test`, or `stable` remain pending until the content amendment is
released by an RFC version bump.

### Edit Clause Text

```bash
# From stdin (recommended for multi-line)
govctl clause edit RFC-0010:C-SCOPE text --stdin <<'EOF'
The system MUST validate all inputs.
The system SHOULD log validation failures.
EOF

# Inline text
govctl clause edit RFC-0010:C-SCOPE text --set "The system MUST validate all inputs."

# From file
govctl clause edit RFC-0010:C-SCOPE text --stdin < clause-text.md
```

### Delete a Clause

Accidentally created clauses can be deleted before they become part of a sealed
version:

```bash
govctl clause delete RFC-0010:C-MISTAKE -f
```

**Safety:** Deletion is only allowed when no artifact references the Clause and
either:

- The RFC status is `draft`; or
- The RFC is `normative/spec` and the Clause `since` equals the current RFC version.

The second case identifies a Clause introduced only in the open candidate.
Inherited Clauses and all Clauses in sealed phases use `govctl clause deprecate`
or `govctl clause supersede` instead.

### List Clauses

```bash
govctl clause list
govctl clause list RFC-0010
govctl clause list --tag core    # Filter by tag
```

### View a Clause

```bash
govctl clause show RFC-0010:C-SCOPE
```

## Status Lifecycle

RFCs have three statuses:

```
draft → normative → deprecated
```

### Finalize to Normative

When the spec is complete and approved:

```bash
govctl rfc finalize RFC-0010 normative
```

This ratifies the RFC lineage. While its current version remains in `spec`, that
content is still an authoring candidate. The version becomes the implementation
baseline when it advances to `impl`.

### Deprecate

When an RFC is superseded or obsolete:

```bash
govctl rfc deprecate RFC-0010
```

## Phase Lifecycle

RFCs progress through four phases:

```
spec → impl → test → stable
```

### Advance Phase

```bash
govctl rfc advance RFC-0010 impl    # Ready for implementation
govctl rfc advance RFC-0010 test    # Implementation complete, ready for testing
govctl rfc advance RFC-0010 stable  # Tested, ready for production
```

Phase transitions are gated:

- `spec → impl` requires `status = normative`
- `spec → impl` requires every Clause to have a resolved `since` version
- `spec → impl` seals the current RFC and Clause content signature
- `impl → test` and `test → stable` require that sealed signature to remain present
- Each phase has invariants that must be satisfied

The sealed signature is a content baseline, not a file lock. A code-only defect
found during `impl` can be fixed without changing the RFC. If RFC or Clause
content must change, edit it and then release that amendment with a patch,
minor, or major bump. The bump starts the new version in `spec`; phase
progression rejects the amendment until that happens. Changelog-only corrections
do not affect the sealed baseline. If a sealed-phase RFC has no signature, bump
and later phase progression stop without changing files; run `govctl migrate` or
restore the baseline from version-control history instead of guessing it.

## Versioning

RFCs use semantic versioning after normative finalization. Draft RFCs remain on
their initial version while they are authored, and deprecated RFCs cannot start
a new version lifecycle:

```bash
# Bump version with changelog entry
govctl rfc bump RFC-0010 --patch -m "Fix typo in clause C-SCOPE"
govctl rfc bump RFC-0010 --minor -m "Add new clause for edge case"
govctl rfc bump RFC-0010 --major -m "Breaking change to API contract"
```

A content-changing bump starts the new version in `spec` only from `impl`, `test`,
or `stable`. RFC and Clause content can continue changing during that `spec` phase
without another version bump; a second version-changing bump is rejected.
Advancing to `impl` seals the final content for the version.

Use change-only bump syntax to append categorized metadata without changing the
RFC version:

```bash
govctl rfc bump RFC-0010 --change "fix: Correct current-version wording"
```

Current-version changelog correction always resolves the entry whose version
matches the RFC version, regardless of array order. Historical entries and all
RFC/changelog version and date fields are not editable through the resource
edit surface.

## Listing and Viewing

```bash
govctl rfc list
govctl rfc list normative    # Filter by status
govctl rfc list impl         # Filter by phase
govctl rfc list --tag api    # Filter by tag
govctl rfc show RFC-0010     # Styled markdown to stdout
```
