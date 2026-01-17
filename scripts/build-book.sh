#!/usr/bin/env bash
# Build mdbook from governance artifacts
# Usage: ./scripts/build-book.sh [--serve] [--skip-render]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCS_DIR="$PROJECT_ROOT/docs"

cd "$PROJECT_ROOT"

# Parse arguments
SKIP_RENDER=false
SERVE=false
for arg in "$@"; do
    case $arg in
        --skip-render) SKIP_RENDER=true ;;
        --serve) SERVE=true ;;
    esac
done

# Render governance artifacts to markdown (unless --skip-render)
# Note: We render rfc, adr, and changelog for the book (not work items - too noisy)
if [[ "$SKIP_RENDER" == "false" ]]; then
    echo "Rendering governance artifacts..."
    cargo run --quiet -- render rfc
    cargo run --quiet -- render adr
    cargo run --quiet -- render changelog
    # Copy changelog to docs/ for mdbook
    cp "$PROJECT_ROOT/CHANGELOG.md" "$DOCS_DIR/CHANGELOG.md"
else
    echo "Skipping render (--skip-render)"
fi

# Generate SUMMARY.md dynamically
echo "Generating SUMMARY.md..."

SUMMARY="$DOCS_DIR/SUMMARY.md"
cat > "$SUMMARY" << 'EOF'
# Summary

[Introduction](./INTRODUCTION.md)

---

# User Guide

- [Getting Started](./guide/getting-started.md)
- [Working with RFCs](./guide/rfcs.md)
- [Working with ADRs](./guide/adrs.md)
- [Working with Work Items](./guide/work-items.md)
- [Validation & Rendering](./guide/validation.md)

# Specifications
EOF

# Add RFCs (sorted)
if [[ -d "$DOCS_DIR/rfc" ]]; then
    for rfc in $(ls "$DOCS_DIR/rfc/"*.md 2>/dev/null | sort); do
        filename=$(basename "$rfc")
        id="${filename%.md}"
        # Extract title from first H1
        title=$(grep -m1 '^# ' "$rfc" | sed 's/^# //' || echo "$id")
        echo "- [$title](./rfc/$filename)" >> "$SUMMARY"
    done
fi

# Add ADRs section if any exist
if [[ -d "$DOCS_DIR/adr" ]] && ls "$DOCS_DIR/adr/"*.md &>/dev/null; then
    echo "" >> "$SUMMARY"
    echo "# Decisions" >> "$SUMMARY"
    for adr in $(ls "$DOCS_DIR/adr/"*.md 2>/dev/null | sort); do
        filename=$(basename "$adr")
        id="${filename%.md}"
        title=$(grep -m1 '^# ' "$adr" | sed 's/^# //' || echo "$id")
        echo "- [$title](./adr/$filename)" >> "$SUMMARY"
    done
fi

# Add Changelog if it exists
if [[ -f "$DOCS_DIR/CHANGELOG.md" ]]; then
    echo "" >> "$SUMMARY"
    echo "---" >> "$SUMMARY"
    echo "" >> "$SUMMARY"
    echo "[Changelog](./CHANGELOG.md)" >> "$SUMMARY"
fi

echo "Generated: $SUMMARY"

# Build or serve
cd "$DOCS_DIR"
if [[ "$SERVE" == "true" ]]; then
    echo "Starting mdbook server..."
    mdbook serve --open
else
    echo "Building mdbook..."
    mdbook build
    echo "Book built: $DOCS_DIR/book/"
fi
