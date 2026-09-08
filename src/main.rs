//! govctl: Project governance CLI for RFC, ADR, and Work Item management.

use clap::Parser;
use std::process::ExitCode;

mod artifact_catalog;
mod artifact_index;
mod cli;
mod cmd;
mod command_router;
mod config;
mod diagnostic;
mod load;
mod local_index;
mod lock;
mod loop_planner;
mod loop_state;
mod model;
mod parse;
mod reference_pattern;
mod registry;
mod render;
mod resource_plan;
mod scan;
mod schema;
mod signature;
mod status_counts;
mod terminal_md;
mod theme;
mod ui;
mod validate;
mod verification;
mod workspace;
mod write;

#[cfg(feature = "tui")]
mod tui;

// Re-export CLI types so modules can use `crate::TickStatus`, etc.
pub(crate) use cli::*;

use config::Config;
use diagnostic::{Diagnostic, DiagnosticLevel, DiagnosticResult, Diagnostics};

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if let Some(diag) = cli::misrouted_nested_clause(&args) {
        ui::diagnostic(&diag);
        return ExitCode::FAILURE;
    }

    let cli = Cli::parse_from(&args);
    if let Some(diag) = cli::misrouted_clause_after_parse(&cli, &args) {
        ui::diagnostic(&diag);
        return ExitCode::FAILURE;
    }
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
        Err(diag) => {
            ui::diagnostic(&diag);
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> DiagnosticResult<Diagnostics> {
    // Convert parsed CLI command to canonical form
    let plan = command_router::CommandPlan::from_parsed(&cli.command, cli.dry_run)?;
    let project_independent = matches!(
        &plan.op,
        command_router::Op::Builtin(command_router::BuiltinOp::Completions { .. })
            | command_router::Op::Builtin(command_router::BuiltinOp::SelfUpdate { .. })
            | command_router::Op::Builtin(command_router::BuiltinOp::Agent { .. })
            | command_router::Op::Builtin(command_router::BuiltinOp::AgentHook { .. })
            | command_router::Op::Builtin(command_router::BuiltinOp::Describe { context: false })
    );
    let is_migrate = matches!(
        &plan.op,
        command_router::Op::Builtin(command_router::BuiltinOp::Migrate)
    );
    let config = if project_independent {
        Config::default()
    } else if cli.config.is_none() {
        match &plan.op {
            command_router::Op::Builtin(command_router::BuiltinOp::Init { force }) => {
                Config::for_init(*force)?
            }
            _ if is_migrate => Config::load_for_migration(None)?,
            _ => Config::load(None)?,
        }
    } else if is_migrate {
        Config::load_for_migration(cli.config.as_deref())?
    } else {
        Config::load(cli.config.as_deref())?
    };
    if !is_migrate {
        load::reject_unmigrated_conformance(&config)?;
    }

    // [[RFC-0010:C-COMMAND-SCOPE]]: trunk-scoped commands run only in the
    // primary workspace of a clone.
    let trunk_warning = enforce_trunk_scope(&plan, &config)?;

    let op = write::WriteOp::from_dry_run(cli.dry_run);

    let lock_disposition = plan.lock_disposition();
    // Acquire gov-root exclusive lock for mutating operations (RFC-0004)
    let _guard = if matches!(
        lock_disposition,
        command_router::LockDisposition::GovRootExclusive
    ) {
        if matches!(
            plan.op,
            command_router::Op::Builtin(command_router::BuiltinOp::Init { .. })
        ) {
            let gov_root = config.gov_root.as_path();
            if !op.is_preview() && !gov_root.exists() {
                std::fs::create_dir_all(gov_root).map_err(|e| {
                    Diagnostic::io_error("create gov root", e, gov_root.display().to_string())
                })?;
            }
        }
        if op.is_preview() {
            None
        } else {
            Some(lock::acquire_gov_lock(&config)?)
        }
    } else {
        None
    };

    // Execute via canonical command pattern (single execution path)
    match plan.execute(&config, op) {
        Ok(mut diags) => {
            if let Some(warning) = trunk_warning {
                diags.insert(0, warning);
            }
            Ok(diags)
        }
        Err(diag) => {
            // [[RFC-0010:C-COMMAND-SCOPE]]: the enforcement-inactive warning
            // must be emitted even when the trunk command itself fails
            // downstream.
            if let Some(warning) = trunk_warning {
                ui::diagnostic(&warning);
            }
            Err(diag)
        }
    }
}

/// Enforce trunk-scope placement per [[RFC-0010:C-COMMAND-SCOPE]].
///
/// Returns a warning when version control is present but the primary
/// workspace cannot be determined; fails when the current directory is a
/// secondary workspace. Directories without version control proceed silently.
fn enforce_trunk_scope(
    plan: &command_router::CommandPlan,
    config: &Config,
) -> DiagnosticResult<Option<Diagnostic>> {
    if plan.command_scope() != command_router::CommandScope::Trunk {
        return Ok(None);
    }
    let cwd = std::env::current_dir()
        .map_err(|err| Diagnostic::io_error("resolve current directory", err, "."))?;
    match workspace::detect(&cwd, config) {
        workspace::WorkspaceContext::Secondary { primary } => Err(Diagnostic::new(
            diagnostic::DiagnosticCode::E0823TrunkScopeViolation,
            format!(
                "Trunk-scoped commands (release, release undo, migrate) must run in the primary workspace: {}",
                primary.display()
            ),
            cwd.display().to_string(),
        )),
        workspace::WorkspaceContext::Undeterminable => Ok(Some(Diagnostic::new(
            diagnostic::DiagnosticCode::W0114TrunkScopeEnforcementInactive,
            "Version control is present but the primary workspace cannot be determined; \
             trunk-scope enforcement is inactive. Set workspace.primary in gov/config.toml \
             to name the primary workspace.",
            cwd.display().to_string(),
        ))),
        workspace::WorkspaceContext::Primary | workspace::WorkspaceContext::NoVcs => Ok(None),
    }
}
