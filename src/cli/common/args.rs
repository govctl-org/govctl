use clap::Args;

use super::actions::EditActionArgs;
use super::targets::{GetOutputFormat, ListOutputFormat, ShowOutputFormat};

#[derive(Args, Clone, Debug)]
pub(crate) struct CommonListArgs {
    /// Optional status, phase, or ID-pattern filter
    pub(crate) filter: Option<String>,
    /// Limit number of results
    #[arg(short = 'n', long)]
    pub(crate) limit: Option<usize>,
    /// Output format; defaults to table in a TTY and JSON otherwise
    #[arg(short = 'o', long, value_enum)]
    pub(crate) output: Option<ListOutputFormat>,
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
    /// Output format; valid formats depend on whether FIELD is present
    #[arg(short = 'o', long, value_enum)]
    pub(crate) output: Option<GetOutputFormat>,
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
