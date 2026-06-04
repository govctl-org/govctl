use clap::Subcommand;

use crate::{
    AdrAddArgs, AdrEditArgs, AdrTickArgs, CommonDeprecateArgs, CommonGetArgs, CommonIdArgs,
    CommonListArgs, CommonRemoveArgs, CommonRenderArgs, CommonSetArgs, CommonShowArgs,
    CommonSupersedeArgs,
};

/// ADR commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum AdrCommand {
    /// List ADRs
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be an ADR status, ADR ID, or title substring.

EXAMPLES:
    govctl adr list
    govctl adr list proposed
    govctl adr list ADR-0038 -o json
"
    )]
    List(CommonListArgs),
    /// Get ADR metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, date, status, superseded_by
    - context, decision, consequences, refs, alternatives

EXAMPLES:
    govctl adr get ADR-0001
    govctl adr get ADR-0001 decision
    govctl adr get ADR-0001 alternatives[0].status
")]
    Get(CommonGetArgs),
    /// Show rendered ADR content
    #[command(after_help = "\
EXAMPLES:
    govctl adr show ADR-0001
    govctl adr show ADR-0001 -o plain
")]
    Show(CommonShowArgs),
    /// Create a new ADR
    #[command(after_help = "\
EXAMPLES:
    govctl adr new \"Adopt PostgreSQL for primary storage\"

NOTES:
    - New ADRs start in proposed state.
    - Follow the alternatives-first workflow: add alternatives, discuss, then decide.
")]
    New {
        /// ADR title
        title: String,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl adr edit ADR-0001 content.decision --set \"We will ...\"
    govctl adr edit ADR-0001 content.consequences --set \"Trade-off summary\"
    govctl adr edit ADR-0001 content.alternatives --add \"Option A\"
    govctl adr edit ADR-0001 content.alternatives[0].pros --add \"Readable\"
    govctl adr edit ADR-0001 alternatives --tick accepted --at 0
")]
    Edit(AdrEditArgs),
    /// Set ADR field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - context: Background and problem description
    - decision: The decision made and rationale
    - consequences: Impact of this decision
    - title: ADR title
    - date: ADR date

  Array fields (use 'add'/'remove' instead):
    - refs, alternatives

EXAMPLES:
    govctl adr set ADR-0001 context \"New context\"
    govctl adr set ADR-0001 decision --stdin <<'EOF'
    Multi-line decision here
    EOF

Use dedicated lifecycle verbs instead of `set` for:
    - status / superseded_by → `govctl adr accept` / `govctl adr reject` / `govctl adr supersede`
")]
    Set(CommonSetArgs),
    /// Add value to ADR array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to RFCs/ADRs (e.g., \"RFC-0001\", \"ADR-0002\")
    - alternatives: Options that were considered

ALTERNATIVES FORMAT (per ADR-0027):
    Each alternative has:
    - text: Description of the option (required)
    - status: considered | accepted | rejected
    - pros: Advantages (use --pro to add)
    - cons: Disadvantages (use --con to add)
    - rejection_reason: Why rejected (use --reject-reason)

EXAMPLES:
    govctl adr add ADR-0001 refs RFC-0001
    govctl adr add ADR-0001 alternatives \"Option A: Use PostgreSQL\"
    govctl adr add ADR-0001 alternatives \"Option B: Use Redis\" --pro \"Fast caching\" --con \"Additional infrastructure\"
    govctl adr add ADR-0001 alternatives \"Option C: No cache\" --reject-reason \"Performance issues\"
")]
    Add(AdrAddArgs),
    /// Remove value from ADR array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, alternatives

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl adr remove ADR-0001 refs RFC-0001     # Remove first match
    govctl adr remove ADR-0001 refs --at 1       # Remove by index
")]
    Remove(CommonRemoveArgs),
    /// Accept ADR (proposed → accepted)
    #[command(after_help = "\
EXAMPLES:
    govctl adr accept ADR-0001
    govctl adr accept ADR-0001 --force   # bypass completeness checks

NOTES:
    - Use this when discussion is complete and the ADR becomes governing.
    - Mark the selected alternative as `accepted` before accepting the ADR.
    - Requires at least 2 alternatives (1 accepted, 1 rejected) per [[ADR-0042]].
    - Use --force for historical backfills where alternatives cannot be reconstructed.
")]
    Accept {
        /// ADR ID
        id: String,
        /// Bypass alternatives-completeness checks (for historical backfills)
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Reject ADR (proposed → rejected)
    #[command(after_help = "\
EXAMPLES:
    govctl adr reject ADR-0001

NOTES:
    - Reject the ADR itself when the proposal should not proceed.
    - Use `adr tick ... -s rejected` to reject a specific alternative instead.
")]
    Reject(CommonIdArgs),
    /// Explain that ADRs must be superseded, not deprecated
    #[command(after_help = "\
NOTES:
    - ADRs cannot be deprecated; use `govctl adr supersede ADR-0001 --by ADR-0002` when a newer ADR replaces it.
    - Use `govctl adr reject ADR-0001` for a proposal that should not proceed.
")]
    Deprecate(CommonDeprecateArgs),
    /// Supersede ADR
    #[command(after_help = "\
EXAMPLES:
    govctl adr supersede ADR-0001 --by ADR-0002
    govctl adr supersede ADR-0001 --by ADR-0002 --force
")]
    Supersede(CommonSupersedeArgs),
    /// Update ADR alternative status
    #[command(after_help = "\
EXAMPLES:
    govctl adr tick ADR-0001 alternatives \"Option A\" -s accepted
    govctl adr tick ADR-0001 alternatives --at 1 -s rejected
    govctl adr tick ADR-0001 alternatives --at 0 -s considered

NOTES:
    - ADR tick applies to alternative status, not checklist state.
    - Valid ADR statuses are `accepted`, `considered`, and `rejected`.
")]
    Tick(AdrTickArgs),
    /// Render a single ADR to markdown
    #[command(after_help = "\
EXAMPLES:
    govctl adr render ADR-0001
    govctl adr render ADR-0001 --dry-run
")]
    Render(CommonRenderArgs),
}
