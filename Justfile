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

# Test coverage (summary)
cov:
    cargo llvm-cov --summary-only

# Test coverage (full report)
cov-html:
    cargo llvm-cov --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# Test coverage (lcov format for CI)
cov-lcov:
    cargo llvm-cov --lcov --output-path lcov.info

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

# Build with TUI feature
build-tui:
    cargo build --release --features tui

# Launch TUI dashboard
tui:
    cargo run --quiet --features tui -- tui

# =============================================================================
# Release Helpers
# =============================================================================

# Update all dependencies (Cargo + Nix inputs)
update-deps:
    cargo update
    nix flake update

# Bump version everywhere: just bump 0.8.0
bump new_version:
    #!/usr/bin/env bash
    set -euo pipefail
    # 1. Update Cargo.toml
    sed -i '' "s/^version = \".*\"/version = \"{{new_version}}\"/" Cargo.toml
    cargo check --quiet 2>/dev/null || true  # refresh Cargo.lock
    # 2. Stamp plugin manifests
    jq --arg v "{{new_version}}" '.plugins[0].version = $v' .claude-plugin/marketplace.json > .claude-plugin/marketplace.json.tmp
    mv .claude-plugin/marketplace.json.tmp .claude-plugin/marketplace.json
    jq --arg v "{{new_version}}" '.version = $v' .claude/.claude-plugin/plugin.json > .claude/.claude-plugin/plugin.json.tmp
    mv .claude/.claude-plugin/plugin.json.tmp .claude/.claude-plugin/plugin.json
    # 3. flake.nix reads from Cargo.toml automatically
    # 4. Update gov/releases.toml
    cargo run --quiet -- release {{new_version}}
    # 5. Render CHANGELOG.md
    cargo run --quiet -- render changelog
    echo "Bumped to {{new_version}}"
