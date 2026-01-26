//! Canonical command pattern for unified command routing.
//!
//! This module defines a single internal representation (`CanonicalCommand`)
//! that both old (deprecated verb-first) and new (resource-first) CLI syntaxes
//! map to. This ensures zero code duplication in business logic.
//!
//! Architecture:
//! 1. Parse CLI arguments (old or new syntax) → Commands enum
//! 2. Convert to CanonicalCommand (single source of truth)
//! 3. Execute via business logic in cmd::* modules

use crate::cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::{ChangelogCategory, ClauseKind, RfcPhase, WorkItemStatus};
use crate::write::{BumpLevel, WriteOp};
use crate::{Commands, FinalizeStatus, ListTarget, NewTarget, RenderTarget, TickStatus};
use std::path::PathBuf;

/// Owned version of MatchOptions for storage in CanonicalCommand.
#[derive(Debug, Clone)]
pub struct OwnedMatchOptions {
    pub pattern: Option<String>,
    pub at: Option<i32>,
    pub exact: bool,
    pub regex: bool,
    pub all: bool,
}

impl OwnedMatchOptions {
    /// Convert to borrowed MatchOptions with appropriate lifetime.
    pub fn as_match_options(&self) -> cmd::edit::MatchOptions<'_> {
        cmd::edit::MatchOptions {
            pattern: self.pattern.as_deref(),
            at: self.at,
            exact: self.exact,
            regex: self.regex,
            all: self.all,
        }
    }
}

/// Canonical internal representation of all commands.
/// This is the single source of truth for command execution.
#[derive(Debug, Clone)]
pub enum CanonicalCommand {
    // ========================================
    // Global Commands (no resource prefix)
    // ========================================
    Init {
        force: bool,
    },
    SyncCommands {
        force: bool,
    },
    Check {
        #[allow(dead_code)]
        deny_warnings: bool,
    },
    Status,
    Render {
        target: RenderTarget,
        rfc_id: Option<String>,
        dry_run: bool,
        force: bool,
    },
    Describe {
        context: bool,
        #[allow(dead_code)]
        format: String,
    },
    Completions {
        shell: clap_complete::Shell,
    },
    #[cfg(feature = "tui")]
    Tui,

