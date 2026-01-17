//! govctl: Project governance CLI for RFC, ADR, and Work Item management.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;

mod config;
mod diagnostic;
mod load;
mod model;
mod parse;
mod render;
mod scan;
mod signature;
mod ui;
mod validate;
mod write;

mod cmd;

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
    #[arg(short = 'n', long, global = true)]
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

    /// List artifacts
    #[command(visible_alias = "ls")]
    List {
        /// Target to list
        #[arg(value_enum)]
        target: ListTarget,
        /// Filter (e.g., status for ADRs, RFC ID for clauses)
        filter: Option<String>,
    },

    /// Create a new artifact
    New {
        #[command(subcommand)]
        target: NewTarget,
    },

    /// Render artifacts to markdown from SSOT
    #[command(visible_alias = "gen")]
    Render {
        /// What to render: rfc (default), adr, work, or all
        #[arg(value_enum, default_value = "rfc")]
        target: RenderTarget,
        /// Specific RFC ID to render (e.g., RFC-0001)
        #[arg(long)]
        rfc_id: Option<String>,
        /// Dry run: show what would be written
        #[arg(long)]
        dry_run: bool,
    },

    /// Edit clause text
    Edit {
        /// Clause ID (e.g., RFC-0001:C-PHASE-ORDER)
        clause_id: String,
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

    /// Set a field value
    Set {
        /// Artifact ID (e.g., RFC-0001 or ADR-0001)
        id: String,
        /// Field name
        field: String,
        /// New value (omit if using --stdin)
        #[arg(required_unless_present = "stdin")]
        value: Option<String>,
        /// Read value from stdin (for multi-line content)
        #[arg(long)]
        stdin: bool,
    },

    /// Get a field value
    Get {
        /// Artifact ID
        id: String,
        /// Field name (omit to show all)
        field: Option<String>,
    },

    /// Add a value to an array field
    Add {
        /// Artifact ID (e.g., RFC-0001 or ADR-0001)
        id: String,
        /// Array field name (e.g., owners, refs, anchors)
        field: String,
        /// Value to add (optional if --stdin)
        value: Option<String>,
        /// Read value from stdin (supports multi-line)
        #[arg(long)]
        stdin: bool,
    },

    /// Remove a value from an array field
    Remove {
        /// Artifact ID
        id: String,
        /// Array field name
        field: String,
        /// Pattern to match (substring by default, case-insensitive)
        #[arg(required_unless_present = "at")]
        pattern: Option<String>,
        /// Remove by index (0-based, negative from end)
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match (case-sensitive)
        #[arg(long)]
        exact: bool,
        /// Regex pattern match
        #[arg(long)]
        regex: bool,
        /// Remove all matches (required for multiple matches)
        #[arg(long)]
        all: bool,
    },

    /// Bump RFC version
    Bump {
        /// RFC ID
        rfc_id: String,
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

    /// Transition RFC status to normative/deprecated
    Finalize {
        /// RFC ID
        rfc_id: String,
        /// Target status
        #[arg(value_enum)]
        status: FinalizeStatus,
    },

    /// Advance RFC phase
    Advance {
        /// RFC ID
        rfc_id: String,
        /// Target phase
        #[arg(value_enum)]
        phase: RfcPhase,
    },

    /// Accept an ADR (proposed -> accepted)
    Accept {
        /// ADR ID or filename
        adr: String,
    },

    /// Deprecate an artifact
    Deprecate {
        /// Artifact ID (RFC, clause, or ADR)
        id: String,
    },

    /// Supersede an artifact
    Supersede {
        /// Artifact ID to supersede
        id: String,
        /// Replacement artifact ID
        #[arg(long)]
        by: String,
    },

    /// Move work item to new status
    #[command(visible_alias = "mv")]
    Move {
        /// Work item file
        file: PathBuf,
        /// Target status
        #[arg(value_enum)]
        status: WorkItemStatus,
    },

    /// Mark a checklist item as done/pending/cancelled
    Tick {
        /// Artifact ID (WI-xxx or ADR-xxx)
        id: String,
        /// Field (acceptance_criteria, decisions, or alternatives)
        field: String,
        /// Pattern to match (substring by default, case-insensitive)
        pattern: Option<String>,
        /// New status (done, pending, cancelled for WI; accepted, rejected, considered for ADR)
        #[arg(short, long, value_enum, default_value = "done")]
        status: TickStatus,
        /// Match by index (0-based, negative from end)
        #[arg(long, allow_hyphen_values = true)]
        at: Option<i32>,
        /// Exact match (case-sensitive)
        #[arg(long)]
        exact: bool,
        /// Regex pattern match
        #[arg(long)]
        regex: bool,
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

#[derive(ValueEnum, Clone, Copy, Debug)]
enum RenderTarget {
    /// Render RFCs only (default, published to repo)
    Rfc,
    /// Render ADRs (local only, .gitignore'd)
    Adr,
    /// Render Work Items (local only, .gitignore'd)
    Work,
    /// Render all artifact types (local use)
    All,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum FinalizeStatus {
    Normative,
    Deprecated,
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
            ui::error(&e);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<Vec<Diagnostic>> {
    let config = Config::load(cli.config.as_deref())?;
    let op = write::WriteOp::from_dry_run(cli.dry_run);

    match &cli.command {
        Commands::Init { force } => cmd::new::init_project(&config, *force, op),
        Commands::Check { deny_warnings: _ } => cmd::check::check_all(&config),
        Commands::Status => cmd::status::show_status(&config),
        Commands::List { target, filter } => cmd::list::list(&config, *target, filter.as_deref()),
        Commands::New { target } => cmd::new::create(&config, target, op),
        Commands::Render {
            target,
            rfc_id,
            dry_run,
        } => {
            // Combine global and command-specific dry-run flags
            let effective_dry_run = cli.dry_run || *dry_run;
            let mut all_diags = vec![];
            match target {
                RenderTarget::Rfc => {
                    all_diags.extend(cmd::render::render(
                        &config,
                        rfc_id.as_deref(),
                        effective_dry_run,
                    )?);
                }
                RenderTarget::Adr => {
                    all_diags.extend(cmd::render::render_adrs(&config, effective_dry_run)?);
                }
                RenderTarget::Work => {
                    all_diags.extend(cmd::render::render_work_items(&config, effective_dry_run)?);
                }
                RenderTarget::All => {
                    all_diags.extend(cmd::render::render(
                        &config,
                        rfc_id.as_deref(),
                        effective_dry_run,
                    )?);
                    all_diags.extend(cmd::render::render_adrs(&config, effective_dry_run)?);
                    all_diags.extend(cmd::render::render_work_items(&config, effective_dry_run)?);
                }
            }
            Ok(all_diags)
        }
        Commands::Edit {
            clause_id,
            text,
            text_file,
            stdin,
        } => cmd::edit::edit_clause(
            &config,
            clause_id,
            text.as_deref(),
            text_file.as_deref(),
            *stdin,
            op,
        ),
        Commands::Set {
            id,
            field,
            value,
            stdin,
        } => cmd::edit::set_field(&config, id, field, value.as_deref(), *stdin, op),
        Commands::Get { id, field } => cmd::edit::get_field(&config, id, field.as_deref()),
        Commands::Add {
            id,
            field,
            value,
            stdin,
        } => cmd::edit::add_to_field(&config, id, field, value.as_deref(), *stdin, op),
        Commands::Remove {
            id,
            field,
            pattern,
            at,
            exact,
            regex,
            all,
        } => {
            let match_opts = cmd::edit::MatchOptions {
                pattern: pattern.as_deref(),
                at: *at,
                exact: *exact,
                regex: *regex,
                all: *all,
            };
            cmd::edit::remove_from_field(&config, id, field, &match_opts, op)
        }
        Commands::Bump {
            rfc_id,
            patch,
            minor,
            major,
            summary,
            changes,
        } => {
            let level = match (patch, minor, major) {
                (true, false, false) => Some(write::BumpLevel::Patch),
                (false, true, false) => Some(write::BumpLevel::Minor),
                (false, false, true) => Some(write::BumpLevel::Major),
                (false, false, false) => None,
                _ => unreachable!("clap arg group ensures mutual exclusivity"),
            };
            cmd::lifecycle::bump(&config, rfc_id, level, summary.as_deref(), changes, op)
        }
        Commands::Finalize { rfc_id, status } => {
            cmd::lifecycle::finalize(&config, rfc_id, *status, op)
        }
        Commands::Advance { rfc_id, phase } => cmd::lifecycle::advance(&config, rfc_id, *phase, op),
        Commands::Accept { adr } => cmd::lifecycle::accept_adr(&config, adr, op),
        Commands::Deprecate { id } => cmd::lifecycle::deprecate(&config, id, op),
        Commands::Supersede { id, by } => cmd::lifecycle::supersede(&config, id, by, op),
        Commands::Move { file, status } => cmd::move_::move_item(&config, file, *status, op),
        Commands::Tick {
            id,
            field,
            pattern,
            status,
            at,
            exact,
            regex,
        } => {
            let match_opts = cmd::edit::MatchOptions {
                pattern: pattern.as_deref(),
                at: *at,
                exact: *exact,
                regex: *regex,
                all: false, // tick never allows --all
            };
            cmd::edit::tick_item(&config, id, field, &match_opts, *status, op)
        }
        #[cfg(feature = "tui")]
        Commands::Tui => {
            tui::run(&config)?;
            Ok(vec![])
        }
    }
}
