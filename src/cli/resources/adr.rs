use clap::Subcommand;

use crate::{
    CommonDeprecateArgs, CommonEditArgs, CommonGetArgs, CommonIdArgs, CommonListArgs,
    CommonRenderArgs, CommonShowArgs, CommonSupersedeArgs,
};

/// ADR commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum AdrCommand {
    /// List ADRs
    #[command(after_help = "\
FILTERS:
    Filter may be an ADR status, ADR ID, or title substring.

EXAMPLES:
    govctl adr list
    govctl adr list proposed
    govctl adr list ADR-0038 -o json
")]
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
    govctl adr show ADR-0001 --history
    govctl adr show ADR-0001 -o plain

NOTES:
    - Human-readable `show` hides superseded ADR bodies by default.
    - Use `--history` for complete archival content; structured output is always complete.
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
    govctl adr edit ADR-0001 decision --set \"We will ...\"
    govctl adr edit ADR-0001 consequences --set \"Trade-off summary\"
    govctl adr edit ADR-0001 alternatives --add \"Option A\"
    govctl adr edit ADR-0001 alternatives[0].pros --add \"Readable\"
    govctl adr edit ADR-0001 alternatives[0] --tick accepted
")]
    Edit(CommonEditArgs),
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
    - Use `govctl adr edit ADR-0001 alternatives[N] --tick rejected` to reject a specific alternative instead.
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
    /// Render a single ADR to markdown
    #[command(after_help = "\
EXAMPLES:
    govctl adr render ADR-0001
    govctl adr render ADR-0001 --dry-run
")]
    Render(CommonRenderArgs),
}
