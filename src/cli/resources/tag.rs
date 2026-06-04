use clap::Subcommand;

/// Tag management subcommands
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum TagCommand {
    /// Add a new allowed tag to config.toml
    #[command(after_help = "\
EXAMPLES:
    govctl tag new caching
    govctl tag new breaking-change
")]
    New {
        /// Tag name (must match ^[a-z][a-z0-9-]*$)
        tag: String,
    },
    /// Remove an allowed tag from config.toml (fails if any artifact uses it)
    #[command(after_help = "\
EXAMPLES:
    govctl tag delete caching
")]
    Delete {
        /// Tag name to remove
        tag: String,
    },
    /// List all allowed tags and their usage counts
    #[command(
        visible_alias = "ls",
        after_help = "\
EXAMPLES:
    govctl tag list
    govctl tag list -o json
"
    )]
    List {
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: crate::OutputFormat,
    },
}
