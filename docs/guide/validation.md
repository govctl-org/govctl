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

### Guard Fields

| Field          | Required | Description                                  |
| -------------- | -------- | -------------------------------------------- |
| `id`           | Yes      | Unique guard identifier (e.g., `GUARD-LINT`) |
| `title`        | Yes      | Human-readable description                   |
| `refs`         | No       | Related RFCs/ADRs                            |
| `command`      | Yes      | Shell command to execute from project root   |
| `timeout_secs` | No       | Max execution time (default: 300s)           |
| `pattern`      | No       | Regex pattern that must match stdout+stderr  |

### Creating Custom Guards

```bash
# Create a guard file
cat > gov/guard/my-lint.toml <<'EOF'
#:schema ../schema/guard.schema.json

[govctl]
id = "GUARD-MY-LINT"
title = "Linting passes"

[check]
command = "npm run lint"
timeout_secs = 60
EOF
```

Then add it to `gov/config.toml`:

```toml
[verification]
default_guards = ["GUARD-GOVCTL-CHECK", "GUARD-MY-LINT"]
```

### Guard Behavior

- A guard **passes** when its command exits with code 0 (and matches `pattern` if specified)
- A guard **fails** when the command exits non-zero, times out, or doesn't match the pattern
- All guards must pass before `govctl work move <WI-ID> done` succeeds

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

## Schema Migration

When the governance schema evolves between govctl versions:

```bash
govctl migrate
```

This upgrades artifact file formats (e.g., adding `#:schema` headers, converting JSON to TOML) with transactional safety — changes are staged, backed up, and committed atomically.
