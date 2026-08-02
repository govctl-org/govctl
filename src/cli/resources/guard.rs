use clap::Subcommand;

use crate::{CommonDeleteArgs, CommonEditArgs, CommonGetArgs, CommonListArgs, CommonShowArgs};

/// Guard commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum GuardCommand {
    /// List guards
    #[command(after_help = "\
FILTERS:
    Filter may be a guard ID or title substring.

EXAMPLES:
    govctl guard list
    govctl guard list clippy
    govctl guard list GUARD-CLIPPY -o json
")]
    List(CommonListArgs),
    /// Get guard metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, refs
    - description, command, timeout_secs, pattern

EXAMPLES:
    govctl guard get GUARD-0001
    govctl guard get GUARD-0001 description
    govctl guard get GUARD-0001 command
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
    - Create the guard first, then use `guard edit` to define checks.
")]
    New {
        /// Guard title
        title: String,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl guard edit GUARD-0001 command --set \"cargo test\"
    govctl guard edit GUARD-0001 refs --add RFC-0001
    govctl guard edit GUARD-0001 \"refs[0]\" --remove
")]
    Edit(CommonEditArgs),
    /// Delete a verification guard
    #[command(after_help = "\
EXAMPLES:
    govctl guard delete GUARD-CLIPPY
    govctl guard delete GUARD-CLIPPY --force
")]
    Delete(CommonDeleteArgs),
}