    // ========================================
    // RFC Commands
    // ========================================
    RfcNew {
        title: String,
        id: Option<String>,
    },
    RfcList {
        filter: Option<String>,
        limit: Option<usize>,
    },
    RfcGet {
        id: String,
        field: Option<String>,
    },
    RfcSet {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    RfcBump {
        id: String,
        level: Option<BumpLevel>,
        summary: Option<String>,
        changes: Vec<String>,
    },
    RfcFinalize {
        id: String,
        status: FinalizeStatus,
    },
    RfcAdvance {
        id: String,
        phase: RfcPhase,
    },
    RfcDeprecate {
        id: String,
    },
    RfcSupersede {
        id: String,
        by: String,
    },

    // ========================================
    // Clause Commands
    // ========================================
    ClauseNew {
        clause_id: String,
        title: String,
        section: String,
        kind: ClauseKind,
    },
    ClauseList {
        rfc_id: Option<String>,
        limit: Option<usize>,
    },
    ClauseGet {
        id: String,
        field: Option<String>,
    },
    ClauseEdit {
        id: String,
        text: Option<String>,
        text_file: Option<PathBuf>,
        stdin: bool,
    },
    ClauseSet {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    ClauseDelete {
        id: String,
        force: bool,
    },
    ClauseDeprecate {
        id: String,
    },
    ClauseSupersede {
        id: String,
        by: String,
    },

    // ========================================
    // ADR Commands
    // ========================================
    AdrNew {
        title: String,
    },
    AdrList {
        status: Option<String>,
        limit: Option<usize>,
    },
    AdrGet {
        id: String,
        field: Option<String>,
    },
    AdrSet {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    AdrAdd {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    AdrRemove {
        id: String,
        field: String,
        match_opts: OwnedMatchOptions,
    },
    AdrAccept {
        id: String,
    },
    AdrReject {
        id: String,
    },
    AdrDeprecate {
        id: String,
    },
    AdrSupersede {
        id: String,
        by: String,
    },
    AdrTick {
        id: String,
        field: String,
        match_opts: OwnedMatchOptions,
        status: TickStatus,
    },

    // ========================================
    // Work Item Commands
    // ========================================
    WorkNew {
        title: String,
        active: bool,
    },
    WorkList {
        status: Option<String>,
        limit: Option<usize>,
    },
    WorkGet {
        id: String,
        field: Option<String>,
    },
    WorkSet {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    WorkAdd {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
        category: Option<ChangelogCategory>,
    },
    WorkRemove {
        id: String,
        field: String,
        match_opts: OwnedMatchOptions,
    },
    WorkMove {
        file_or_id: PathBuf,
        status: WorkItemStatus,
    },
    WorkTick {
        id: String,
        field: String,
        match_opts: OwnedMatchOptions,
        status: TickStatus,
    },
    WorkDelete {
        id: String,
        force: bool,
    },

    // ========================================
    // Release Commands
    // ========================================
    ReleaseCut {
        version: String,
        date: Option<String>,
    },
}

impl CanonicalCommand {
    /// Convert parsed CLI commands to canonical form.
    ///
    /// This is where both old (deprecated) and new (resource-first) syntaxes
    /// are unified into a single representation.
    pub fn from_parsed(cmd: &Commands) -> anyhow::Result<Self> {
        Ok(match cmd {
            // Global commands
            Commands::Init { force } => Self::Init { force: *force },
            Commands::SyncCommands { force } => Self::SyncCommands { force: *force },
            Commands::Check { deny_warnings } => Self::Check {
                deny_warnings: *deny_warnings,
            },
            Commands::Status => Self::Status,
            Commands::Render {
                target,
                rfc_id,
                dry_run,
                force,
            } => Self::Render {
                target: *target,
                rfc_id: rfc_id.clone(),
                dry_run: *dry_run,
                force: *force,
            },
            Commands::Describe { context, format } => Self::Describe {
                context: *context,
                format: format.clone(),
            },
            Commands::Completions { shell } => Self::Completions { shell: *shell },
            #[cfg(feature = "tui")]
            Commands::Tui => Self::Tui,

            // ========================================
            // Resource-First Commands (RFC-0002)
            // ========================================
            Commands::Rfc { command } => Self::from_rfc_command(command)?,
            Commands::Clause { command } => Self::from_clause_command(command)?,
            Commands::Adr { command } => Self::from_adr_command(command)?,
            Commands::Work { command } => Self::from_work_command(command)?,

            Commands::Release { version, date } => Self::ReleaseCut {
                version: version.clone(),
                date: date.clone(),
            },
        })
    }

    /// Execute the canonical command using business logic from cmd::* modules.
    ///
    /// This is the single execution path - no duplication.
    pub fn execute(&self, config: &Config, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
        match self {
            // Global commands
            Self::Init { force } => cmd::new::init_project(config, *force, op),
            Self::SyncCommands { force } => cmd::new::sync_commands(config, *force, op),
            Self::Check { deny_warnings: _ } => cmd::check::check_all(config),
            Self::Status => cmd::status::show_status(config),
            Self::Render {
                target,
                rfc_id,
                dry_run,
                force,
            } => {
                let mut all_diags = vec![];
                match target {
                    RenderTarget::Rfc => {
                        all_diags.extend(cmd::render::render(config, rfc_id.as_deref(), *dry_run)?);
                    }
                    RenderTarget::Adr => {
                        all_diags.extend(cmd::render::render_adrs(config, *dry_run)?);
                    }
                    RenderTarget::Work => {
                        all_diags.extend(cmd::render::render_work_items(config, *dry_run)?);
                    }
                    RenderTarget::Changelog => {
                        all_diags.extend(cmd::render::render_changelog(config, *dry_run, *force)?);
                    }
                    RenderTarget::All => {
                        all_diags.extend(cmd::render::render(config, rfc_id.as_deref(), *dry_run)?);
                        all_diags.extend(cmd::render::render_adrs(config, *dry_run)?);
                        all_diags.extend(cmd::render::render_work_items(config, *dry_run)?);
                    }
                }
                Ok(all_diags)
            }
            Self::Describe { context, format: _ } => cmd::describe::describe(config, *context),
            Self::Completions { shell } => {
                use crate::Cli;
                use clap::CommandFactory;
                let mut cmd = Cli::command();
                clap_complete::generate(*shell, &mut cmd, "govctl", &mut std::io::stdout());
                Ok(vec![])
            }
            #[cfg(feature = "tui")]
            Self::Tui => {
                crate::tui::run(config)?;
                Ok(vec![])
            }

            // RFC commands
            Self::RfcNew { title, id } => {
                let target = NewTarget::Rfc {
                    title: title.clone(),
                    id: id.clone(),
                };
                cmd::new::create(config, &target, op)
            }
            Self::RfcList { filter, limit } => {
                cmd::list::list(config, ListTarget::Rfc, filter.as_deref(), *limit)
            }
            Self::RfcGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::RfcSet {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::set_field(config, id, field, value.as_deref(), *stdin, op),
            Self::RfcBump {
                id,
                level,
                summary,
                changes,
            } => cmd::lifecycle::bump(config, id, *level, summary.as_deref(), changes, op),
            Self::RfcFinalize { id, status } => cmd::lifecycle::finalize(config, id, *status, op),
            Self::RfcAdvance { id, phase } => cmd::lifecycle::advance(config, id, *phase, op),
            Self::RfcDeprecate { id } => cmd::lifecycle::deprecate(config, id, op),
            Self::RfcSupersede { id, by } => cmd::lifecycle::supersede(config, id, by, op),

            // Clause commands
            Self::ClauseNew {
                clause_id,
                title,
                section,
                kind,
            } => {
                let target = NewTarget::Clause {
                    clause_id: clause_id.clone(),
                    title: title.clone(),
                    section: section.clone(),
                    kind: *kind,
                };
                cmd::new::create(config, &target, op)
            }
            Self::ClauseList { rfc_id, limit } => {
                cmd::list::list(config, ListTarget::Clause, rfc_id.as_deref(), *limit)
            }
            Self::ClauseGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::ClauseEdit {
                id,
                text,
                text_file,
                stdin,
            } => cmd::edit::edit_clause(
                config,
                id,
                text.as_deref(),
                text_file.as_deref(),
                *stdin,
                op,
            ),
            Self::ClauseSet {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::set_field(config, id, field, value.as_deref(), *stdin, op),
            Self::ClauseDelete { id, force } => cmd::edit::delete_clause(config, id, *force, op),
            Self::ClauseDeprecate { id } => cmd::lifecycle::deprecate(config, id, op),
            Self::ClauseSupersede { id, by } => cmd::lifecycle::supersede(config, id, by, op),

            // ADR commands
            Self::AdrNew { title } => {
                let target = NewTarget::Adr {
                    title: title.clone(),
                };
                cmd::new::create(config, &target, op)
            }
            Self::AdrList { status, limit } => {
                cmd::list::list(config, ListTarget::Adr, status.as_deref(), *limit)
            }
            Self::AdrGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::AdrSet {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::set_field(config, id, field, value.as_deref(), *stdin, op),
            Self::AdrAdd {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::add_to_field(config, id, field, value.as_deref(), *stdin, None, op),
            Self::AdrRemove {
                id,
                field,
                match_opts,
            } => {
                cmd::edit::remove_from_field(config, id, field, &match_opts.as_match_options(), op)
            }
            Self::AdrAccept { id } => cmd::lifecycle::accept_adr(config, id, op),
            Self::AdrReject { id } => cmd::lifecycle::reject_adr(config, id, op),
            Self::AdrDeprecate { id } => cmd::lifecycle::deprecate(config, id, op),
            Self::AdrSupersede { id, by } => cmd::lifecycle::supersede(config, id, by, op),
            Self::AdrTick {
                id,
                field,
                match_opts,
                status,
            } => cmd::edit::tick_item(
                config,
                id,
                field,
                &match_opts.as_match_options(),
                *status,
                op,
            ),

            // Work item commands
            Self::WorkNew { title, active } => {
                let target = NewTarget::Work {
                    title: title.clone(),
                    active: *active,
                };
                cmd::new::create(config, &target, op)
            }
            Self::WorkList { status, limit } => {
                cmd::list::list(config, ListTarget::Work, status.as_deref(), *limit)
            }
            Self::WorkGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::WorkSet {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::set_field(config, id, field, value.as_deref(), *stdin, op),
            Self::WorkAdd {
                id,
                field,
                value,
                stdin,
                category,
            } => {
                cmd::edit::add_to_field(config, id, field, value.as_deref(), *stdin, *category, op)
            }
            Self::WorkRemove {
                id,
                field,
                match_opts,
            } => {
                cmd::edit::remove_from_field(config, id, field, &match_opts.as_match_options(), op)
            }
            Self::WorkMove { file_or_id, status } => {
                cmd::move_::move_item(config, file_or_id, *status, op)
            }
            Self::WorkTick {
                id,
                field,
                match_opts,
                status,
            } => cmd::edit::tick_item(
                config,
                id,
                field,
                &match_opts.as_match_options(),
                *status,
                op,
            ),
            Self::WorkDelete { id, force } => cmd::edit::delete_work_item(config, id, *force, op),

            // Release commands
            Self::ReleaseCut { version, date } => {
                cmd::lifecycle::cut_release(config, version, date.as_deref(), op)
            }
        }
    }

    // ========================================
    // Resource-First Command Converters (RFC-0002)
    // ========================================

    /// Convert RFC subcommand to canonical form.
    fn from_rfc_command(cmd: &crate::RfcCommand) -> anyhow::Result<Self> {
        use crate::RfcCommand;
        Ok(match cmd {
            RfcCommand::New { title, id } => Self::RfcNew {
                title: title.clone(),
                id: id.clone(),
            },
            RfcCommand::List { filter, limit } => Self::RfcList {
                filter: filter.clone(),
                limit: *limit,
            },
            RfcCommand::Get { id, field } => Self::RfcGet {
                id: id.clone(),
                field: field.clone(),
            },
            RfcCommand::Set {
                id,
                field,
                value,
                stdin,
            } => Self::RfcSet {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            RfcCommand::Bump {
                id,
                patch,
                minor,
                major,
                summary,
                changes,
            } => {
                let level = match (patch, minor, major) {
                    (true, false, false) => Some(BumpLevel::Patch),
                    (false, true, false) => Some(BumpLevel::Minor),
                    (false, false, true) => Some(BumpLevel::Major),
                    (false, false, false) => None,
                    _ => unreachable!("clap arg group ensures mutual exclusivity"),
                };
                Self::RfcBump {
                    id: id.clone(),
                    level,
                    summary: summary.clone(),
                    changes: changes.clone(),
                }
            }
            RfcCommand::Finalize { id, status } => Self::RfcFinalize {
                id: id.clone(),
                status: *status,
            },
            RfcCommand::Advance { id, phase } => Self::RfcAdvance {
                id: id.clone(),
                phase: *phase,
            },
            RfcCommand::Deprecate { id } => Self::RfcDeprecate { id: id.clone() },
            RfcCommand::Supersede { id, by } => Self::RfcSupersede {
                id: id.clone(),
                by: by.clone(),
            },
        })
    }

    /// Convert Clause subcommand to canonical form.
    fn from_clause_command(cmd: &crate::ClauseCommand) -> anyhow::Result<Self> {
        use crate::ClauseCommand;
        Ok(match cmd {
            ClauseCommand::New {
                clause_id,
                title,
                section,
                kind,
            } => Self::ClauseNew {
                clause_id: clause_id.clone(),
                title: title.clone(),
                section: section.clone(),
                kind: *kind,
            },
            ClauseCommand::List { filter, limit } => Self::ClauseList {
                rfc_id: filter.clone(),
                limit: *limit,
            },
            ClauseCommand::Get { id, field } => Self::ClauseGet {
                id: id.clone(),
                field: field.clone(),
            },
            ClauseCommand::Edit {
                id,
                text,
                text_file,
                stdin,
            } => Self::ClauseEdit {
                id: id.clone(),
                text: text.clone(),
                text_file: text_file.clone(),
                stdin: *stdin,
            },
            ClauseCommand::Set {
                id,
                field,
                value,
                stdin,
            } => Self::ClauseSet {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            ClauseCommand::Delete { id, force } => Self::ClauseDelete {
                id: id.clone(),
                force: *force,
            },
            ClauseCommand::Deprecate { id } => Self::ClauseDeprecate { id: id.clone() },
            ClauseCommand::Supersede { id, by } => Self::ClauseSupersede {
                id: id.clone(),
                by: by.clone(),
            },
        })
    }

    /// Convert ADR subcommand to canonical form.
    fn from_adr_command(cmd: &crate::AdrCommand) -> anyhow::Result<Self> {
        use crate::AdrCommand;
        Ok(match cmd {
            AdrCommand::New { title } => Self::AdrNew {
                title: title.clone(),
            },
            AdrCommand::List { filter, limit } => Self::AdrList {
                status: filter.clone(),
                limit: *limit,
            },
            AdrCommand::Get { id, field } => Self::AdrGet {
                id: id.clone(),
                field: field.clone(),
            },
            AdrCommand::Set {
                id,
                field,
                value,
                stdin,
            } => Self::AdrSet {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            AdrCommand::Add {
                id,
                field,
                value,
                stdin,
            } => Self::AdrAdd {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            AdrCommand::Remove {
                id,
                field,
                pattern,
                at,
                exact,
                regex,
                all,
            } => {
                let match_opts = OwnedMatchOptions {
                    pattern: pattern.clone(),
                    at: *at,
                    exact: *exact,
                    regex: *regex,
                    all: *all,
                };
                Self::AdrRemove {
                    id: id.clone(),
                    field: field.clone(),
                    match_opts,
                }
            }
            AdrCommand::Accept { id } => Self::AdrAccept { id: id.clone() },
            AdrCommand::Reject { id } => Self::AdrReject { id: id.clone() },
            AdrCommand::Deprecate { id } => Self::AdrDeprecate { id: id.clone() },
            AdrCommand::Supersede { id, by } => Self::AdrSupersede {
                id: id.clone(),
                by: by.clone(),
            },
            AdrCommand::Tick {
                id,
                field,
                pattern,
                status,
                at,
                exact,
                regex,
            } => {
                let match_opts = OwnedMatchOptions {
                    pattern: pattern.clone(),
                    at: *at,
                    exact: *exact,
                    regex: *regex,
                    all: false,
                };
                Self::AdrTick {
                    id: id.clone(),
                    field: field.clone(),
                    match_opts,
                    status: *status,
                }
            }
        })
    }

    /// Convert Work subcommand to canonical form.
    fn from_work_command(cmd: &crate::WorkCommand) -> anyhow::Result<Self> {
        use crate::WorkCommand;
        Ok(match cmd {
            WorkCommand::New { title, active } => Self::WorkNew {
                title: title.clone(),
                active: *active,
            },
            WorkCommand::List { filter, limit } => Self::WorkList {
                status: filter.clone(),
                limit: *limit,
            },
            WorkCommand::Get { id, field } => Self::WorkGet {
                id: id.clone(),
                field: field.clone(),
            },
            WorkCommand::Set {
                id,
                field,
                value,
                stdin,
            } => Self::WorkSet {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            WorkCommand::Add {
                id,
                field,
                value,
                stdin,
                category,
            } => Self::WorkAdd {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
                category: *category,
            },
            WorkCommand::Remove {
                id,
                field,
                pattern,
                at,
                exact,
                regex,
                all,
            } => {
                let match_opts = OwnedMatchOptions {
                    pattern: pattern.clone(),
                    at: *at,
                    exact: *exact,
                    regex: *regex,
                    all: *all,
                };
                Self::WorkRemove {
                    id: id.clone(),
                    field: field.clone(),
                    match_opts,
                }
            }
            WorkCommand::Move { file, status } => Self::WorkMove {
                file_or_id: file.clone(),
                status: *status,
            },
            WorkCommand::Tick {
                id,
                field,
                pattern,
                status,
                at,
                exact,
                regex,
            } => {
                let match_opts = OwnedMatchOptions {
                    pattern: pattern.clone(),
                    at: *at,
                    exact: *exact,
                    regex: *regex,
                    all: false,
                };
                Self::WorkTick {
                    id: id.clone(),
                    field: field.clone(),
                    match_opts,
                    status: *status,
                }
            }
            WorkCommand::Delete { id, force } => Self::WorkDelete {
                id: id.clone(),
                force: *force,
            },
        })
    }
}
