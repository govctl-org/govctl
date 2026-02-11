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

sync_file() {
    local template="$1" output="$2"
    mkdir -p "$(dirname "$output")"
    sed "s|$PLACEHOLDER|$REPLACEMENT|g" "$template" > "$output"
    echo "Synced: $output"
}

for category in "${CATEGORIES[@]}"; do
    src_dir="$PROJECT_ROOT/assets/$category"
    out_dir="$PROJECT_ROOT/.claude/$category"

    # Find all .md files (flat or in subdirectories)
    while IFS= read -r -d '' template; do
        rel="${template#"$src_dir"/}"
        sync_file "$template" "$out_dir/$rel"
    done < <(find "$src_dir" -name '*.md' -type f -print0 2>/dev/null)
done

echo "Done. Assets synced to .claude/"
