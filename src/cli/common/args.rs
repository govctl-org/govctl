use crate::model::ChangelogCategory;
use clap::Args;

use super::actions::{AdrTickStatus, EditActionArgs, WorkTickStatus};
use super::targets::{OutputFormat, ShowOutputFormat};

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
    pub(crate) output: ShowOutputFormat,
    /// Include complete historical body content in human-readable output
    #[arg(long)]
    pub(crate) history: bool,
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
