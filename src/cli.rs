//! CLI argument definitions for govctl.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::model::{ChangelogCategory, ClauseKind, RfcPhase, WorkItemStatus};

/// Output format for agent definitions in `init-skills`.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum SkillFormat {
    /// Claude Code / Cursor / Windsurf (agents as .md with YAML frontmatter)
    #[default]
    Claude,
    /// Codex CLI (agents as .toml with developer_instructions)
    Codex,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct EditActionArgs {
    /// Set a scalar value (omit VALUE only when using --stdin)
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) set: Option<Option<String>>,
    /// Append a value to a list (omit VALUE only when using --stdin)
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) add: Option<Option<String>>,
    /// Remove a matching value, or omit PATTERN when removing an indexed path
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) remove: Option<Option<String>>,
    /// Update checklist-style item status
    #[arg(long, group = "edit_action")]
    pub(crate) tick: Option<TickStatus>,
    /// Read set/add value from stdin
    #[arg(long)]
    pub(crate) stdin: bool,
    /// Match by index for remove/tick
    #[arg(long, allow_hyphen_values = true)]
    pub(crate) at: Option<i32>,
    /// Exact match for remove/tick
    #[arg(long)]
    pub(crate) exact: bool,
    /// Regex match for remove/tick
    #[arg(long)]
    pub(crate) regex: bool,
    /// Remove all matches
    #[arg(long)]
    pub(crate) all: bool,
}

#[derive(Parser)]
#[command(name = "govctl")]
#[command(about = "Project governance CLI for RFC, ADR, and Work Item management")]
#[command(version)]
pub(crate) struct Cli {
    /// Path to govctl config (TOML)
    #[arg(short = 'C', long, global = true)]
    pub(crate) config: Option<PathBuf>,

    /// Dry run: preview changes without writing files
    #[arg(long, global = true)]
    pub(crate) dry_run: bool,

    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialize govctl in the current directory
    #[command(after_help = "\
EXAMPLES:
    govctl init
    govctl init --force

NOTES:
    - Creates `gov/`, `gov/config.toml`, and baseline governance artifacts.
    - Use `--force` to overwrite an existing initialization.
")]
    Init {
        /// Overwrite existing config
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Install skills and agents into the project's agent directory
    #[command(name = "init-skills")]
    #[command(after_help = "\
EXAMPLES:
    govctl init-skills
    govctl init-skills --force

NOTES:
    - Installs or refreshes project-local skills and agents.
    - Use `--force` to overwrite existing generated assets.
")]
    InitSkills {
        /// Force overwrite existing assets
        #[arg(short = 'f', long)]
        force: bool,
        /// Output format for agent definitions: claude (default) or codex
        #[arg(long, default_value = "claude")]
        format: SkillFormat,
        /// Override output directory (default: agent_dir from config, or format-implied)
        #[arg(long)]
        dir: Option<PathBuf>,
    },

    /// Validate all governed documents
    #[command(visible_alias = "lint")]
    #[command(after_help = "\
EXAMPLES:
    govctl check
    govctl check -W
    govctl check --has-active

NOTES:
    - `-W/--deny-warnings` treats warnings as errors.
    - `--has-active` asserts that an active work item exists.
")]
    Check {
        /// Treat warnings as errors
        #[arg(short = 'W', long)]
        deny_warnings: bool,

        /// Assert that an active work item exists (exits non-zero if none)
        #[arg(long)]
        has_active: bool,
    },

    /// Show summary counts
    #[command(visible_alias = "stat")]
    #[command(after_help = "\
EXAMPLES:
    govctl status

NOTES:
    - Prints high-level counts for governed artifacts.
")]
    Status,

    /// Render artifacts to markdown from SSOT (bulk operation)
    ///
    /// For single-item render, use: govctl rfc render <ID>, govctl adr render <ID>, etc.
    #[command(
        visible_alias = "gen",
        after_help = "\
EXAMPLES:
    govctl render
    govctl render adr
    govctl render work --dry-run
    govctl render changelog --force

NOTES:
    - This is a bulk render entrypoint.
    - For a single artifact, use resource render:
      `govctl rfc render <ID>`, `govctl adr render <ID>`, `govctl work render <ID>`.
"
    )]
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

    /// Migrate legacy governance storage to current canonical formats
    #[command(after_help = "\
EXAMPLES:
    govctl migrate
    govctl --dry-run migrate

NOTES:
    - Reads legacy JSON artifacts and upgrades them to current canonical storage.
    - Intended for one-time repository migration, not normal day-to-day editing.
")]
    Migrate,

    /// Execute reusable verification guards
    #[command(after_help = "\
EXAMPLES:
    govctl verify GUARD-CLIPPY
    govctl verify GUARD-CLIPPY GUARD-TESTS
    govctl verify --work WI-2026-04-06-001

NOTES:
    - Pass guard IDs to run specific guards directly.
    - Use `--work` to run the effective guard set for a work item.
    - `--work` conflicts with explicit guard IDs.
