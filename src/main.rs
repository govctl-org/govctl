//! govctl: Project governance CLI for RFC, ADR, and Work Item management.

use clap::Parser;
use std::process::ExitCode;

mod cli;
mod cmd;
mod command_router;
mod config;
mod diagnostic;
mod load;
mod lock;
mod model;
mod parse;
mod render;
mod scan;
mod schema;
mod signature;
mod terminal_md;
mod theme;
mod ui;
mod validate;
mod verification;
mod write;

#[cfg(feature = "tui")]
mod tui;

// Re-export CLI types so modules can use `crate::TickStatus`, etc.
pub(crate) use cli::*;

use config::Config;
use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticLevel};

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
                    } | Commands::Check {
                        has_active: true,
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
        if matches!(canonical, command_router::CanonicalCommand::Init { .. }) {
            let gov_root = config.gov_root.as_path();
            if !gov_root.exists() {
                std::fs::create_dir_all(gov_root).map_err(|e| {
                    Diagnostic::new(
                        DiagnosticCode::E0901IoError,
                        format!("Failed to create gov root: {}", e),
                        gov_root.display().to_string(),
                    )
                })?;
            }
        }
        Some(lock::acquire_gov_lock(&config)?)
    } else {
        None
    };

    // Execute via canonical command pattern (single execution path)
    canonical.execute(&config, op)
}
