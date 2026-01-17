#!/usr/bin/env bash
# Sync Claude commands from assets/ with cargo run substitution
# Usage: ./scripts/sync-commands.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

ASSETS_DIR="$PROJECT_ROOT/assets"
OUTPUT_DIR="$PROJECT_ROOT/.claude/commands"

# Placeholder and replacement
PLACEHOLDER='{{GOVCTL}}'
REPLACEMENT='cargo run --quiet --'

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Process each template
for template in "$ASSETS_DIR"/*.md; do
    if [[ -f "$template" ]]; then
        filename=$(basename "$template")
        output="$OUTPUT_DIR/$filename"
        sed "s|$PLACEHOLDER|$REPLACEMENT|g" "$template" > "$output"
        echo "Synced: $output"
    fi
done

echo "Done. Claude commands synced to $OUTPUT_DIR"