")]
    Verify {
        /// Verification guard IDs to run
        #[arg(value_name = "GUARD-ID")]
        guard_ids: Vec<String>,
        /// Run the effective guard set for a specific work item
        #[arg(long, conflicts_with = "guard_ids")]
        work: Option<String>,
    },

    // ========================================
    // Resource-First Commands (RFC-0002)
    // ========================================
    /// RFC operations
    #[command(after_help = "\
COMMON WORKFLOW:
    1. `govctl rfc list` to discover RFCs
    2. `govctl rfc get <ID> ...` for metadata/fields
    3. `govctl rfc show <ID>` for rendered prose
    4. `govctl rfc edit <ID> ...` to update content
    5. `govctl rfc finalize/advance/...` for lifecycle

START HERE:
    - New RFC: `govctl rfc new \"Title\"`
    - Inspect one RFC: `govctl rfc get RFC-0001`
    - Render one RFC: `govctl rfc show RFC-0001`
")]
    Rfc {
        #[command(subcommand)]
        command: RfcCommand,
    },

    /// Clause operations
    #[command(after_help = "\
COMMON WORKFLOW:
    1. `govctl clause list` to discover clauses
    2. `govctl clause get <ID> ...` for metadata/fields
    3. `govctl clause show <ID>` for rendered clause text
    4. `govctl clause edit <ID> ...` to update content
    5. `govctl clause deprecate/supersede` for lifecycle

START HERE:
    - New clause: `govctl clause new RFC-0001:C-SCOPE \"Scope\"`
    - Inspect one clause: `govctl clause get RFC-0001:C-SCOPE`
")]
    Clause {
        #[command(subcommand)]
        command: ClauseCommand,
    },

    /// ADR operations
    #[command(after_help = "\
COMMON WORKFLOW:
    1. `govctl adr list` to discover ADRs
    2. `govctl adr get <ID> ...` for metadata/fields
    3. `govctl adr show <ID>` for rendered prose
    4. `govctl adr edit/add/tick` to work through alternatives
    5. `govctl adr accept/reject/...` for lifecycle

START HERE:
    - New ADR: `govctl adr new \"Title\"`
    - Inspect one ADR: `govctl adr get ADR-0001`
    - Move an alternative to accepted: `govctl adr tick ADR-0001 alternatives --at 0 -s accepted`
")]
    Adr {
        #[command(subcommand)]
        command: AdrCommand,
    },

    /// Work item operations
    #[command(visible_alias = "wi")]
    #[command(after_help = "\
COMMON WORKFLOW:
    1. `govctl work list` to discover work items
    2. `govctl work get <ID> ...` for metadata/fields
    3. `govctl work edit/add` to define scope and acceptance criteria
    4. `govctl work tick` to update acceptance-criteria status
    5. `govctl work move` to change lifecycle state

START HERE:
    - New work item: `govctl work new \"Title\"`
    - Activate work: `govctl work move WI-<DATE>-001 active`
    - Inspect one work item: `govctl work get WI-<DATE>-001`
")]
    Work {
        #[command(subcommand)]
        command: WorkCommand,
    },

    /// Verification guard operations
    #[command(after_help = "\
COMMON WORKFLOW:
    1. `govctl guard list` to discover guards
    2. `govctl guard get <ID> ...` for metadata/fields
    3. `govctl guard edit/set` to define checks
    4. `govctl verify <GUARD-ID>` or `govctl verify --work <WI-ID>` to run guards

START HERE:
    - New guard: `govctl guard new \"clippy lint\"`
    - Inspect one guard: `govctl guard get GUARD-CLIPPY`
")]
    Guard {
        #[command(subcommand)]
        command: GuardCommand,
    },

    /// Cut a release (collect unreleased work items into a version)
    #[command(after_help = "\
EXAMPLES:
    govctl release 0.2.0
    govctl release 0.2.0 --date 2026-04-07

NOTES:
    - Collects unreleased completed work items into a versioned release.
    - Use a semver version string.
")]
    Release {
        /// Version number (semver, e.g., 0.2.0)
        version: String,
        /// Release date (defaults to today)
        #[arg(long)]
        date: Option<String>,
    },

    /// Output machine-readable CLI metadata for agents
    #[command(after_help = "\
EXAMPLES:
    govctl describe
    govctl describe --context
    govctl describe -o json

NOTES:
    - `--context` includes current project state and suggested next actions.
    - Output is intended for agents and tooling.
")]
    Describe {
        /// Include project state and suggested actions
        #[arg(long)]
        context: bool,
        /// Output format (currently only json is supported)
        #[arg(short = 'o', long, default_value = "json")]
        output: String,
    },

    /// Generate shell completion scripts
    #[command(after_help = "\
EXAMPLES:
    govctl completions bash
    govctl completions zsh

NOTES:
    - Writes completion script text to stdout for the selected shell.
")]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Update govctl binary to the latest release
    #[command(name = "self-update")]
    #[command(after_help = "\
EXAMPLES:
    govctl self-update
    govctl self-update --check

NOTES:
    - Downloads the latest binary from GitHub Releases and replaces the current executable.
    - Use `--check` to see if an update is available without installing it.
    - Implements [[RFC-0002:C-SELF-UPDATE]].
