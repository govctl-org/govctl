use std::path::PathBuf;

use clap::Subcommand;

use crate::model::{ClauseKind, RfcPhase, WorkItemStatus};

use super::{
    AdrAddArgs, AdrEditArgs, AdrTickArgs, CommonAddArgs, CommonDeleteArgs, CommonDeprecateArgs,
    CommonEditArgs, CommonGetArgs, CommonIdArgs, CommonListArgs, CommonRemoveArgs,
    CommonRenderArgs, CommonSetArgs, CommonShowArgs, CommonSupersedeArgs, FinalizeStatus,
    GuardAddArgs, TickStatus, WorkAddArgs, WorkEditArgs, WorkTickArgs,
};

/// RFC commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum RfcCommand {
    /// List RFCs
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be an RFC status, phase, or ID/title substring.

EXAMPLES:
    govctl rfc list
    govctl rfc list draft
    govctl rfc list impl -n 5
    govctl rfc list RFC-0002 -o json
"
    )]
    List(CommonListArgs),
    /// Get RFC metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, version, status, phase, owners, refs

EXAMPLES:
    govctl rfc get RFC-0001
    govctl rfc get RFC-0001 title
    govctl rfc get RFC-0001 refs
")]
    Get(CommonGetArgs),
    /// Show rendered RFC content
    #[command(after_help = "\
EXAMPLES:
    govctl rfc show RFC-0001
    govctl rfc show RFC-0001 -o plain

NOTES:
    - `show` prints human-readable rendered content.
    - Use `get` for field/path-level inspection.
")]
    Show(CommonShowArgs),
    /// Create a new RFC
    #[command(after_help = "\
EXAMPLES:
    govctl rfc new \"Add incremental index rebuilding\"
    govctl rfc new \"Add incremental index rebuilding\" --id RFC-0010

NOTES:
    - Use `--id` only when you need to pin a specific RFC ID.
    - New RFCs start as draft and can later be finalized.
")]
    New {
        /// RFC title
        title: String,
        /// RFC ID (e.g., RFC-0010). Auto-generated if omitted.
        #[arg(long)]
        id: Option<String>,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl rfc edit RFC-0001 version --set 1.2.0
    govctl rfc edit RFC-0001 refs --add RFC-0002
    govctl rfc edit RFC-0001 refs[0] --remove
")]
    Edit(CommonEditArgs),
    /// Set RFC field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - title: RFC title

  Array fields (use 'add' / 'remove'):
    - owners, refs, sections

EXAMPLES:
    govctl rfc set RFC-0001 title \"New Title\"

Use dedicated lifecycle verbs instead of `set` for:
    - version → `govctl rfc bump`
    - status → `govctl rfc finalize` / `govctl rfc deprecate` / `govctl rfc supersede`
    - phase → `govctl rfc advance`
")]
    Set(CommonSetArgs),
    /// Add value to RFC array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to other RFCs (e.g., \"RFC-0002\")
    - owners: RFC owners (e.g., \"@alice\")

EXAMPLES:
    govctl rfc add RFC-0001 refs RFC-0002
    govctl rfc add RFC-0001 owners @alice
")]
    Add(CommonAddArgs),
    /// Remove value from RFC array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, owners

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl rfc remove RFC-0001 refs RFC-0002     # Remove first match
    govctl rfc remove RFC-0001 refs --at 1       # Remove by index
")]
    Remove(CommonRemoveArgs),
    /// Bump RFC version
    #[command(after_help = "\
EXAMPLES:
    govctl rfc bump RFC-0001 --patch -m \"Clarify examples\"
    govctl rfc bump RFC-0001 --minor -c \"changed: New normative clause\"

NOTES:
    - Exactly one of `--patch`, `--minor`, or `--major` must be chosen.
    - Use `-m/--summary` for a release summary and `-c/--change` for detailed entries.
")]
    Bump {
        /// RFC ID
        id: String,
        /// Patch version bump
        #[arg(long, group = "bump_level")]
        patch: bool,
        /// Minor version bump
        #[arg(long, group = "bump_level")]
        minor: bool,
        /// Major version bump
        #[arg(long, group = "bump_level")]
        major: bool,
        /// Changelog summary
        #[arg(short = 'm', long)]
        summary: Option<String>,
        /// Add change description(s)
        #[arg(short = 'c', long = "change")]
        changes: Vec<String>,
    },
    /// Finalize RFC status (draft → normative or deprecated)
    #[command(after_help = "\
EXAMPLES:
    govctl rfc finalize RFC-0001 normative
    govctl rfc finalize RFC-0001 deprecated

NOTES:
    - Finalize is used once the RFC leaves draft state.
    - Use `advance` to move phase after finalization.
")]
    Finalize {
        /// RFC ID
        id: String,
        /// Target status
        #[arg(value_enum)]
        status: FinalizeStatus,
    },
    /// Advance RFC phase
    #[command(after_help = "\
EXAMPLES:
    govctl rfc advance RFC-0001 impl
    govctl rfc advance RFC-0001 test

NOTES:
    - Typical progression is `spec -> impl -> test -> stable`.
    - Use this after the RFC has been finalized.
")]
    Advance {
        /// RFC ID
        id: String,
        /// Target phase
        #[arg(value_enum)]
        phase: RfcPhase,
    },
    /// Deprecate RFC
    #[command(after_help = "\
EXAMPLES:
    govctl rfc deprecate RFC-0001
    govctl rfc deprecate RFC-0001 --force
")]
    Deprecate(CommonDeprecateArgs),
    /// Supersede RFC
    #[command(after_help = "\
EXAMPLES:
    govctl rfc supersede RFC-0001 --by RFC-0002
    govctl rfc supersede RFC-0001 --by RFC-0002 --force
")]
    Supersede(CommonSupersedeArgs),
    /// Render a single RFC to markdown
    #[command(after_help = "\
EXAMPLES:
    govctl rfc render RFC-0001
    govctl rfc render RFC-0001 --dry-run
")]
    Render(CommonRenderArgs),
}

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

/// Work item commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum WorkCommand {
    /// List work items
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be a work-item status, work-item ID, or title substring.

EXAMPLES:
    govctl work list
    govctl work list active
    govctl work list queue -n 10
"
    )]
    List(CommonListArgs),
    /// Get work item metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, description, status, completed_at, refs, depends_on
    - notes, acceptance_criteria

