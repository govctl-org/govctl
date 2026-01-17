#!/usr/bin/env bash
# Build mdbook from governance artifacts
# Usage: ./scripts/build-book.sh [--serve]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCS_DIR="$PROJECT_ROOT/docs"

cd "$PROJECT_ROOT"

# Render all governance artifacts to markdown
echo "Rendering governance artifacts..."
cargo run --quiet -- render all

# Generate SUMMARY.md dynamically
echo "Generating SUMMARY.md..."

SUMMARY="$DOCS_DIR/SUMMARY.md"
cat > "$SUMMARY" << 'EOF'
# Summary

[Introduction](./introduction.md)

---

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

# Add Work Items section if any exist
if [[ -d "$DOCS_DIR/work" ]] && ls "$DOCS_DIR/work/"*.md &>/dev/null; then
    echo "" >> "$SUMMARY"
    echo "# Work Items" >> "$SUMMARY"
    for work in $(ls "$DOCS_DIR/work/"*.md 2>/dev/null | sort); do
        filename=$(basename "$work")
        id="${filename%.md}"
        title=$(grep -m1 '^# ' "$work" | sed 's/^# //' || echo "$id")
        echo "- [$title](./work/$filename)" >> "$SUMMARY"
    done
fi

echo "Generated: $SUMMARY"

# Build or serve
cd "$DOCS_DIR"
if [[ "${1:-}" == "--serve" ]]; then
    echo "Starting mdbook server..."
    mdbook serve --open
else
    echo "Building mdbook..."
    mdbook build
    echo "Book built: $DOCS_DIR/book/"
fi
