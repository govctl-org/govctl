//! phaseos: Project governance CLI for RFC, ADR, and Work Item management.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::process::ExitCode;

mod config;
mod diagnostic;
mod load;
mod model;
mod parse;
mod render;
mod validate;
mod write;

mod cmd_check;
mod cmd_edit;
mod cmd_lifecycle;
mod cmd_list;
mod cmd_move;
mod cmd_new;
mod cmd_render;
mod cmd_status;

use config::Config;
use diagnostic::{Diagnostic, DiagnosticLevel};
use model::{ClauseKind, RfcPhase, WorkItemStatus};

#[derive(Parser)]
#[command(name = "phaseos")]
#[command(about = "Project governance CLI for RFC, ADR, and Work Item management")]
#[command(version)]
struct Cli {
    /// Path to phaseos config (TOML)
    #[arg(short = 'C', long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize phaseos in the current directory
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

    /// Render RFC markdown from JSON source
    #[command(visible_alias = "gen")]
    Render {
        /// RFC ID to render (e.g., RFC-0001). If omitted, renders all.
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
        text_stdin: bool,
    },

    /// Set a field value
    Set {
        /// Artifact ID (e.g., RFC-0001 or ADR-0001)
        id: String,
        /// Field name
        field: String,
        /// New value
        value: String,
    },

    /// Get a field value
    Get {
        /// Artifact ID
        id: String,
        /// Field name (omit to show all)
        field: Option<String>,
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
}

#[derive(Subcommand, Clone, Debug)]
enum NewTarget {
    /// Create a new RFC
    Rfc {
        /// RFC ID (e.g., RFC-0010)
        rfc_id: String,
        /// RFC title
        title: String,
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
                eprintln!("{diag}");
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
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> anyhow::Result<Vec<Diagnostic>> {
    let config = Config::load(cli.config.as_deref())?;

    match &cli.command {
        Commands::Init { force } => cmd_new::init_project(&config, *force),
        Commands::Check { deny_warnings: _ } => cmd_check::check_all(&config),
        Commands::Status => cmd_status::show_status(&config),
        Commands::List { target, filter } => cmd_list::list(&config, *target, filter.as_deref()),
        Commands::New { target } => cmd_new::create(&config, target),
        Commands::Render { rfc_id, dry_run } => {
            cmd_render::render(&config, rfc_id.as_deref(), *dry_run)
        }
        Commands::Edit {
            clause_id,
            text,
            text_file,
            text_stdin,
        } => cmd_edit::edit_clause(
            &config,
            clause_id,
            text.as_deref(),
            text_file.as_deref(),
            *text_stdin,
        ),
        Commands::Set { id, field, value } => cmd_edit::set_field(&config, id, field, value),
        Commands::Get { id, field } => cmd_edit::get_field(&config, id, field.as_deref()),
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
            cmd_lifecycle::bump(&config, rfc_id, level, summary.as_deref(), changes)
        }
        Commands::Finalize { rfc_id, status } => cmd_lifecycle::finalize(&config, rfc_id, *status),
        Commands::Advance { rfc_id, phase } => cmd_lifecycle::advance(&config, rfc_id, *phase),
        Commands::Accept { adr } => cmd_lifecycle::accept_adr(&config, adr),
        Commands::Deprecate { id } => cmd_lifecycle::deprecate(&config, id),
        Commands::Supersede { id, by } => cmd_lifecycle::supersede(&config, id, by),
        Commands::Move { file, status } => cmd_move::move_item(&config, file, *status),
    }
}
