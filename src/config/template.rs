use super::Config;

impl Config {
    /// Generate default config TOML at the given schema version.
    pub fn default_toml(schema_version: u32) -> String {
        format!(
            r#"[project]
name = "my-project"
# Default owner for new RFCs (uses git user.name if not set)
# default_owner = "@your-handle"

[paths]
docs_output = "docs"
# AI agent directory — target for `govctl init-skills`
# Contains skills/ and agents/ subdirs
# Default: ".claude" (Claude Code / Claude Desktop)
# For Cursor: ".cursor"
# For Windsurf: ".windsurf"
# agent_dir = ".claude"

[schema]
version = {schema_version}

# [work_item]
# ID strategy for work items (default: sequential)
# - sequential: WI-YYYY-MM-DD-NNN (solo projects)
# - author-hash: WI-YYYY-MM-DD-{{hash}}-NNN (multi-person teams, uses git email)
# - random: WI-YYYY-MM-DD-{{rand}} (simple uniqueness)
# id_strategy = "author-hash"

# [verification]
# Enable project-level default verification guards.
# enabled = true
# default_guards = ["GUARD-GOVCTL-CHECK", "GUARD-CARGO-TEST"]

# [source_scan]
# Scan source files for [[artifact-id]] references during `govctl check`
# enabled = false
# include = ["src/**/*.rs", "crates/**/*.rs", "**/*.md"]
# exclude = []

# [concurrency]
# Maximum seconds to wait for exclusive lock before failing (default: 30)
# Implements [[RFC-0004]] concurrent write safety
# lock_timeout_secs = 30

# [tags]
# Controlled-vocabulary tags for artifact classification — [[RFC-0002:C-RESOURCES]]
# Artifacts may only use tags listed here.
# allowed = ["security", "breaking-change", "performance"]
"#
        )
    }
}
