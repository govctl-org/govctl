# govctl Justfile - Thin wrapper around govctl commands
# Requires: just >= 1.46.0

set shell := ["bash", "-cu"]

# Default: show status
default: status

# =============================================================================
# Build & Development
# =============================================================================

# Build the CLI (release mode)
build:
    cargo build --release

# Build with TUI feature
build-tui:
    cargo build --release --features tui

# Run clippy lints
lint:
    cargo clippy --all-targets

# Format code
fmt:
    cargo fmt

# Run tests
test:
    cargo test

# Pre-commit hooks
[unix]
pre-commit:
    @if command -v prek > /dev/null 2>&1; then prek run --all-files; else pre-commit run --all-files; fi

[windows]
pre-commit:
    @where prek >nul 2>&1 && prek run --all-files || pre-commit run --all-files

# =============================================================================
# Governance Commands
# =============================================================================

# Show project status summary
status:
    cargo run --quiet -- status

# Validate all governed documents
check:
    cargo run --quiet -- check

# Validate with warnings as errors
check-strict:
    cargo run --quiet -- check -W

# =============================================================================
# List Commands (positional filter arg, optional)
# =============================================================================

# List RFCs (filter: status/phase/id substring)
list-rfc filter="":
    @if [ -n "{{filter}}" ]; then cargo run --quiet -- list rfc "{{filter}}"; else cargo run --quiet -- list rfc; fi

# List ADRs (filter: status/id substring)
list-adr filter="":
    @if [ -n "{{filter}}" ]; then cargo run --quiet -- list adr "{{filter}}"; else cargo run --quiet -- list adr; fi

# List work items (filter: pending/all/queue/active/done/cancelled)
list-work filter="pending":
    cargo run --quiet -- list work "{{filter}}"

# List clauses (filter: RFC ID)
list-clause filter="":
    @if [ -n "{{filter}}" ]; then cargo run --quiet -- list clause "{{filter}}"; else cargo run --quiet -- list clause; fi

# Shorthand aliases
alias ls-rfc := list-rfc
alias ls-adr := list-adr
alias ls-work := list-work
alias ls := list-work

# =============================================================================
# Create Commands (positional required args)
# =============================================================================

# Create a new RFC (auto-generates ID if not provided)
new-rfc title id="":
    @if [ -n "{{id}}" ]; then cargo run --quiet -- new rfc --id "{{id}}" "{{title}}"; else cargo run --quiet -- new rfc "{{title}}"; fi

# Create a new clause: just new-clause RFC-0001:C-NAME "Title" [section] [kind]
new-clause clause_id title section="Specification" kind="normative":
    cargo run --quiet -- new clause "{{clause_id}}" "{{title}}" -s "{{section}}" -k "{{kind}}"

# Create a new ADR
new-adr title:
    cargo run --quiet -- new adr "{{title}}"

# Create a new work item (use --active flag to activate immediately)
[arg("active", short="a", long="active", value="true")]
new-work title active="false":
    @if [ "{{active}}" = "true" ]; then cargo run --quiet -- new work "{{title}}" --active; else cargo run --quiet -- new work "{{title}}"; fi

# =============================================================================
# Render Commands
# =============================================================================

# Render RFCs to markdown
[arg("dry_run", short="n", long="dry-run", value="true")]
render rfc_id="" dry_run="false":
    @cmd="cargo run --quiet -- render rfc"; \
    if [ -n "{{rfc_id}}" ]; then cmd="$cmd --rfc-id {{rfc_id}}"; fi; \
    if [ "{{dry_run}}" = "true" ]; then cmd="$cmd --dry-run"; fi; \
    eval $cmd

# Render ADRs to markdown
[arg("dry_run", short="n", long="dry-run", value="true")]
render-adr dry_run="false":
    @if [ "{{dry_run}}" = "true" ]; then cargo run --quiet -- render adr --dry-run; else cargo run --quiet -- render adr; fi

# Render work items to markdown
[arg("dry_run", short="n", long="dry-run", value="true")]
render-work dry_run="false":
    @if [ "{{dry_run}}" = "true" ]; then cargo run --quiet -- render work --dry-run; else cargo run --quiet -- render work; fi

# Render all artifacts
[arg("dry_run", short="n", long="dry-run", value="true")]
render-all dry_run="false":
    @if [ "{{dry_run}}" = "true" ]; then cargo run --quiet -- render all --dry-run; else cargo run --quiet -- render all; fi

# =============================================================================
# Edit Commands (positional args)
# =============================================================================

# Set a field value: just set <id> <field> <value>
set id field value:
    cargo run --quiet -- set "{{id}}" "{{field}}" "{{value}}"

# Get a field value: just get <id> [field]
get id field="":
    @if [ -n "{{field}}" ]; then cargo run --quiet -- get "{{id}}" "{{field}}"; else cargo run --quiet -- get "{{id}}"; fi

# Add value to array field: just add <id> <field> <value>
add id field value:
    cargo run --quiet -- add "{{id}}" "{{field}}" "{{value}}"

# Remove value from array field: just remove <id> <field> <value>
remove id field value:
    cargo run --quiet -- remove "{{id}}" "{{field}}" "{{value}}"

# =============================================================================
# Lifecycle Commands (positional args)
# =============================================================================

# Move work item to new status: just move <file> <status>
move file status:
    cargo run --quiet -- move "{{file}}" "{{status}}"

# Activate a work item (queue -> active)
activate file:
    cargo run --quiet -- move "{{file}}" active

# Complete a work item (active -> done)
done file:
    cargo run --quiet -- move "{{file}}" done

# Bump RFC version: just bump <rfc_id> <summary> [--patch|--minor|--major]
[arg("patch", long="patch", value="true")]
[arg("minor", long="minor", value="true")]
[arg("major", long="major", value="true")]
bump rfc_id summary patch="true" minor="false" major="false":
    @level="--patch"; \
    if [ "{{major}}" = "true" ]; then level="--major"; \
    elif [ "{{minor}}" = "true" ]; then level="--minor"; fi; \
    cargo run --quiet -- bump "{{rfc_id}}" $level -m "{{summary}}"

# Finalize RFC to normative
finalize rfc_id:
    cargo run --quiet -- finalize "{{rfc_id}}" normative

# Advance RFC phase: just advance <rfc_id> <phase>
advance rfc_id phase:
    cargo run --quiet -- advance "{{rfc_id}}" "{{phase}}"

# Accept an ADR
accept adr_id:
    cargo run --quiet -- accept "{{adr_id}}"

# Deprecate an artifact
deprecate id:
    cargo run --quiet -- deprecate "{{id}}"

# Supersede an artifact: just supersede <id> <by>
supersede id by:
    cargo run --quiet -- supersede "{{id}}" --by "{{by}}"

# =============================================================================
# Checklist Commands (positional args)
# =============================================================================

# Mark checklist item as done: just tick-done <id> <field> <item>
tick-done id field item:
    cargo run --quiet -- tick "{{id}}" "{{field}}" "{{item}}" done

# Mark checklist item as pending
tick-pending id field item:
    cargo run --quiet -- tick "{{id}}" "{{field}}" "{{item}}" pending

# Mark checklist item as cancelled
tick-cancel id field item:
    cargo run --quiet -- tick "{{id}}" "{{field}}" "{{item}}" cancelled

# =============================================================================
# TUI (optional feature)
# =============================================================================

# Launch interactive TUI dashboard
tui:
    cargo run --quiet --features tui -- tui
