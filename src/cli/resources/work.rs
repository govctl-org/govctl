use std::path::PathBuf;

use clap::Subcommand;

use crate::model::WorkItemStatus;
use crate::{
    CommonDeleteArgs, CommonEditArgs, CommonGetArgs, CommonListArgs, CommonRenderArgs,
    CommonShowArgs,
};

/// Work item commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum WorkCommand {
    /// List work items
    #[command(after_help = "\
FILTERS:
    Filter may be a work-item status, work-item ID, or title substring.

EXAMPLES:
    govctl work list
    govctl work list active
    govctl work list queue -n 10
")]
    List(CommonListArgs),
    /// Get work item metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, description, status, completed_at, refs, depends_on
    - notes, acceptance_criteria
    - verification.required_guards, verification.waivers

EXAMPLES:
    govctl work get WI-2026-04-06-001
    govctl work get WI-2026-04-06-001 description
    govctl work get WI-2026-04-06-001 \"acceptance_criteria[0].status\"
    govctl work get WI-2026-04-06-001 verification.required_guards
")]
    Get(CommonGetArgs),
    /// Show rendered work item content
    #[command(after_help = "\
EXAMPLES:
    govctl work show WI-2026-04-06-001
    govctl work show WI-2026-04-06-001 --history
    govctl work show WI-2026-04-06-001 -o plain

NOTES:
    - Work Items have no obsolete-body state, so current and archival content are equivalent.
    - JSON, YAML, and TOML output is complete and cannot be combined with `--history`.
")]
    Show(CommonShowArgs),
    /// Create a new work item
    #[command(after_help = "\
EXAMPLES:
    govctl work new \"Implement RFC-0005 parser\"
    govctl work new \"Implement RFC-0005 parser\" --active

NOTES:
    - Use `--active` to immediately start the work item.
    - Add acceptance criteria before moving to `done`.
")]
    New {
        /// Work item title
        title: String,
        /// Immediately activate the work item
        #[arg(long)]
        active: bool,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl work edit WI-2026-04-06-001 description --set \"Scope and why\"
    govctl work edit WI-2026-04-06-001 depends_on --add WI-2026-04-06-002
    govctl work edit WI-2026-04-06-001 acceptance_criteria --add \"add: Implement feature X\"
    govctl work edit WI-2026-04-06-001 \"acceptance_criteria[0]\" --set \"fix: Correct edge case\"
    govctl work edit WI-2026-04-06-001 \"acceptance_criteria[0]\" --tick done
    govctl work edit WI-2026-04-06-001 verification.required_guards --add GUARD-CARGO-TEST
")]
    Edit(CommonEditArgs),
    /// Move work item to new status
    #[command(after_help = "\
EXAMPLES:
    govctl work move WI-2026-04-06-001 active
    govctl work move WI-2026-04-06-001 done

NOTES:
    - `done` requires acceptance criteria and effective guards to pass.
    - Use `work edit ... \"acceptance_criteria[N]\" --tick` to update criterion status.
")]
    Move {
        /// Work item file path or ID
        #[arg(value_name = "FILE_OR_ID")]
        file: PathBuf,
        /// Target status
        #[arg(value_enum)]
        status: WorkItemStatus,
    },
    /// Delete work item
    #[command(after_help = "\
EXAMPLES:
    govctl work delete WI-2026-04-06-001
    govctl work delete WI-2026-04-06-001 --force
")]
    Delete(CommonDeleteArgs),
    /// Render a single work item to markdown
    #[command(after_help = "\
EXAMPLES:
    govctl work render WI-2026-04-06-001
    govctl work render WI-2026-04-06-001 --dry-run
")]
    Render(CommonRenderArgs),
}
