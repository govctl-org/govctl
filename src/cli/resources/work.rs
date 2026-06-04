use std::path::PathBuf;

use clap::Subcommand;

use crate::model::WorkItemStatus;
use crate::{
    CommonDeleteArgs, CommonGetArgs, CommonListArgs, CommonRemoveArgs, CommonRenderArgs,
    CommonSetArgs, CommonShowArgs, WorkAddArgs, WorkEditArgs, WorkTickArgs,
};

/// Work item commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum WorkCommand {
    /// List work items
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be a work-item status, work-item ID, or title substring.

EXAMPLES:
    govctl work list
    govctl work list active
    govctl work list queue -n 10
"
    )]
    List(CommonListArgs),
    /// Get work item metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, description, status, completed_at, refs, depends_on
    - notes, acceptance_criteria

EXAMPLES:
    govctl work get WI-2026-04-06-001
    govctl work get WI-2026-04-06-001 description
    govctl work get WI-2026-04-06-001 acceptance_criteria[0].status
")]
    Get(CommonGetArgs),
    /// Show rendered work item content
    #[command(after_help = "\
EXAMPLES:
    govctl work show WI-2026-04-06-001
    govctl work show WI-2026-04-06-001 -o plain
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
    govctl work edit WI-2026-04-06-001 content.acceptance_criteria --add \"add: Implement feature X\"
    govctl work edit WI-2026-04-06-001 content.acceptance_criteria[0] --tick done
")]
    Edit(WorkEditArgs),
    /// Set work item field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - description: Task scope declaration
    - title: Work item title

  Array fields (use 'add'/'remove' instead):
    - refs: Cross-references to RFCs/ADRs
    - depends_on: Blocking dependencies on other work items
    - notes: Durable constraints or retry rules (short strings)
    - acceptance_criteria: Completion criteria with category

FIELD SEMANTICS:
  - description: Task scope - define once, rarely change
  - depends_on: Work item IDs that must complete before this item starts
  - notes: Closure-worthy durable context only; do not store progress, commands run, next actions, temporary blockers, or TODOs

EXAMPLES:
    govctl work set WI-001 description \"New description\"
    govctl work set WI-001 description --stdin <<'EOF'
    Multi-line description here
    EOF

Use dedicated verbs instead of `set` for:
    - status → `govctl work move`
    - acceptance_criteria[*].status → `govctl work tick`
")]
    Set(CommonSetArgs),
    /// Add value to work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to RFCs/ADRs (e.g., \"RFC-0001\", \"ADR-0002\")
    - depends_on: Blocking dependencies on work items (e.g., \"WI-2026-04-06-001\")
    - notes: Durable constraints or retry rules (short strings)
    - acceptance_criteria: Completion criteria with category prefix

FIELD SEMANTICS:
  - description: Task scope - define once, rarely change
  - depends_on: Work item IDs only; cyclic dependencies are rejected
  - notes: Closure-worthy durable context only; do not store progress, commands run, next actions, temporary blockers, or TODOs

ACCEPTANCE CRITERIA FORMAT:
    Use category prefix for changelog generation:
    - \"add: New feature\"       → Added section
    - \"fix: Bug fixed\"         → Fixed section
    - \"changed: Behavior\"      → Changed section
    - \"chore: Tests pass\"      → Excluded from changelog

EXAMPLES:
    govctl work add WI-001 refs RFC-0001
    govctl work add WI-001 depends_on WI-2026-04-06-002
    govctl work add WI-001 acceptance_criteria \"add: Implement feature\"
    govctl work add WI-001 notes \"Do not retry parser path X; it cannot preserve normalized arrays\"
")]
    Add(WorkAddArgs),
    /// Remove value from work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, depends_on, notes, acceptance_criteria

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl work remove WI-001 refs RFC-0001     # Remove first match
    govctl work remove WI-001 depends_on WI-2026-04-06-002
    govctl work remove WI-001 refs --at 1       # Remove by index
    govctl work remove WI-001 notes --all       # Remove all
")]
    Remove(CommonRemoveArgs),
    /// Move work item to new status
    #[command(visible_alias = "mv")]
    #[command(after_help = "\
EXAMPLES:
    govctl work move WI-2026-04-06-001 active
    govctl work move WI-2026-04-06-001 done

NOTES:
    - `done` requires acceptance criteria and effective guards to pass.
    - Use `work tick` to update acceptance-criteria status.
")]
    Move {
        /// Work item file path or ID
        #[arg(value_name = "FILE_OR_ID")]
        file: PathBuf,
        /// Target status
        #[arg(value_enum)]
        status: WorkItemStatus,
    },
    /// Tick acceptance criteria item
    #[command(after_help = "\
EXAMPLES:
    govctl work tick WI-2026-04-06-001 acceptance_criteria \"Criterion 1\"
    govctl work tick WI-2026-04-06-001 acceptance_criteria --at 0 -s cancelled

NOTES:
    - Valid work-item statuses are `done`, `pending`, and `cancelled`.
    - Omitting `-s/--status` defaults to `done`.
")]
    Tick(WorkTickArgs),
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