")]
    SelfUpdate {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,
    },

    /// Launch interactive TUI dashboard
    #[cfg(feature = "tui")]
    Tui,

    /// Manage controlled-vocabulary tags
    #[command(after_help = "\
EXAMPLES:
    govctl tag list
    govctl tag new caching
    govctl tag delete caching

NOTES:
    - Tags are defined project-wide in gov/config.toml [tags] allowed.
    - Artifacts may only reference tags declared here.
    - Implements [[RFC-0002:C-RESOURCES]] controlled-vocabulary tags.
")]
    Tag {
        #[command(subcommand)]
        command: TagCommand,
    },
}

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

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum TickStatus {
    /// Mark work items as done
    Done,
    /// Mark work items as pending
    Pending,
    /// Mark work items as cancelled
    Cancelled,
    /// Mark ADR alternatives as accepted
    Accepted,
    /// Mark ADR alternatives as considered
    Considered,
    /// Mark ADR alternatives as rejected
    Rejected,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum WorkTickStatus {
    /// Mark work items as done
    Done,
    /// Mark work items as pending
    Pending,
    /// Mark work items as cancelled
    Cancelled,
}

impl From<WorkTickStatus> for TickStatus {
    fn from(value: WorkTickStatus) -> Self {
        match value {
            WorkTickStatus::Done => TickStatus::Done,
            WorkTickStatus::Pending => TickStatus::Pending,
            WorkTickStatus::Cancelled => TickStatus::Cancelled,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum AdrTickStatus {
    /// Mark ADR alternatives as accepted
    Accepted,
    /// Mark ADR alternatives as considered
    Considered,
    /// Mark ADR alternatives as rejected
    Rejected,
}

impl From<AdrTickStatus> for TickStatus {
    fn from(value: AdrTickStatus) -> Self {
        match value {
            AdrTickStatus::Accepted => TickStatus::Accepted,
            AdrTickStatus::Considered => TickStatus::Considered,
            AdrTickStatus::Rejected => TickStatus::Rejected,
        }
    }
}

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum NewTarget {
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

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ListTarget {
    Rfc,
    Clause,
    Adr,
    Work,
    Guard,
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
pub(crate) enum RenderTarget {
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
pub(crate) enum FinalizeStatus {
    Normative,
    Deprecated,
}

// ========================================
// Resource-First Commands (RFC-0002)
// ========================================

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
    pub(crate) output: OutputFormat,
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
    /// Scope/topic for journal creation
    #[arg(long)]
    pub(crate) scope: Option<String>,
}

#[derive(Args, Clone, Debug)]
pub(crate) struct WorkAddArgs {
    #[command(flatten)]
    pub(crate) common: CommonAddArgs,
    /// Changelog category for acceptance_criteria (alternative to prefix)
    #[arg(short = 'c', long, value_enum)]
    pub(crate) category: Option<ChangelogCategory>,
    /// Scope/topic for journal entry (e.g., "backend", "frontend", "docs")
    #[arg(long)]
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
    /// Deprecate ADR
    #[command(after_help = "\
EXAMPLES:
    govctl adr deprecate ADR-0001
    govctl adr deprecate ADR-0001 --force
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
    - title, description, status, completed_at, refs
    - journal, notes, acceptance_criteria

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
    - journal: Execution tracking entries (date + content)
    - notes: Ad-hoc key points (short strings)
    - acceptance_criteria: Completion criteria with category

FIELD SEMANTICS (per ADR-0026):
  - description: Task scope - define once, rarely change
  - journal: Execution tracking - append on each progress (has date, scope, content)
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
    - journal: Execution tracking entries - append progress with date
    - notes: Ad-hoc key points (short strings)
    - acceptance_criteria: Completion criteria with category prefix

FIELD SEMANTICS:
  - description: Task scope - define once, rarely change
  - journal: Execution tracking - append on each progress
    * Each entry has: date (auto-filled), scope (optional), content
    * Use --stdin for multi-line entries
    * Use --scope to set the scope/topic
  - notes: Ad-hoc points - add anytime, keep concise

ACCEPTANCE CRITERIA FORMAT:
    Use category prefix for changelog generation:
    - \"add: New feature\"       → Added section
    - \"fix: Bug fixed\"         → Fixed section
    - \"changed: Behavior\"      → Changed section
    - \"chore: Tests pass\"      → Excluded from changelog

EXAMPLES:
    govctl work add WI-001 refs RFC-0001
    govctl work add WI-001 acceptance_criteria \"add: Implement feature\"
    govctl work add WI-001 journal --scope typub-html --stdin <<'EOF'
    Progress update for today...
    EOF
    govctl work add WI-001 notes \"Remember to test edge cases\"
")]
    Add(WorkAddArgs),
    /// Remove value from work item array field
    #[command(after_help = "\
VALID ARRAY FIELDS:
    - refs, journal, notes, acceptance_criteria

MATCHING OPTIONS:
    - pattern: Substring match (default)
    - --at N: Remove by index (0-based, negative = from end)
    - --exact: Exact string match
    - --regex: Regex pattern match
    - --all: Remove all matches

EXAMPLES:
    govctl work remove WI-001 refs RFC-0001     # Remove first match
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
