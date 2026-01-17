# PhaseOS Justfile

# Build the CLI
build:
  cargo build --release

# Run checks
check:
  cargo run -- check

# List RFCs
list-rfcs:
  cargo run -- list rfc

# List ADRs
list-adrs:
  cargo run -- list adr

# List work items
list-work:
  cargo run -- list work

# Render all RFCs
render:
  cargo run -- render

# Show status summary
status:
  cargo run -- stat

# Create a new RFC
new-rfc id title:
  cargo run -- new rfc {{id}} "{{title}}"

# Create a new ADR
new-adr title:
  cargo run -- new adr "{{title}}"

# Create a new work item
new-work title:
  cargo run -- new work "{{title}}"

# Pre-commit hooks
[unix]
pre-commit:
  @if command -v prek > /dev/null 2>&1; then prek run --all-files; else pre-commit run --all-files; fi

[windows]
pre-commit:
  @where prek >nul 2>&1 && prek run --all-files || pre-commit run --all-files
