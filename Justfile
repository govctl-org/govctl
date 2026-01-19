# govctl Justfile - Development commands only
# For governance operations, use: cargo run --quiet -- <command>

set shell := ["bash", "-cu"]

# Default: show help
default:
    @just --list

# =============================================================================
# Build & Development
# =============================================================================

# Build release binary
build:
    cargo build --release

# Run tests
test:
    cargo test

# Update snapshots
update-snapshots:
    cargo insta test --accept

# Run clippy lints
lint:
    cargo clippy --all-targets

# Format code
fmt:
    cargo fmt

# Pre-commit hooks (lint + format + test)
[unix]
pre-commit:
    @if command -v prek > /dev/null 2>&1; then prek run --all-files; else pre-commit run --all-files; fi

[windows]
pre-commit:
    @where prek >nul 2>&1 && prek run --all-files || pre-commit run --all-files

# =============================================================================
# Quick Shortcuts
# =============================================================================

# Show governance status
status:
    cargo run --quiet -- status

# Validate all governed documents
check:
    cargo run --quiet -- check

# Render RFCs to markdown
render:
    cargo run --quiet -- render

# =============================================================================
# Development Helpers
# =============================================================================

# Sync Claude commands from assets/ (substitutes {{GOVCTL}} -> cargo run)
[unix]
sync-commands:
    ./scripts/sync-commands.sh

[windows]
sync-commands:
    powershell -ExecutionPolicy Bypass -File scripts/sync-commands.ps1

# Build with TUI feature
build-tui:
    cargo build --release --features tui

# Launch TUI dashboard
tui:
    cargo run --quiet --features tui -- tui
