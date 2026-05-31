use crate::model::{ChangelogCategory, ClauseKind};
use clap::{Args, Subcommand, ValueEnum};

#[derive(Args, Clone, Debug)]
pub(crate) struct EditActionArgs {
    /// Set a scalar value (omit VALUE only when using --stdin)
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) set: Option<Option<String>>,
    /// Append a value to a list (omit VALUE only when using --stdin)
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) add: Option<Option<String>>,
    /// Remove a matching value, or omit PATTERN when removing an indexed path
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) remove: Option<Option<String>>,
    /// Update checklist-style item status
    #[arg(long, group = "edit_action")]
    pub(crate) tick: Option<TickStatus>,
    /// Read set/add value from stdin
    #[arg(long)]
    pub(crate) stdin: bool,
    /// Match by index for remove/tick
    #[arg(long, allow_hyphen_values = true)]
    pub(crate) at: Option<i32>,
    /// Exact match for remove/tick
    #[arg(long)]
    pub(crate) exact: bool,
    /// Regex match for remove/tick
    #[arg(long)]
    pub(crate) regex: bool,
    /// Remove all matches
    #[arg(long)]
    pub(crate) all: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum TickStatus {
    /// Mark work items as done
    Done,
    /// Mark work items as pending
    Pending,
    /// Mark work items as cancelled
    Cancelled,
    /// Mark ADR alternatives as accepted
    Accepted,
    /// Mark ADR alternatives as considered
    Considered,
    /// Mark ADR alternatives as rejected
    Rejected,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum WorkTickStatus {
    /// Mark work items as done
    Done,
    /// Mark work items as pending
    Pending,
    /// Mark work items as cancelled
    Cancelled,
}

impl From<WorkTickStatus> for TickStatus {
    fn from(value: WorkTickStatus) -> Self {
        match value {
            WorkTickStatus::Done => TickStatus::Done,
            WorkTickStatus::Pending => TickStatus::Pending,
            WorkTickStatus::Cancelled => TickStatus::Cancelled,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum AdrTickStatus {
    /// Mark ADR alternatives as accepted
    Accepted,
    /// Mark ADR alternatives as considered
    Considered,
    /// Mark ADR alternatives as rejected
    Rejected,
}

impl From<AdrTickStatus> for TickStatus {
    fn from(value: AdrTickStatus) -> Self {
        match value {
            AdrTickStatus::Accepted => TickStatus::Accepted,
            AdrTickStatus::Considered => TickStatus::Considered,
            AdrTickStatus::Rejected => TickStatus::Rejected,
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum NewTarget {
    /// Create a new RFC
    Rfc {
        /// RFC title
        title: String,
        /// RFC ID (e.g., RFC-0010). Auto-generated if omitted.
        #[arg(long)]
        id: Option<String>,
    },
    /// Create a new clause
    Clause {
        /// Clause ID (e.g., RFC-0010:C-SCOPE)
        clause_id: String,
        /// Clause title
        title: String,
        /// Section to add the clause to
        #[arg(short = 's', long, default_value = "Specification")]
        section: String,
        /// Clause kind
        #[arg(short = 'k', long, value_enum, default_value = "normative")]
        kind: ClauseKind,
    },
    /// Create a new ADR
    Adr {
        /// ADR title
        title: String,
    },
    /// Create a new work item
    Work {
        /// Work item title
        title: String,
        /// Immediately activate the work item
        #[arg(long)]
        active: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListTarget {
    Rfc,
    Clause,
    Adr,
    Work,
    Guard,
}

/// Output format for list/get commands per [[ADR-0017]]
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Table format (default)
    #[default]
    Table,
    /// JSON format
    Json,
    /// Plain text (one item per line)
    Plain,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub(crate) enum RenderTarget {
    /// Render RFCs only (default, published to repo)
    Rfc,
    /// Render ADRs (local only, .gitignore'd)
    Adr,
    /// Render Work Items (local only, .gitignore'd)
    Work,
    /// Render CHANGELOG.md from completed work items
    Changelog,
    /// Render all artifact types (local use)
    All,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub(crate) enum FinalizeStatus {
    Normative,
    Deprecated,
}

// ========================================
// Resource-First Commands (RFC-0002)
// ========================================

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonListArgs {
    /// Optional status, phase, or ID-pattern filter
    pub(crate) filter: Option<String>,
    /// Limit number of results
    #[arg(short = 'n', long)]
    pub(crate) limit: Option<usize>,
    /// Output format
    #[arg(short = 'o', long, value_enum, default_value = "table")]
    pub(crate) output: OutputFormat,
    /// Filter by tag (comma-separated, artifact must have ALL specified tags)
    #[arg(long)]
    pub(crate) tag: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonGetArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Field name or path (omit to show all)
    pub(crate) field: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonShowArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Output format
    #[arg(short = 'o', long, value_enum, default_value = "table")]
    pub(crate) output: OutputFormat,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonEditArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Canonical field path
    pub(crate) path: String,
    #[command(flatten)]
    pub(crate) action: EditActionArgs,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonSetArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Field name
    pub(crate) field: String,
    /// New value (omit if using --stdin)
    #[arg(required_unless_present = "stdin")]
    pub(crate) value: Option<String>,
    /// Read value from stdin
    #[arg(long)]
    pub(crate) stdin: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonAddArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Array field name
    pub(crate) field: String,
    /// Value to add (optional if --stdin)
    pub(crate) value: Option<String>,
    /// Read value from stdin
    #[arg(long)]
    pub(crate) stdin: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonRemoveArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Array field name or nested path
    pub(crate) field: String,
    /// Pattern to match
    pub(crate) pattern: Option<String>,
    /// Remove by index
    #[arg(long, allow_hyphen_values = true)]
    pub(crate) at: Option<i32>,
    /// Exact match
    #[arg(long)]
    pub(crate) exact: bool,
    /// Regex pattern
    #[arg(long)]
    pub(crate) regex: bool,
    /// Remove all matches
    #[arg(long)]
    pub(crate) all: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonTickSelectorArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Field path
    pub(crate) field: String,
    /// Pattern to match
    pub(crate) pattern: Option<String>,
    /// Match by index
    #[arg(long, allow_hyphen_values = true)]
    pub(crate) at: Option<i32>,
    /// Exact match
    #[arg(long)]
    pub(crate) exact: bool,
    /// Regex pattern
    #[arg(long)]
    pub(crate) regex: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct AdrTickArgs {
    #[command(flatten)]
    pub(crate) common: CommonTickSelectorArgs,
    /// New status
    #[arg(short, long, value_enum)]
    pub(crate) status: AdrTickStatus,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct WorkTickArgs {
    #[command(flatten)]
    pub(crate) common: CommonTickSelectorArgs,
    /// New status
    #[arg(short, long, value_enum, default_value = "done")]
    pub(crate) status: WorkTickStatus,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonRenderArgs {
    /// Artifact ID to render
    pub(crate) id: String,
    /// Dry run: show what would be written
    #[arg(long)]
    pub(crate) dry_run: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonDeleteArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Force deletion without confirmation
    #[arg(short = 'f', long)]
    pub(crate) force: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonDeprecateArgs {
    /// Artifact ID
    pub(crate) id: String,
    /// Force without confirmation
    #[arg(short = 'f', long)]
    pub(crate) force: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonSupersedeArgs {
    /// Artifact ID to supersede
    pub(crate) id: String,
    /// Replacement artifact ID
    #[arg(long)]
    pub(crate) by: String,
    /// Force without confirmation
    #[arg(short = 'f', long)]
    pub(crate) force: bool,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonIdArgs {
    /// Artifact ID
    pub(crate) id: String,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct AdrEditArgs {
    #[command(flatten)]
    pub(crate) common: CommonEditArgs,
    /// Pro/advantage for alternative creation (compatibility with `adr add`)
    #[arg(long)]
    pub(crate) pro: Vec<String>,
    /// Con/disadvantage for alternative creation (compatibility with `adr add`)
    #[arg(long)]
    pub(crate) con: Vec<String>,
    /// Rejection reason for alternative creation (compatibility with `adr add`)
    #[arg(long)]
    pub(crate) reject_reason: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct AdrAddArgs {
    #[command(flatten)]
    pub(crate) common: CommonAddArgs,
    /// Pro/advantage for this alternative (can be specified multiple times)
    #[arg(long)]
    pub(crate) pro: Vec<String>,
    /// Con/disadvantage for this alternative (can be specified multiple times)
    #[arg(long)]
    pub(crate) con: Vec<String>,
    /// Reason for rejection (if rejected)
    #[arg(long)]
    pub(crate) reject_reason: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct WorkEditArgs {
    #[command(flatten)]
    pub(crate) common: CommonEditArgs,
    /// Changelog category for acceptance-criteria creation
    #[arg(short = 'c', long, value_enum)]
    pub(crate) category: Option<ChangelogCategory>,
    /// Deprecated compatibility flag; hidden from help
    #[arg(long, hide = true)]
    pub(crate) scope: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct WorkAddArgs {
    #[command(flatten)]
    pub(crate) common: CommonAddArgs,
    /// Changelog category for acceptance_criteria (alternative to prefix)
    #[arg(short = 'c', long, value_enum)]
    pub(crate) category: Option<ChangelogCategory>,
    /// Deprecated compatibility flag; hidden from help
    #[arg(long, hide = true)]
    pub(crate) scope: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct GuardAddArgs {
    /// Guard ID
    pub(crate) id: String,
    /// Array field name
    pub(crate) field: String,
    /// Value to add
    pub(crate) value: String,
}
