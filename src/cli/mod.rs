//! CLI argument definitions for govctl.

mod common;
mod resources;

pub(crate) use common::*;
pub(crate) use resources::*;

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Output format for agent definitions in `init-skills`.
#[derive(Clone, Debug, Default, ValueEnum)]
pub enum SkillFormat {
    /// Claude Code / Cursor / Windsurf (agents as .md with YAML frontmatter)
    #[default]
    Claude,
    /// Codex CLI (agents as .toml with developer_instructions)
    Codex,
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

    /// Loop execution-state commands
    #[command(after_help = "\
COMMON WORKFLOW:
    1. `govctl loop start WI-2026-04-06-001` to create local loop state
    2. `govctl loop run WI-2026-04-06-001` to execute one round for ready work
    3. `govctl loop show <LOOP-ID>` to inspect persisted state
    4. `govctl loop resume WI-2026-04-06-001` to find matching non-terminal state

NOTES:
    - Loop state is local under `.govctl/loops/<LOOP-ID>/state.toml`.
    - `loop run` uses work-item lifecycle commands for status transitions.
")]
    Loop {
        #[command(subcommand)]
        command: LoopCommand,
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

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum LoopCommand {
    /// Start a loop for one or more explicit work items
    #[command(after_help = "\
EXAMPLES:
    govctl loop start WI-2026-04-06-001
    govctl loop start --id loop-demo WI-2026-04-06-001 WI-2026-04-06-002

NOTES:
    - Resolves transitive `depends_on` dependencies before writing state.
    - Reuses an existing non-terminal loop with the same root set when unambiguous.
")]
    Start {
        /// Optional loop ID; generated when omitted
        #[arg(long)]
        id: Option<String>,
        /// Explicit root work item IDs
        #[arg(required = true, value_name = "WI-ID")]
        work_items: Vec<String>,
    },
    /// Show persisted loop state
    #[command(after_help = "\
EXAMPLES:
    govctl loop show loop-demo
")]
    Show {
        /// Loop ID
        id: String,
    },
    /// Resume or inspect an existing non-terminal loop
    #[command(after_help = "\
EXAMPLES:
    govctl loop resume --id loop-demo
    govctl loop resume WI-2026-04-06-001

NOTES:
    - With `--id`, resumes that explicit loop.
    - Without `--id`, searches for exactly one non-terminal loop with the same root set.
")]
    Resume {
        /// Explicit loop ID
        #[arg(long)]
        id: Option<String>,
        /// Root work item IDs for discovery when --id is omitted
        #[arg(value_name = "WI-ID")]
        work_items: Vec<String>,
    },
    /// Run one execution round for each currently executable work item
    #[command(after_help = "\
EXAMPLES:
    govctl loop run WI-2026-04-06-001
    govctl loop run --id loop-demo
    govctl loop run --id loop-demo --max-rounds 2 WI-2026-04-06-001

NOTES:
    - Starts a new loop when no matching non-terminal loop exists.
    - Resumes existing loop state by explicit ID or unambiguous root set.
    - Uses `govctl work move` semantics for work item status transitions.
")]
    Run {
        /// Explicit loop ID; generated when omitted and a new loop is started
        #[arg(long)]
        id: Option<String>,
        /// Maximum rounds each work item may run before loop-level failure
        #[arg(long, default_value_t = 1)]
        max_rounds: u32,
        /// Root work item IDs for start/discovery when needed
        #[arg(value_name = "WI-ID")]
        work_items: Vec<String>,
    },
}
