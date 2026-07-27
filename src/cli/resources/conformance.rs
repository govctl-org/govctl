use clap::Subcommand;

use crate::{
    CommonDeleteArgs, CommonEditArgs, CommonGetArgs, CommonListArgs, CommonShowArgs,
    TraceOutputFormat,
};

/// Conformance Case commands.
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ConformanceCommand {
    /// List Conformance Cases
    List(CommonListArgs),
    /// Get Conformance Case metadata or one field
    Get(CommonGetArgs),
    /// Show the complete current Conformance Case
    Show(CommonShowArgs),
    /// Create a Conformance Case
    New {
        /// Case title
        title: String,
        /// Project-relative scenario file path
        #[arg(long)]
        path: String,
        /// Project-defined locator, or * for the complete file
        #[arg(long)]
        selector: String,
        /// Clause and RFC version binding, as CLAUSE-ID@VERSION
        #[arg(long = "requirement", required = true)]
        requirements: Vec<String>,
        /// Verification Guard ID
        #[arg(long = "guard")]
        guards: Vec<String>,
        /// Explicit CONF-* identifier
        #[arg(long)]
        id: Option<String>,
    },
    /// Edit a Conformance Case through canonical logical paths
    Edit(CommonEditArgs),
    /// Delete a Conformance Case
    Delete(CommonDeleteArgs),
    /// Query declared requirement-to-Case-to-Guard traceability
    Trace {
        /// RFC, Clause, Conformance Case, or Verification Guard ID
        target: Option<String>,
        /// Output format; defaults to table in a TTY and JSON otherwise
        #[arg(short = 'o', long, value_enum)]
        output: Option<TraceOutputFormat>,
    },
}