EXAMPLES:
    govctl work get WI-2026-04-06-001
    govctl work get WI-2026-04-06-001 description
    govctl work get WI-2026-04-06-001 acceptance_criteria[0].status
")]
    Get(CommonGetArgs),
    /// Show rendered work item content
    #[command(after_help = "\
EXAMPLES:
    govctl work show WI-2026-04-06-001
    govctl work show WI-2026-04-06-001 -o plain
")]
    Show(CommonShowArgs),
    /// Create a new work item
    #[command(after_help = "\
EXAMPLES:
    govctl work new \"Implement RFC-0005 parser\"
    govctl work new \"Implement RFC-0005 parser\" --active

NOTES:
    - Use `--active` to immediately start the work item.
    - Add acceptance criteria before moving to `done`.
")]
    New {
        /// Work item title
        title: String,
        /// Immediately activate the work item
        #[arg(long)]
        active: bool,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl work edit WI-2026-04-06-001 description --set \"Scope and why\"
    govctl work edit WI-2026-04-06-001 depends_on --add WI-2026-04-06-002
    govctl work edit WI-2026-04-06-001 content.acceptance_criteria --add \"add: Implement feature X\"
    govctl work edit WI-2026-04-06-001 content.acceptance_criteria[0] --tick done
")]
    Edit(WorkEditArgs),
    /// Set work item field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - description: Task scope declaration
    - title: Work item title

  Array fields (use 'add'/'remove' instead):
    - refs: Cross-references to RFCs/ADRs
    - depends_on: Blocking dependencies on other work items
    - notes: Ad-hoc key points (short strings)
    - acceptance_criteria: Completion criteria with category

FIELD SEMANTICS:
  - description: Task scope - define once, rarely change
  - depends_on: Work item IDs that must complete before this item starts
  - notes: Ad-hoc points - add anytime, keep concise

EXAMPLES:
    govctl work set WI-001 description \"New description\"
    govctl work set WI-001 description --stdin <<'EOF'
    Multi-line description here
    EOF

Use dedicated verbs instead of `set` for:
    - status → `govctl work move`
    - acceptance_criteria[*].status → `govctl work tick`
")]
    Set(CommonSetArgs),
    /// Add value to work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to RFCs/ADRs (e.g., \"RFC-0001\", \"ADR-0002\")
    - depends_on: Blocking dependencies on work items (e.g., \"WI-2026-04-06-001\")
    - notes: Ad-hoc key points (short strings)
    - acceptance_criteria: Completion criteria with category prefix

FIELD SEMANTICS:
  - description: Task scope - define once, rarely change
  - depends_on: Work item IDs only; cyclic dependencies are rejected
  - notes: Ad-hoc points - add anytime, keep concise

ACCEPTANCE CRITERIA FORMAT:
    Use category prefix for changelog generation:
    - \"add: New feature\"       → Added section
    - \"fix: Bug fixed\"         → Fixed section
    - \"changed: Behavior\"      → Changed section
    - \"chore: Tests pass\"      → Excluded from changelog

EXAMPLES:
    govctl work add WI-001 refs RFC-0001
    govctl work add WI-001 depends_on WI-2026-04-06-002
    govctl work add WI-001 acceptance_criteria \"add: Implement feature\"
    govctl work add WI-001 notes \"Remember to test edge cases\"
")]
    Add(WorkAddArgs),
    /// Remove value from work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, depends_on, notes, acceptance_criteria

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl work remove WI-001 refs RFC-0001     # Remove first match
    govctl work remove WI-001 depends_on WI-2026-04-06-002
    govctl work remove WI-001 refs --at 1       # Remove by index
    govctl work remove WI-001 notes --all       # Remove all
")]
    Remove(CommonRemoveArgs),
    /// Move work item to new status
    #[command(visible_alias = "mv")]
    #[command(after_help = "\
EXAMPLES:
    govctl work move WI-2026-04-06-001 active
    govctl work move WI-2026-04-06-001 done

NOTES:
    - `done` requires acceptance criteria and effective guards to pass.
    - Use `work tick` to update acceptance-criteria status.
")]
    Move {
        /// Work item file path or ID
        #[arg(value_name = "FILE_OR_ID")]
        file: PathBuf,
        /// Target status
        #[arg(value_enum)]
        status: WorkItemStatus,
    },
    /// Tick acceptance criteria item
    #[command(after_help = "\
EXAMPLES:
    govctl work tick WI-2026-04-06-001 acceptance_criteria \"Criterion 1\"
    govctl work tick WI-2026-04-06-001 acceptance_criteria --at 0 -s cancelled

NOTES:
    - Valid work-item statuses are `done`, `pending`, and `cancelled`.
    - Omitting `-s/--status` defaults to `done`.
")]
    Tick(WorkTickArgs),
    /// Delete work item
    #[command(after_help = "\
EXAMPLES:
    govctl work delete WI-2026-04-06-001
    govctl work delete WI-2026-04-06-001 --force
")]
    Delete(CommonDeleteArgs),
    /// Render a single work item to markdown
    #[command(after_help = "\
EXAMPLES:
    govctl work render WI-2026-04-06-001
    govctl work render WI-2026-04-06-001 --dry-run
")]
    Render(CommonRenderArgs),
}
/// Guard commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
pub(crate) enum GuardCommand {
    /// List guards
    #[command(
        visible_alias = "ls",
        after_help = "\
FILTERS:
    Filter may be a guard ID or title substring.

EXAMPLES:
    govctl guard list
    govctl guard list clippy
    govctl guard list GUARD-CLIPPY -o json
"
    )]
    List(CommonListArgs),
    /// Get guard metadata or specific field
    #[command(after_help = "\
