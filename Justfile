# govctl Justfile — Development commands only
# For governance operations, use: cargo run --quiet -- <command>

set shell := ["bash", "-euo", "pipefail", "-c"]
set unstable

# Show available commands
default:
    @just --list

# ---------------------------------------------------------------------------- #
#                                 DEPENDENCIES                                 #
# ---------------------------------------------------------------------------- #

# Cargo: https://rust-lang.org
_cargo := require("cargo")
# Nix (optional, for flake operations)
# nix := require("nix")

# ---------------------------------------------------------------------------- #
#                                  CONSTANTS                                   #
# ---------------------------------------------------------------------------- #

# Internal: govctl binary invocation for development
_govctl := "cargo run --quiet --"

# ---------------------------------------------------------------------------- #
#                                BUILD & TEST                                  #
# ---------------------------------------------------------------------------- #

# Build release binary
[group("build")]
@build:
    cargo build --release
alias b := build

# Run all tests
[group("test")]
@test:
    cargo test
alias t := test

# Test coverage (summary)
[group("test")]
@cov:
    cargo llvm-cov --summary-only

# Test coverage (full HTML report)
[group("test")]
@cov-html:
    cargo llvm-cov --html
    @echo -e '{{ GREEN }}Coverage report:{{ NORMAL }} target/llvm-cov/html/index.html'

# Test coverage (lcov format for CI)
[group("test")]
@cov-lcov:
    cargo llvm-cov --lcov --output-path lcov.info

# Update insta snapshots
[group("test")]
@update-snapshots:
    cargo insta test --accept

# ---------------------------------------------------------------------------- #
#                                   CHECKS                                     #
# ---------------------------------------------------------------------------- #

# Run clippy lints
[group("checks")]
@lint:
    cargo clippy --all-targets

# Format code
[group("checks")]
@fmt:
    cargo fmt

# Pre-commit hooks (lint + format + test)
[group("checks")]
[unix]
@pre-commit:
    @if command -v prek > /dev/null 2>&1; then prek run --all-files; else pre-commit run --all-files; fi

# Pre-commit hooks (Windows)
[group("checks")]
[windows]
@pre-commit:
    @where prek >nul 2>&1 && prek run --all-files || pre-commit run --all-files

# Run lint + format check + tests
[group("checks")]
@check-all:
    just _run-with-status lint
    just _run-with-status fmt
    just _run-with-status test
    @echo ''
    @echo -e '{{ GREEN }}All checks passed!{{ NORMAL }}'
alias ca := check-all

# ---------------------------------------------------------------------------- #
#                                 GOVERNANCE                                   #
# ---------------------------------------------------------------------------- #

# Show governance status
[group("gov")]
@status:
    {{ _govctl }} status
alias s := status

# Validate all governed artifacts
[group("gov")]
@check:
    {{ _govctl }} check
alias c := check

# Render all RFCs, ADRs, and work items to markdown
[group("gov")]
@render:
    {{ _govctl }} render
alias r := render

# Launch TUI dashboard
[group("gov")]
@tui:
    cargo run --quiet -- tui

# ---------------------------------------------------------------------------- #
#                                  RELEASE                                     #
# ---------------------------------------------------------------------------- #

# Update all dependencies (Cargo + Nix inputs)
[group("release")]
@update-deps:
    cargo update
    nix flake update

# Bump version everywhere (Cargo, plugin manifests, releases, changelog)
[group("release")]
[confirm("Bump version to {{ new_version }}?")]
bump new_version:
    @just _run-with-status _bump-cargo {{ new_version }}
    @just _run-with-status _bump-plugin {{ new_version }}
    @just _run-with-status _bump-releases {{ new_version }}
    @just _run-with-status _bump-changelog
    @echo ''
    @echo -e '{{ GREEN }}Bumped to {{ new_version }}{{ NORMAL }}'

# ---------------------------------------------------------------------------- #
#                              INTERNAL HELPERS                                #
# ---------------------------------------------------------------------------- #

# Run a recipe with formatted status output
[private]
@_run-with-status recipe *args:
    @echo ''
    @echo -e '{{ CYAN }}→ Running {{ recipe }}...{{ NORMAL }}'
    just {{ recipe }} {{ args }}
    @echo -e '{{ GREEN }}✓ {{ recipe }} completed{{ NORMAL }}'

# Cross-platform sed in-place edit (macOS and Linux)
[private]
[script("bash")]
_sed_inplace pattern file:
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "$1" "$2"
    else
        sed -i "$1" "$2"
    fi

[private]
_bump-cargo new_version:
    just _sed_inplace 's/^version = ".*"/version = "{{ new_version }}"/' Cargo.toml
    cargo check --quiet 2>/dev/null || true

[private]
_bump-plugin new_version:
    jq --arg v "{{ new_version }}" '.plugins[0].version = $v' .claude-plugin/marketplace.json > .claude-plugin/marketplace.json.tmp
    mv .claude-plugin/marketplace.json.tmp .claude-plugin/marketplace.json
    jq --arg v "{{ new_version }}" '.version = $v' .claude/.claude-plugin/plugin.json > .claude/.claude-plugin/plugin.json.tmp
    mv .claude/.claude-plugin/plugin.json.tmp .claude/.claude-plugin/plugin.json

[private]
_bump-releases new_version:
    {{ _govctl }} release {{ new_version }}

[private]
_bump-changelog:
    {{ _govctl }} render changelog
