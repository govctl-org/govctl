//! govctl: Project governance CLI for RFC, ADR, and Work Item management.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;

mod config;
mod diagnostic;
mod load;
mod lock;
mod model;
mod parse;
mod render;
mod scan;
mod signature;
mod ui;
mod validate;
mod write;

mod cmd;
mod command_router;

#[cfg(feature = "tui")]
mod tui;

use config::Config;
use diagnostic::{Diagnostic, DiagnosticLevel};
use model::{ClauseKind, RfcPhase, WorkItemStatus};

#[derive(Parser)]
#[command(name = "govctl")]
#[command(about = "Project governance CLI for RFC, ADR, and Work Item management")]
#[command(version)]
struct Cli {
    /// Path to govctl config (TOML)
    #[arg(short = 'C', long, global = true)]
    config: Option<PathBuf>,

    /// Dry run: preview changes without writing files
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize govctl in the current directory
    Init {
        /// Overwrite existing config
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Sync .claude/ assets (commands, skills, agents) after upgrading govctl
    #[command(visible_alias = "sync-commands")]
    Sync {
        /// Force overwrite existing assets
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Validate all governed documents
    #[command(visible_alias = "lint")]
    Check {
        /// Treat warnings as errors
        #[arg(short = 'W', long)]
        deny_warnings: bool,
    },

    /// Show summary counts
    #[command(visible_alias = "stat")]
    Status,

    /// Render artifacts to markdown from SSOT (bulk operation)
    ///
    /// For single-item render, use: govctl rfc render <ID>, govctl adr render <ID>, etc.
    #[command(visible_alias = "gen")]
    Render {
        /// What to render: rfc (default), adr, work, changelog, or all
        #[arg(value_enum, default_value = "rfc")]
        target: RenderTarget,
        /// Dry run: show what would be written
        #[arg(long)]
        dry_run: bool,
        /// Force full regeneration (for changelog: overwrites released sections)
        #[arg(long, short)]
        force: bool,
    },

    // ========================================
    // Resource-First Commands (RFC-0002)
    // ========================================
    /// RFC operations
    Rfc {
        #[command(subcommand)]
        command: RfcCommand,
    },

    /// Clause operations
    Clause {
        #[command(subcommand)]
        command: ClauseCommand,
    },

    /// ADR operations
    Adr {
        #[command(subcommand)]
        command: AdrCommand,
    },

    /// Work item operations
    Work {
        #[command(subcommand)]
        command: WorkCommand,
    },

    /// Cut a release (collect unreleased work items into a version)
    Release {
        /// Version number (semver, e.g., 0.2.0)
        version: String,
        /// Release date (defaults to today)
        #[arg(long)]
        date: Option<String>,
    },

    /// Output machine-readable CLI metadata for agents
    Describe {
        /// Include project state and suggested actions
        #[arg(long)]
        context: bool,
        /// Output format (currently only json is supported)
        #[arg(short = 'o', long, default_value = "json")]
        output: String,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Launch interactive TUI dashboard
    #[cfg(feature = "tui")]
    Tui,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum TickStatus {
    /// Mark as done/accepted
    Done,
    /// Mark as pending/considered
    Pending,
    /// Mark as cancelled/rejected
    Cancelled,
}

#[derive(Subcommand, Clone, Debug)]
enum NewTarget {
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

#[derive(ValueEnum, Clone, Copy, Debug)]
enum ListTarget {
    Rfc,
    Clause,
    Adr,
    Work,
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
enum RenderTarget {
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
enum FinalizeStatus {
    Normative,
    Deprecated,
}

// ========================================
// Resource-First Commands (RFC-0002)
// ========================================

/// RFC commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
enum RfcCommand {
    /// Create a new RFC
    New {
        /// RFC title
        title: String,
        /// RFC ID (e.g., RFC-0010). Auto-generated if omitted.
        #[arg(long)]
        id: Option<String>,
    },
    /// List all RFCs
    #[command(
        visible_alias = "ls",
        after_help = "\
EXAMPLES:
    govctl rfc list                    # List all RFCs
    govctl rfc list draft              # Filter by status
    govctl rfc list impl               # Filter by phase
    govctl rfc list -n 5               # Limit to 5 results
    govctl rfc list -o json            # Output as JSON
"
    )]
    List {
        /// Filter by status (draft|normative|deprecated), phase (spec|impl|test|stable), or ID pattern
        filter: Option<String>,
        /// Limit number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
    /// Get RFC field value
    #[command(after_help = "\
VALID FIELDS:
  String fields:
    - title: RFC title
    - version: RFC version (semver)
    - status: RFC status (draft|normative|deprecated)
    - phase: RFC phase (spec|impl|test|stable)

  Array fields (returns JSON array):
    - owners: RFC owners
    - refs: Cross-references to other artifacts
    - sections: RFC sections

EXAMPLES:
    govctl rfc get RFC-0001                    # Show all fields
    govctl rfc get RFC-0001 title              # Get specific field
    govctl rfc get RFC-0001 refs               # Get array field
")]
    Get {
        /// RFC ID
        id: String,
        /// Field name (omit to show all)
        field: Option<String>,
    },
    /// Set RFC field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - title: RFC title
    - version: RFC version (semver)
    - status: RFC status (draft|normative|deprecated)
    - phase: RFC phase (spec|impl|test|stable)

  Array fields (modify via rfc.json directly):
    - owners, refs, sections

EXAMPLES:
    govctl rfc set RFC-0001 title \"New Title\"
    govctl rfc set RFC-0001 version 0.2.0
    govctl rfc set RFC-0001 phase impl
")]
    Set {
        /// RFC ID
        id: String,
        /// Field name
        field: String,
        /// New value (omit if using --stdin)
        #[arg(required_unless_present = "stdin")]
        value: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Bump RFC version
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
    Finalize {
        /// RFC ID
        id: String,
        /// Target status
        #[arg(value_enum)]
        status: FinalizeStatus,
    },
    /// Advance RFC phase
    Advance {
        /// RFC ID
        id: String,
        /// Target phase
        #[arg(value_enum)]
        phase: RfcPhase,
    },
    /// Deprecate RFC
    Deprecate {
        /// RFC ID
        id: String,
        /// Force without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Supersede RFC
    Supersede {
        /// RFC ID to supersede
        id: String,
        /// Replacement RFC ID
        #[arg(long)]
        by: String,
        /// Force without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Render a single RFC to markdown
    Render {
        /// RFC ID to render
        id: String,
        /// Dry run: show what would be written
        #[arg(long)]
        dry_run: bool,
    },
    /// Show RFC content to stdout (no file written)
    Show {
        /// RFC ID
        id: String,
        /// Output format (markdown or json)
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
}

/// Clause commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
enum ClauseCommand {
    /// Create a new clause
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
    /// List clauses
    #[command(
        visible_alias = "ls",
        after_help = "\
EXAMPLES:
    govctl clause list                 # List all clauses
    govctl clause list RFC-0001        # Filter by RFC ID
    govctl clause list active          # Filter by status
    govctl clause list -o json         # Output as JSON
"
    )]
    List {
        /// Filter by RFC ID or clause ID pattern
        filter: Option<String>,
        /// Limit number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
    /// Get clause field value
    #[command(after_help = "\
VALID FIELDS:
  String fields:
    - title: Clause title
    - kind: Clause kind (normative|informative)
    - status: Clause status (active|deprecated|superseded)
    - text: Clause text content
    - since: Version when clause was added

  Array fields (returns JSON array):
    - anchors: Cross-reference anchors

EXAMPLES:
    govctl clause get RFC-0001:C-SUMMARY           # Show all fields
    govctl clause get RFC-0001:C-SUMMARY text      # Get clause text
")]
    Get {
        /// Clause ID (e.g., RFC-0001:C-PHASE-ORDER)
        id: String,
        /// Field name (omit to show all)
        field: Option<String>,
    },
    /// Edit clause text
    Edit {
        /// Clause ID
        id: String,
        /// Set text directly
        #[arg(long, group = "text_source")]
        text: Option<String>,
        /// Read text from file
        #[arg(long, group = "text_source")]
        text_file: Option<PathBuf>,
        /// Read text from stdin (recommended for multi-line)
        #[arg(long, group = "text_source")]
        stdin: bool,
    },
    /// Set clause field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - title: Clause title
    - kind: Clause kind (normative|informative)
    - status: Clause status (active|deprecated|superseded)
    - text: Clause text content (use 'edit' for multi-line)

  Array fields (modify via clause.json directly):
    - anchors

EXAMPLES:
    govctl clause set RFC-0001:C-SUMMARY title \"New Title\"
    govctl clause set RFC-0001:C-SUMMARY kind informative

For editing clause text, prefer 'govctl clause edit' command.
")]
    Set {
        /// Clause ID
        id: String,
        /// Field name
        field: String,
        /// New value (omit if using --stdin)
        #[arg(required_unless_present = "stdin")]
        value: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Delete clause
    Delete {
        /// Clause ID
        id: String,
        /// Force deletion without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Deprecate clause
    Deprecate {
        /// Clause ID
        id: String,
        /// Force without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Supersede clause
    Supersede {
        /// Clause ID to supersede
        id: String,
        /// Replacement clause ID
        #[arg(long)]
        by: String,
        /// Force without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Show clause content to stdout (no file written)
    Show {
        /// Clause ID (e.g., RFC-0001:C-SUMMARY)
        id: String,
        /// Output format (markdown or json)
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
}

/// ADR commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
enum AdrCommand {
    /// Create a new ADR
    New {
        /// ADR title
        title: String,
    },
    /// List ADRs
    #[command(
        visible_alias = "ls",
        after_help = "\
EXAMPLES:
    govctl adr list                    # List all ADRs
    govctl adr list accepted           # Filter by status
    govctl adr list -n 10              # Limit to 10 results
    govctl adr list -o json            # Output as JSON
"
    )]
    List {
        /// Filter by status (proposed|accepted|rejected|superseded|deprecated) or ID pattern
        filter: Option<String>,
        /// Limit number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
    /// Get ADR field value
    #[command(after_help = "\
VALID FIELDS:
  String fields:
    - context: Background and problem description
    - decision: The decision made and rationale
    - consequences: Impact of this decision
    - status: ADR status (proposed|accepted|rejected|superseded)

  Array fields (returns JSON array):
    - refs: Cross-references to RFCs/ADRs
    - alternatives: Options that were considered

EXAMPLES:
    govctl adr get ADR-0001                    # Show all fields
    govctl adr get ADR-0001 context            # Get specific field
    govctl adr get ADR-0001 alternatives       # Get array field
")]
    Get {
        /// ADR ID
        id: String,
        /// Field name (omit to show all)
        field: Option<String>,
    },
    /// Set ADR field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - context: Background and problem description
    - decision: The decision made and rationale
    - consequences: Impact of this decision

  Array fields (use 'add'/'remove' instead):
    - refs, alternatives

EXAMPLES:
    govctl adr set ADR-0001 context \"New context\"
    govctl adr set ADR-0001 decision --stdin <<'EOF'
    Multi-line decision here
    EOF
")]
    Set {
        /// ADR ID
        id: String,
        /// Field name
        field: String,
        /// New value (omit if using --stdin)
        #[arg(required_unless_present = "stdin")]
        value: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Add value to ADR array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to RFCs/ADRs (e.g., \"RFC-0001\", \"ADR-0002\")
    - alternatives: Options that were considered

ALTERNATIVES FORMAT:
    Each alternative can have:
    - text: Description of the option
    - status: considered|accepted|rejected

EXAMPLES:
    govctl adr add ADR-0001 refs RFC-0001
    govctl adr add ADR-0001 alternatives \"Option A: Description\"
")]
    Add {
        /// ADR ID
        id: String,
        /// Array field name (refs, alternatives)
        field: String,
        /// Value to add (optional if --stdin)
        value: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Remove value from ADR array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, alternatives

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (1-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl adr remove ADR-0001 refs RFC-0001     # Remove first match
    govctl adr remove ADR-0001 refs --at 1       # Remove by index
")]
    Remove {
        /// ADR ID
        id: String,
        /// Array field name (refs, alternatives)
        field: String,
        /// Pattern to match
        #[arg(required_unless_present = "at")]
        pattern: Option<String>,
        /// Remove by index
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match
        #[arg(long)]
        exact: bool,
        /// Regex pattern
        #[arg(long)]
        regex: bool,
        /// Remove all matches
        #[arg(long)]
        all: bool,
    },
    /// Accept ADR (proposed → accepted)
    Accept {
        /// ADR ID
        id: String,
    },
    /// Reject ADR (proposed → rejected)
    Reject {
        /// ADR ID
        id: String,
    },
    /// Deprecate ADR
    Deprecate {
        /// ADR ID
        id: String,
        /// Force without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Supersede ADR
    Supersede {
        /// ADR ID to supersede
        id: String,
        /// Replacement ADR ID
        #[arg(long)]
        by: String,
        /// Force without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Tick checklist item
    Tick {
        /// ADR ID
        id: String,
        /// Field (decisions or alternatives)
        field: String,
        /// Pattern to match
        pattern: Option<String>,
        /// New status
        #[arg(short, long, value_enum, default_value = "done")]
        status: TickStatus,
        /// Match by index
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match
        #[arg(long)]
        exact: bool,
        /// Regex pattern
        #[arg(long)]
        regex: bool,
    },
    /// Render a single ADR to markdown
    Render {
        /// ADR ID to render
        id: String,
        /// Dry run: show what would be written
        #[arg(long)]
        dry_run: bool,
    },
    /// Show ADR content to stdout (no file written)
    Show {
        /// ADR ID
        id: String,
        /// Output format (markdown or json)
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
}

/// Work item commands (resource-first structure)
#[derive(Subcommand, Clone, Debug)]
enum WorkCommand {
    /// Create a new work item
    New {
        /// Work item title
        title: String,
        /// Immediately activate the work item
        #[arg(long)]
        active: bool,
    },
    /// List work items
    #[command(
        visible_alias = "ls",
        after_help = "\
EXAMPLES:
    govctl work list                   # List all work items
    govctl work list active            # Filter by status
    govctl work list pending           # Show queue + active
    govctl work list -n 5 -o json      # JSON output, 5 items
"
    )]
    List {
        /// Filter by status (queue|active|done|cancelled) or ID pattern
        filter: Option<String>,
        /// Limit number of results
        #[arg(short = 'n', long)]
        limit: Option<usize>,
        /// Output format
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
    /// Get work item field value
    #[command(after_help = "\
VALID FIELDS:
  String fields:
    - description: Task scope declaration
    - status: Work item status (queue|active|done|cancelled)

  Array fields (returns JSON array):
    - refs: Cross-references to RFCs/ADRs
    - journal: Execution tracking entries
    - notes: Ad-hoc key points
    - acceptance_criteria: Completion criteria

EXAMPLES:
    govctl work get WI-001                    # Show all fields
    govctl work get WI-001 description        # Get specific field
    govctl work get WI-001 refs               # Get array field
")]
    Get {
        /// Work item ID
        id: String,
        /// Field name (omit to show all)
        field: Option<String>,
    },
    /// Set work item field value
    #[command(after_help = "\
VALID FIELDS:
  String fields (use 'set'):
    - description: Task scope declaration
    - status: Work item status (queue|active|done|cancelled)

  Array fields (use 'add'/'remove' instead):
    - refs, journal, notes, acceptance_criteria

EXAMPLES:
    govctl work set WI-001 description \"New description\"
    govctl work set WI-001 status active
    govctl work set WI-001 description --stdin <<'EOF'
    Multi-line description here
    EOF
")]
    Set {
        /// Work item ID
        id: String,
        /// Field name
        field: String,
        /// New value (omit if using --stdin)
        #[arg(required_unless_present = "stdin")]
        value: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
    },
    /// Add value to work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs: Cross-references to RFCs/ADRs (e.g., \"RFC-0001\", \"ADR-0002\")
    - journal: Execution tracking entries (multi-line via --stdin)
    - notes: Ad-hoc key points (short strings)
    - acceptance_criteria: Completion criteria with category prefix

ACCEPTANCE CRITERIA FORMAT:
    Use category prefix for changelog generation:
    - \"add: New feature\"       → Added section
    - \"fix: Bug fixed\"         → Fixed section
    - \"changed: Behavior\"      → Changed section
    - \"chore: Tests pass\"      → Excluded from changelog

EXAMPLES:
    govctl work add WI-001 refs RFC-0001
    govctl work add WI-001 acceptance_criteria \"add: Implement feature\"
    govctl work add WI-001 journal --stdin <<'EOF'
    Progress update for today...
    EOF
")]
    Add {
        /// Work item ID
        id: String,
        /// Array field name (refs, journal, notes, acceptance_criteria)
        field: String,
        /// Value to add (optional if --stdin)
        value: Option<String>,
        /// Read value from stdin
        #[arg(long)]
        stdin: bool,
        /// Changelog category for acceptance_criteria (alternative to prefix)
        #[arg(short = 'c', long, value_enum)]
        category: Option<model::ChangelogCategory>,
    },
    /// Remove value from work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, journal, notes, acceptance_criteria

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (1-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl work remove WI-001 refs RFC-0001     # Remove first match
    govctl work remove WI-001 refs --at 1       # Remove by index
    govctl work remove WI-001 notes --all       # Remove all
")]
    Remove {
        /// Work item ID
        id: String,
        /// Array field name (refs, journal, notes, acceptance_criteria)
        field: String,
        /// Pattern to match
        #[arg(required_unless_present = "at")]
        pattern: Option<String>,
        /// Remove by index
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match
        #[arg(long)]
        exact: bool,
        /// Regex pattern
        #[arg(long)]
        regex: bool,
        /// Remove all matches
        #[arg(long)]
        all: bool,
    },
    /// Move work item to new status
    #[command(visible_alias = "mv")]
    Move {
        /// Work item file path or ID
        #[arg(value_name = "FILE_OR_ID")]
        file: PathBuf,
        /// Target status
        #[arg(value_enum)]
        status: WorkItemStatus,
    },
    /// Tick acceptance criteria item
    Tick {
        /// Work item ID
        id: String,
        /// Field (acceptance_criteria)
        field: String,
        /// Pattern to match
        pattern: Option<String>,
        /// New status
        #[arg(short, long, value_enum, default_value = "done")]
        status: TickStatus,
        /// Match by index
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match
        #[arg(long)]
        exact: bool,
        /// Regex pattern
        #[arg(long)]
        regex: bool,
    },
    /// Delete work item
    Delete {
        /// Work item ID
        id: String,
        /// Force deletion without confirmation
        #[arg(short = 'f', long)]
        force: bool,
    },
    /// Render a single work item to markdown
    Render {
        /// Work item ID to render
        id: String,
        /// Dry run: show what would be written
        #[arg(long)]
        dry_run: bool,
    },
    /// Show work item content to stdout (no file written)
    Show {
        /// Work item ID
        id: String,
        /// Output format (markdown or json)
        #[arg(short = 'o', long, value_enum, default_value = "table")]
        output: OutputFormat,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = run(&cli);

    match result {
        Ok(diags) => {
            let has_errors = diags.iter().any(|d| d.level == DiagnosticLevel::Error);
            let has_warnings = diags.iter().any(|d| d.level == DiagnosticLevel::Warning);

            for diag in &diags {
                ui::diagnostic(diag);
            }

            if has_errors {
                ExitCode::FAILURE
            } else if has_warnings {
                if matches!(
                    cli.command,
                    Commands::Check {
                        deny_warnings: true,
                        ..
                    }
                ) {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            // Try to extract Diagnostic for structured error output
            if let Some(diag) = e.downcast_ref::<Diagnostic>() {
                ui::diagnostic(diag);
            } else {
                ui::error(&e);
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<Vec<Diagnostic>> {
    let config = Config::load(cli.config.as_deref())?;
    let op = write::WriteOp::from_dry_run(cli.dry_run);

    // Convert parsed CLI command to canonical form
    let canonical = command_router::CanonicalCommand::from_parsed(&cli.command)?;

    // Handle render command dry-run flag combination (special case)
    let canonical = match canonical {
        command_router::CanonicalCommand::Render {
            target,
            dry_run,
            force,
        } => command_router::CanonicalCommand::Render {
            target,
            dry_run: cli.dry_run || dry_run,
            force,
        },
        command_router::CanonicalCommand::RfcRender { id, dry_run } => {
            command_router::CanonicalCommand::RfcRender {
                id,
                dry_run: cli.dry_run || dry_run,
            }
        }
        command_router::CanonicalCommand::AdrRender { id, dry_run } => {
            command_router::CanonicalCommand::AdrRender {
                id,
                dry_run: cli.dry_run || dry_run,
            }
        }
        command_router::CanonicalCommand::WorkRender { id, dry_run } => {
            command_router::CanonicalCommand::WorkRender {
                id,
                dry_run: cli.dry_run || dry_run,
            }
        }
        other => other,
    };

    // Acquire gov-root exclusive lock for write commands (RFC-0004)
    let _guard = if canonical.is_write_command() {
        // Init creates gov_root; ensure it exists so we can create the lock file there
        if matches!(canonical, command_router::CanonicalCommand::Init { .. }) {
            let gov_root = config.paths.gov_root.as_path();
            if !gov_root.exists() {
                std::fs::create_dir_all(gov_root)
                    .map_err(|e| anyhow::anyhow!("Failed to create gov root: {}", e))?;
            }
        }
        Some(lock::acquire_gov_lock(&config)?)
    } else {
        None
    };

    // Execute via canonical command pattern (single execution path)
    canonical.execute(&config, op)
}
