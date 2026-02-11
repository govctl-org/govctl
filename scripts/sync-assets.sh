#!/usr/bin/env bash
# Sync assets (commands, skills, agents) from assets/ to .claude/ with {{GOVCTL}} substitution
# Usage: ./scripts/sync-assets.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Placeholder and replacement
PLACEHOLDER='{{GOVCTL}}'
REPLACEMENT='cargo run --quiet --'

# Categories to sync: assets/<category>/ -> .claude/<category>/
CATEGORIES=(commands skills agents)

for category in "${CATEGORIES[@]}"; do
    src_dir="$PROJECT_ROOT/assets/$category"
    out_dir="$PROJECT_ROOT/.claude/$category"

    # Skip if source directory has no .md files
    if ! compgen -G "$src_dir"/*.md > /dev/null 2>&1; then
        continue
    fi

    mkdir -p "$out_dir"

    for template in "$src_dir"/*.md; do
        if [[ -f "$template" ]]; then
            filename=$(basename "$template")
            output="$out_dir/$filename"
            sed "s|$PLACEHOLDER|$REPLACEMENT|g" "$template" > "$output"
            echo "Synced: $output"
        fi
    done
done

echo "Done. Assets synced to .claude/"
