use clap::Subcommand;

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum LoopCommand {
    /// List persisted local loop states
    #[command(after_help = "\
EXAMPLES:
    govctl loop list
    govctl loop list open
    govctl loop list paused -n 5
    govctl loop list -o plain
    govctl loop list -o json

NOTES:
    - Reads local state from `.govctl/loops/<LOOP-ID>/state.toml`.
    - Lists loops by canonical loop ID in deterministic order.
    - Filter may be a loop lifecycle state, `open`, `resumable`, loop ID, or work item ID.
")]
    List {
        /// Optional lifecycle state, alias, loop ID, or work item ID filter
        filter: Option<String>,
        /// Limit number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: crate::OutputFormat,
    },
    /// Start a loop for one or more explicit work items
    #[command(after_help = "\
EXAMPLES:
    govctl loop start WI-2026-04-06-001
    govctl loop start --id LOOP-2026-04-06-001 WI-2026-04-06-001 WI-2026-04-06-002

NOTES:
    - Resolves transitive `depends_on` dependencies before writing state.
    - Reuses an existing non-terminal loop with the same explicit work set when unambiguous.
")]
    Start {
        /// Optional loop ID; generated when omitted
        #[arg(long)]
        id: Option<String>,
        /// Explicit loop work item IDs
        #[arg(required = true, value_name = "WI-ID")]
        work_ids: Vec<String>,
    },
    /// Show persisted loop state
    #[command(after_help = "\
EXAMPLES:
    govctl loop show LOOP-2026-04-06-001
")]
    Show {
        /// Loop ID
        id: String,
    },
    /// Resume or inspect an existing non-terminal loop
    #[command(after_help = "\
EXAMPLES:
    govctl loop resume LOOP-2026-04-06-001

NOTES:
    - Resumes by explicit loop ID.
")]
    Resume {
        /// Loop ID
        id: String,
    },
    /// Recompute dependency closure for the current explicit work set
    #[command(after_help = "\
EXAMPLES:
    govctl loop replan LOOP-2026-04-06-001

NOTES:
    - Re-reads current work item files and preserves applicable loop item state.
")]
    Replan {
        /// Loop ID
        id: String,
    },
    /// Add a value to a loop field and replan when needed
    #[command(after_help = "\
EXAMPLES:
    govctl loop add LOOP-2026-04-06-001 work WI-2026-04-06-002
    govctl loop add LOOP-2026-04-06-001 wi WI-2026-04-06-002

NOTES:
    - `work` is the editable loop work item field.
    - `wi` is accepted as a shorthand field alias.
    - The resolved dependency closure is recomputed after changing work.
")]
    Add {
        /// Loop ID
        id: String,
        /// Loop field name (`work`; `wi` alias)
        field: String,
        /// Work item ID to add
        #[arg(value_name = "WI-ID")]
        value: String,
    },
    /// Remove a value from a loop field and replan when needed
    #[command(after_help = "\
EXAMPLES:
    govctl loop remove LOOP-2026-04-06-001 work WI-2026-04-06-002
    govctl loop remove LOOP-2026-04-06-001 wi WI-2026-04-06-002

NOTES:
    - `work` is the editable loop work item field.
    - `wi` is accepted as a shorthand field alias.
    - The resolved dependency closure is recomputed after changing work.
")]
    Remove {
        /// Loop ID
        id: String,
        /// Loop field name (`work`; `wi` alias)
        field: String,
        /// Work item ID to remove
        #[arg(value_name = "WI-ID")]
        value: String,
    },
    /// Advance the local round protocol for ready work items
    #[command(after_help = "\
EXAMPLES:
    govctl loop run LOOP-2026-04-06-001
    govctl loop run LOOP-2026-04-06-001 --max-rounds 2
    govctl loop run LOOP-2026-04-06-001 --work WI-2026-04-06-002

NOTES:
    - Opens a loop-level round when no open round exists.
    - Validates summary evidence when an open round exists.
    - Use --work to select target work items inside the loop when opening a round.
    - Does not change Work Item lifecycle state; use `govctl work move` explicitly.
")]
    Run {
        /// Loop ID
        id: String,
        /// Maximum rounds each work item may run before loop-level failure
        #[arg(long, default_value_t = 1)]
        max_rounds: u32,
        /// Work item IDs to target inside an existing explicit loop
        #[arg(long = "work", value_name = "WI-ID")]
        target_work_ids: Vec<String>,
    },
}
