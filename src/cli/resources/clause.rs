use std::path::PathBuf;

use clap::Subcommand;

use crate::TickStatus;
use crate::model::ClauseKind;
use crate::{
    CommonDeleteArgs, CommonDeprecateArgs, CommonGetArgs, CommonListArgs, CommonSetArgs,
    CommonShowArgs, CommonSupersedeArgs,
};

/// Clause commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ClauseCommand {
    /// List clauses
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be a clause kind, status, clause ID, or title substring.

EXAMPLES:
    govctl clause list
    govctl clause list normative
    govctl clause list RFC-0002:C-SCOPE
"
    )]
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
    govctl clause show RFC-0001:C-SCOPE -o plain
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

LEGACY SUGAR:
    govctl clause edit RFC-0001:C-SUMMARY --text \"Updated clause text\"
    govctl clause edit RFC-0001:C-SUMMARY --stdin
")]
    Edit {
        /// Clause ID
        id: String,
        /// Canonical field path (`text`, `title`, `kind`, or `anchors`)
        path: Option<String>,
        /// Set a scalar value (omit VALUE only when using --stdin)
        #[arg(long, group = "clause_edit_action", num_args = 0..=1)]
        set: Option<Option<String>>,
        /// Append a value to a list (omit VALUE only when using --stdin)
        #[arg(long, group = "clause_edit_action", num_args = 0..=1)]
        add: Option<Option<String>>,
        /// Remove a matching value, or omit PATTERN when removing an indexed path
        #[arg(long, group = "clause_edit_action", num_args = 0..=1)]
        remove: Option<Option<String>>,
        /// Update checklist-style item status
        #[arg(long, group = "clause_edit_action")]
        tick: Option<TickStatus>,
        /// Read set/add value from stdin
        #[arg(long)]
        stdin: bool,
        /// Match by index for remove/tick
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match for remove/tick
        #[arg(long)]
        exact: bool,
        /// Regex match for remove/tick
        #[arg(long)]
        regex: bool,
        /// Remove all matches
        #[arg(long)]
        all: bool,
        /// Legacy sugar: set text directly
        #[arg(long, group = "text_source")]
        text: Option<String>,
        /// Legacy sugar: read text from file
        #[arg(long, group = "text_source")]
        text_file: Option<PathBuf>,
    },
    /// Set clause field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set' or `edit ... --set`):
    - title: Clause title
    - kind: Clause kind (normative|informative)
    - text: Clause text content

  Array fields (use 'add' / 'remove' or `edit ... --add/--remove`):
    - anchors: Cross-reference anchors

EXAMPLES:
    govctl clause set RFC-0001:C-SUMMARY title \"New Title\"
    govctl clause set RFC-0001:C-SUMMARY kind informative
    govctl clause set RFC-0001:C-SUMMARY text --stdin

Use dedicated verbs instead of `set` for:
    - status / superseded_by → `govctl clause deprecate` / `govctl clause supersede`
    - since → `govctl rfc bump` / `govctl rfc finalize`
")]
    Set(CommonSetArgs),
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
