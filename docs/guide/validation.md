# Validation & Rendering

govctl provides tools to validate governance artifacts, enforce completion gates, and render human-readable documentation.

## Validation

### Check All Artifacts

```bash
govctl check
```

This validates:

- **Schema conformance** — All required fields present, correct types
- **Phase discipline** — No invalid state transitions
- **Cross-references** — `refs` and `[[...]]` annotations point to existing artifacts
- **Controlled-vocabulary tags** — All artifact tags are registered in `gov/config.toml [tags] allowed`
- **Clause structure** — Normative clauses in spec sections
- **Source code scanning** — `[[RFC-0001]]` annotations in source files are verified

### Exit Codes

- `0` — All validations passed
- `1` — Validation errors found

### Source Code Scanning

govctl scans source files for `[[artifact-id]]` annotations and verifies they reference existing, non-deprecated artifacts:

```rust
// Implements [[RFC-0001:C-VALIDATION]]
fn validate() { ... }

// Per [[ADR-0005]], we use semantic colors
```

Configure scanning in `gov/config.toml`:

```toml
[source_scan]
enabled = true
include = ["src/**/*.rs"]
exclude = []
```

## Controlled-Vocabulary Tags

Tags provide cross-cutting categorization across all governance artifacts. Every tag must be registered in a project-level allow list before use.

### Managing the Tag Registry

```bash
# List all registered tags with usage counts
govctl tag list

# Register a new tag
govctl tag new caching

# Remove a tag (fails if any artifact still uses it)
govctl tag delete caching
```

### Tagging Artifacts

Once a tag is registered, apply it to any artifact via the standard `edit` command with `--add`:

```bash
govctl rfc edit RFC-0010 tags --add caching
govctl adr edit ADR-0003 tags --add caching
govctl work edit WI-2026-01-17-001 tags --add caching
```

### Filtering by Tag

List commands support `--tag` to filter by one or more tags (comma-separated, AND logic):

```bash
govctl rfc list --tag caching
govctl adr list --tag caching,performance
govctl work list --tag breaking-change
```

Tags are validated at `govctl check` time — any tag not in the allow list produces error `E1105`.

## Verification Guards

Guards are executable completion checks that run automatically when a work item moves to `done`. They prevent work items from closing unless all configured checks pass.

### How Guards Work

When you run `govctl work move <WI-ID> done`, govctl executes each guard defined in `gov/config.toml`:

```toml
[verification]
enabled = true
default_guards = ["GUARD-GOVCTL-CHECK", "GUARD-CARGO-TEST"]
```

Each guard is a TOML file in `gov/guard/`:

```toml
#:schema ../schema/guard.schema.json

[govctl]
id = "GUARD-CARGO-TEST"
title = "cargo test passes"
refs = ["RFC-0000"]

[check]
command = "cargo test"
timeout_secs = 300
```

### Guard Subcommands

Guards are first-class resources with their own CRUD verbs:

```bash
# Create a new guard
govctl guard new "My Lint Check"

# List all guards
govctl guard list

# Show guard definition
govctl guard show GUARD-MY-LINT

# Set guard fields
govctl guard edit GUARD-MY-LINT check.command --set "npm run lint"
govctl guard edit GUARD-MY-LINT check.timeout_secs --set 60

# Delete a guard (blocked if still referenced by work items or project defaults)
govctl guard delete GUARD-MY-LINT
```

### Guard Fields

| Field          | Required | Description                                  |
| -------------- | -------- | -------------------------------------------- |
| `id`           | Yes      | Unique guard identifier (e.g., `GUARD-LINT`) |
| `title`        | Yes      | Human-readable description                   |
| `refs`         | No       | Related RFCs/ADRs                            |
| `command`      | Yes      | Shell command to execute from project root   |
| `timeout_secs` | No       | Max execution time (default: 300s)           |
| `pattern`      | No       | Regex pattern that must match stdout+stderr  |

### Guard Behavior

- A guard **passes** when its command exits with code 0 (and matches `pattern` if specified)
- A guard **fails** when the command exits non-zero, times out, or doesn't match the pattern
- All guards must pass before `govctl work move <WI-ID> done` succeeds

