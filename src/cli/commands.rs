use super::help;
use super::{
    AdrCommand, ClauseCommand, GuardCommand, LoopCommand, RenderTarget, RfcCommand, SkillFormat,
    TagCommand, WorkCommand,
};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Initialize govctl in the current directory
    #[command(after_help = help::INIT)]
    Init {
        /// Overwrite existing config
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Install skills and agents into the project's agent directory
    #[command(name = "init-skills")]
    #[command(after_help = help::INIT_SKILLS)]
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
    #[command(after_help = help::CHECK)]
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
    #[command(after_help = help::STATUS)]
    Status,

    /// Render artifacts to markdown from SSOT (bulk operation)
    ///
    /// For single-item render, use: govctl rfc render <ID>, govctl adr render <ID>, etc.
    #[command(
        visible_alias = "gen",
        after_help = help::RENDER
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
    #[command(after_help = help::MIGRATE)]
    Migrate,

    /// Execute reusable verification guards
    #[command(after_help = help::VERIFY)]
    Verify {
        /// Verification guard IDs to run
        #[arg(value_name = "GUARD-ID")]
        guard_ids: Vec<String>,
        /// Run the effective guard set for a specific work item
        #[arg(long, conflicts_with = "guard_ids")]
        work: Option<String>,
    },

    /// Loop execution-state commands
    #[command(after_help = help::LOOP)]
    Loop {
        #[command(subcommand)]
        command: LoopCommand,
    },

    /// RFC operations
    #[command(after_help = help::RFC)]
    Rfc {
        #[command(subcommand)]
        command: RfcCommand,
    },

    /// Clause operations
    #[command(after_help = help::CLAUSE)]
    Clause {
        #[command(subcommand)]
        command: ClauseCommand,
    },

    /// ADR operations
    #[command(after_help = help::ADR)]
    Adr {
        #[command(subcommand)]
        command: AdrCommand,
    },

    /// Work item operations
    #[command(visible_alias = "wi")]
    #[command(after_help = help::WORK)]
    Work {
        #[command(subcommand)]
        command: WorkCommand,
    },

    /// Verification guard operations
    #[command(after_help = help::GUARD)]
    Guard {
        #[command(subcommand)]
        command: GuardCommand,
    },

    /// Cut a release (collect unreleased work items into a version)
    #[command(after_help = help::RELEASE)]
    Release {
        /// Version number (semver, e.g., 0.2.0)
        version: String,
        /// Release date (defaults to today)
        #[arg(long)]
        date: Option<String>,
    },

    /// Output machine-readable CLI metadata for agents
    #[command(after_help = help::DESCRIBE)]
    Describe {
        /// Include project state and suggested actions
        #[arg(long)]
        context: bool,
        /// Output format (currently only json is supported)
        #[arg(short = 'o', long, default_value = "json")]
        output: String,
    },

    /// Generate shell completion scripts
    #[command(after_help = help::COMPLETIONS)]
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Update govctl binary to the latest release
    #[command(name = "self-update")]
    #[command(after_help = help::SELF_UPDATE)]
    SelfUpdate {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,
    },

    /// Launch interactive TUI dashboard
    #[cfg(feature = "tui")]
    Tui,

    /// Manage controlled-vocabulary tags
    #[command(after_help = help::TAG)]
    Tag {
        #[command(subcommand)]
        command: TagCommand,
    },
}
