use clap::Subcommand;

use crate::model::ClauseKind;
use crate::{
    CommonDeleteArgs, CommonDeprecateArgs, CommonEditArgs, CommonGetArgs, CommonListArgs,
    CommonShowArgs, CommonSupersedeArgs,
};

/// Clause commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ClauseCommand {
    /// List clauses
    #[command(after_help = "\
FILTERS:
    Filter may be a clause kind, status, clause ID, or title substring.

EXAMPLES:
    govctl clause list
    govctl clause list normative
    govctl clause list RFC-0002:C-SCOPE
")]
    List(CommonListArgs),
    /// Get clause metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, kind, text, status, anchors, superseded_by, since

EXAMPLES:
    govctl clause get RFC-0001:C-SCOPE
    govctl clause get RFC-0001:C-SCOPE text
    govctl clause get RFC-0001:C-SCOPE anchors
")]
    Get(CommonGetArgs),
    /// Show rendered clause content
    #[command(after_help = "\
EXAMPLES:
    govctl clause show RFC-0001:C-SCOPE
    govctl clause show RFC-0001:C-SCOPE --history
    govctl clause show RFC-0001:C-SCOPE -o plain

NOTES:
    - Human-readable `show` hides deprecated or superseded Clause text by default.
    - Use `--history` for complete archival content; structured output is always complete.
")]
    Show(CommonShowArgs),
    /// Create a new clause
    #[command(after_help = "\
EXAMPLES:
    govctl clause new RFC-0001:C-SCOPE \"Scope\"
    govctl clause new RFC-0001:C-SCOPE \"Scope\" --section Specification --kind normative

NOTES:
    - Clause IDs are scoped to an RFC: `RFC-XXXX:C-NAME`.
    - Use `--kind informative` for explanatory clauses.
")]
    New {
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
    /// Canonical path-first clause edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl clause edit RFC-0001:C-SUMMARY text --set \"Updated clause text\"
    govctl clause edit RFC-0001:C-SUMMARY text --stdin
    govctl clause edit RFC-0001:C-SUMMARY title --set \"New Title\"
    govctl clause edit RFC-0001:C-SUMMARY kind --set informative

")]
    Edit(CommonEditArgs),
    /// Delete clause
    #[command(after_help = "\
EXAMPLES:
    govctl clause delete RFC-0001:C-SCOPE
    govctl clause delete RFC-0001:C-SCOPE --force

NOTES:
    - Draft Clauses may be deleted when unreferenced.
    - A normative spec Clause may be deleted only when its since version equals the RFC's current version.
    - Inherited or sealed Clauses must be deprecated or superseded instead.
")]
    Delete(CommonDeleteArgs),
    /// Deprecate clause
    #[command(after_help = "\
EXAMPLES:
    govctl clause deprecate RFC-0001:C-SCOPE
    govctl clause deprecate RFC-0001:C-SCOPE --force
")]
    Deprecate(CommonDeprecateArgs),
    /// Supersede clause
    #[command(after_help = "\
EXAMPLES:
    govctl clause supersede RFC-0001:C-SCOPE --by RFC-0001:C-NEW-SCOPE
    govctl clause supersede RFC-0001:C-SCOPE --by RFC-0001:C-NEW-SCOPE --force
")]
    Supersede(CommonSupersedeArgs),
}