### Running Guards Independently

Use `govctl verify` to run guards without moving a work item:

```bash
# Run all project default guards
govctl verify

# Run specific guards
govctl verify GUARD-CARGO-TEST GUARD-GOVCTL-CHECK

# Run guards required by a specific work item
govctl verify --work WI-2026-01-17-001
```

### Per-Work-Item Guards

Project-level `default_guards` are only part of the picture. A work item can also require additional guards of its own:

```toml
[verification]
required_guards = ["GUARD-CLIPPY"]
```

This is useful when one work item needs an extra check that should not become a project-wide default.

The effective required guard set for a work item is:

- the project-level `default_guards` when verification is enabled
- plus the work item's `verification.required_guards`
- minus any explicitly waived guards

### Guard Waivers

If a guard must be waived for a specific work item, record that explicitly with a reason:

```toml
[[verification.waivers]]
guard = "GUARD-CARGO-TEST"
reason = "Flaky on CI runner image; tracked in issue #123"
```

Waivers are scoped to a single work item. They do not disable verification globally, and they should be treated as an exception that must be explained.

## Rendering

Render governance artifacts to markdown for documentation.

### Render All

```bash
govctl render            # RFCs to docs/rfc/
govctl render adr        # ADRs to docs/adr/
govctl render work       # Work items to docs/work/
govctl render all        # Everything
govctl render changelog  # CHANGELOG.md
```

### Render Single Items

```bash
govctl rfc render RFC-0010
govctl adr render ADR-0005
govctl work render WI-2026-01-17-001
```

### View Without Writing Files

The `show` commands render styled markdown to stdout without writing files:

```bash
govctl rfc show RFC-0010
govctl adr show ADR-0005
govctl work show WI-2026-01-17-001
govctl clause show RFC-0010:C-SCOPE
```

### Hash Signatures

Rendered markdown includes a SHA-256 signature for tampering detection:

```markdown
<!-- SIGNATURE: sha256:abc123... -->
```

If the source changes, the signature won't match — indicating the rendered doc is stale.

## Project Status

```bash
govctl status
```

Shows RFC/ADR/work item counts by status, phase breakdown, and active work items.

## CLI Self-Description

govctl provides a machine-readable command catalog for agent discoverability:

```bash
govctl describe
govctl describe --context   # Includes project context (RFCs, ADRs, active work items)
govctl describe --output json
```

## Self-Update

Update govctl to the latest release:

```bash
govctl self-update          # Download and replace binary
govctl self-update --check  # Check for newer version without downloading
```

Supports `GITHUB_TOKEN` environment variable for authenticated API requests.

## Agent Skill Installation

Install or update govctl's agent skills and reviewer agents for your AI coding tool:

```bash
# Claude Code (default)
govctl init-skills

# Codex CLI
govctl init-skills --format codex

# Custom output directory
govctl init-skills --dir /path/to/agent-config
```

This writes workflow skills (`/gov`, `/quick`, `/discuss`, `/commit`) and reviewer agents (RFC/ADR/WI reviewer, compliance checker) to the configured agent directory.

## Schema Migration

When the governance schema evolves between govctl versions, artifact files may need format upgrades:

```bash
govctl migrate
```

This upgrades artifact file formats (e.g., adding `#:schema` headers, converting JSON to TOML) with transactional safety — changes are staged, backed up, and committed atomically.

### `govctl migrate` vs the `/migrate` Workflow

These are related but serve different purposes:

|            | `govctl migrate`                                    | `/migrate` skill                                      |
| ---------- | --------------------------------------------------- | ----------------------------------------------------- |
| **What**   | Upgrade existing govctl artifacts to current format | Adopt govctl in an existing project                   |
| **When**   | After updating govctl version                       | When starting governance in a brownfield repo         |
| **Effect** | Rewrites TOML/JSON files in `gov/`                  | Discovers decisions, backfills ADRs, annotates source |
| **Risk**   | Low — transactional, reversible                     | Medium — requires human review of generated ADRs      |

Run `govctl migrate` when govctl tells you a migration is needed (error `E0505`). Use the `/migrate` skill when bringing a legacy project under governance for the first time.
