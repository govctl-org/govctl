use clap::Subcommand;

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum LoopCommand {
    /// List persisted local loop states
    #[command(after_help = "\
EXAMPLES:
    govctl loop list
    govctl loop list -o plain
    govctl loop list -o json

NOTES:
    - Reads local state from `.govctl/loops/<LOOP-ID>/state.toml`.
    - Lists loops by canonical loop ID in deterministic order.
")]
    List {
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
    - Reuses an existing non-terminal loop with the same root set when unambiguous.
")]
    Start {
        /// Optional loop ID; generated when omitted
        #[arg(long)]
        id: Option<String>,
        /// Explicit root work item IDs
        #[arg(required = true, value_name = "WI-ID")]
        work_items: Vec<String>,
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
    govctl loop resume --id LOOP-2026-04-06-001
    govctl loop resume WI-2026-04-06-001

NOTES:
    - With `--id`, resumes that explicit loop.
    - Without `--id`, searches for exactly one non-terminal loop with the same root set.
")]
    Resume {
        /// Explicit loop ID
        #[arg(long)]
        id: Option<String>,
        /// Root work item IDs for discovery when --id is omitted
        #[arg(value_name = "WI-ID")]
        work_items: Vec<String>,
    },
    /// Recompute dependency closure for the current root set
    #[command(after_help = "\
EXAMPLES:
    govctl loop replan --id LOOP-2026-04-06-001

NOTES:
    - Re-reads current work item files and preserves applicable loop item state.
")]
    Replan {
        /// Explicit loop ID
        #[arg(long)]
        id: String,
    },
    /// Add root work items to an existing loop and replan
    #[command(after_help = "\
EXAMPLES:
    govctl loop add --id LOOP-2026-04-06-001 WI-2026-04-06-002

NOTES:
    - Added roots become part of loop.root_work_items.
    - The resolved dependency closure is recomputed after adding roots.
")]
    Add {
        /// Explicit loop ID
        #[arg(long)]
        id: String,
        /// Root work item IDs to add
        #[arg(required = true, value_name = "WI-ID")]
        work_items: Vec<String>,
    },
    /// Remove root work items from an existing loop and replan
    #[command(after_help = "\
EXAMPLES:
    govctl loop remove --id LOOP-2026-04-06-001 WI-2026-04-06-002

NOTES:
    - Removed roots leave current loop state when no other root depends on them.
    - The resolved dependency closure is recomputed after removing roots.
")]
    Remove {
        /// Explicit loop ID
        #[arg(long)]
        id: String,
        /// Root work item IDs to remove
        #[arg(required = true, value_name = "WI-ID")]
        work_items: Vec<String>,
    },
    /// Run one execution round for each currently executable work item
    #[command(after_help = "\
EXAMPLES:
    govctl loop run WI-2026-04-06-001
    govctl loop run --id LOOP-2026-04-06-001
    govctl loop run --id LOOP-2026-04-06-001 --max-rounds 2 WI-2026-04-06-001

NOTES:
    - Starts a new loop when no matching non-terminal loop exists.
    - Resumes existing loop state by explicit ID or unambiguous root set.
    - Uses `govctl work move` semantics for work item status transitions.
")]
    Run {
        /// Explicit loop ID; generated when omitted and a new loop is started
        #[arg(long)]
        id: Option<String>,
        /// Maximum rounds each work item may run before loop-level failure
        #[arg(long, default_value_t = 1)]
        max_rounds: u32,
        /// Root work item IDs for start/discovery when needed
        #[arg(value_name = "WI-ID")]
        work_items: Vec<String>,
    },
}
