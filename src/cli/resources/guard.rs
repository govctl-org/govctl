use clap::Subcommand;

use crate::{
    CommonDeleteArgs, CommonEditArgs, CommonGetArgs, CommonListArgs, CommonRemoveArgs,
    CommonSetArgs, CommonShowArgs, GuardAddArgs,
};

/// Guard commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum GuardCommand {
    /// List guards
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be a guard ID or title substring.

EXAMPLES:
    govctl guard list
    govctl guard list clippy
    govctl guard list GUARD-CLIPPY -o json
"
    )]
    List(CommonListArgs),
    /// Get guard metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, refs
    - description, check.command, check.timeout_secs, check.pattern

EXAMPLES:
    govctl guard get GUARD-0001
    govctl guard get GUARD-0001 description
    govctl guard get GUARD-0001 check.command
")]
    Get(CommonGetArgs),
    /// Show rendered guard content
    #[command(after_help = "\
EXAMPLES:
    govctl guard show GUARD-CLIPPY
    govctl guard show GUARD-CLIPPY --history
    govctl guard show GUARD-CLIPPY -o json

NOTES:
    - Guards have no obsolete-body state, so current and archival content are equivalent.
    - JSON, YAML, and TOML output is complete and cannot be combined with `--history`.
")]
    Show(CommonShowArgs),
    /// Create a new verification guard
    #[command(after_help = "\
EXAMPLES:
    govctl guard new \"clippy lint\"

NOTES:
    - Create the guard first, then use `guard edit` / `guard set` to define checks.
")]
    New {
        /// Guard title
        title: String,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl guard edit GUARD-0001 description --set \"What this guard verifies\"
    govctl guard edit GUARD-0001 refs --add RFC-0001
    govctl guard edit GUARD-0001 refs[0] --remove
")]
    Edit(CommonEditArgs),
    /// Set guard field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use `set`):
    - title
    - description
    - check.command
    - check.pattern
    - check.timeout_secs

EXAMPLES:
    govctl guard set GUARD-0001 description \"Runs clippy on the workspace\"
    govctl guard set GUARD-0001 check.command \"cargo clippy --all-targets -- -D warnings\"
")]
    Set(CommonSetArgs),
    /// Add value to guard array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs

EXAMPLES:
    govctl guard add GUARD-0001 refs RFC-0001
")]
    Add(GuardAddArgs),
    /// Remove value from guard array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl guard remove GUARD-0001 refs RFC-0001
    govctl guard remove GUARD-0001 refs --at 0
")]
    Remove(CommonRemoveArgs),
    /// Delete a verification guard
    #[command(after_help = "\
EXAMPLES:
    govctl guard delete GUARD-CLIPPY
    govctl guard delete GUARD-CLIPPY --force
")]
    Delete(CommonDeleteArgs),
}