VALID FIELDS:
    - title, refs
    - description, check.command, check.timeout_secs, check.pattern

EXAMPLES:
    govctl guard get GUARD-0001
    govctl guard get GUARD-0001 description
    govctl guard get GUARD-0001 check.command
")]
    Get(CommonGetArgs),
    /// Show rendered guard content
    #[command(after_help = "\
EXAMPLES:
    govctl guard show GUARD-CLIPPY
    govctl guard show GUARD-CLIPPY -o json
")]
    Show(CommonShowArgs),
    /// Create a new verification guard
    #[command(after_help = "\
EXAMPLES:
    govctl guard new \"clippy lint\"

NOTES:
    - Create the guard first, then use `guard edit` / `guard set` to define checks.
")]
    New {
        /// Guard title
        title: String,
    },
    /// Canonical path-first edit entrypoint
    #[command(after_help = "\
EXAMPLES:
    govctl guard edit GUARD-0001 description --set \"What this guard verifies\"
    govctl guard edit GUARD-0001 refs --add RFC-0001
    govctl guard edit GUARD-0001 refs[0] --remove
")]
    Edit(CommonEditArgs),
    /// Set guard field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use `set`):
    - title
    - description
    - check.command
    - check.pattern
    - check.timeout_secs

EXAMPLES:
    govctl guard set GUARD-0001 description \"Runs clippy on the workspace\"
    govctl guard set GUARD-0001 check.command \"cargo clippy --all-targets -- -D warnings\"
")]
    Set(CommonSetArgs),
    /// Add value to guard array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs

EXAMPLES:
    govctl guard add GUARD-0001 refs RFC-0001
")]
    Add(GuardAddArgs),
    /// Remove value from guard array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl guard remove GUARD-0001 refs RFC-0001
    govctl guard remove GUARD-0001 refs --at 0
")]
    Remove(CommonRemoveArgs),
    /// Delete a verification guard
    #[command(after_help = "\
EXAMPLES:
    govctl guard delete GUARD-CLIPPY
    govctl guard delete GUARD-CLIPPY --force
")]
    Delete(CommonDeleteArgs),
}
