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
use crate::{
    Commands, EditActionArgs, FinalizeStatus, GuardCommand, ListTarget, NewTarget, OutputFormat,
    RenderTarget, TickStatus,
};
use std::path::PathBuf;

type OwnedMatchOptions = cmd::edit::MatchOptionsOwned;
type OwnedEditAction = cmd::edit::OwnedEditAction;

fn owned_edit_action(args: &EditActionArgs) -> anyhow::Result<OwnedEditAction> {
    let empty_to_none = |value: Option<&String>| {
        value.and_then(|value| {
            if value.is_empty() {
                None
            } else {
                Some(value.clone())
            }
        })
    };

    if let Some(value) = &args.set {
        return Ok(OwnedEditAction::Set {
            value: if value.is_empty() {
                None
            } else {
                Some(value.clone())
            },
            stdin: args.stdin,
        });
    }
    if let Some(value) = &args.add {
        return Ok(OwnedEditAction::Add {
            value: if value.is_empty() {
                None
            } else {
                Some(value.clone())
            },
            stdin: args.stdin,
        });
    }
    if let Some(status) = args.tick {
        return Ok(OwnedEditAction::Tick {
            match_opts: OwnedMatchOptions {
                pattern: None,
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: false,
            },
            status,
        });
    }
    if args.remove.is_some() {
        return Ok(OwnedEditAction::Remove {
            match_opts: OwnedMatchOptions {
                pattern: empty_to_none(args.remove.as_ref()),
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: args.all,
            },
        });
    }

    Err(anyhow::anyhow!("exactly one edit action is required"))
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
    InitSkills {
        force: bool,
    },
    Check {
        #[allow(dead_code)]
        deny_warnings: bool,
        has_active: bool,
    },
    Status,
    Render {
        target: RenderTarget,
        dry_run: bool,
        force: bool,
    },
    Migrate,
    Verify {
        guard_ids: Vec<String>,
        work: Option<String>,
    },
    Describe {
        context: bool,
        #[allow(dead_code)]
        output: String,
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
        output: OutputFormat,
    },
    RfcGet {
        id: String,
        field: Option<String>,
    },
    RfcEdit {
        id: String,
        path: String,
        action: OwnedEditAction,
    },
    RfcSet {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    RfcAdd {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    RfcRemove {
        id: String,
        field: String,
        match_opts: OwnedMatchOptions,
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
        force: bool,
    },
    RfcSupersede {
        id: String,
        by: String,
        force: bool,
    },
    RfcRender {
        id: String,
        dry_run: bool,
    },
    RfcShow {
        id: String,
        output: OutputFormat,
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
        output: OutputFormat,
    },
    ClauseGet {
        id: String,
        field: Option<String>,
    },
    ClauseEdit {
        id: String,
        path: String,
        action: OwnedEditAction,
    },
    ClauseLegacyEdit {
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
        force: bool,
    },
    ClauseSupersede {
        id: String,
        by: String,
        force: bool,
    },
    ClauseShow {
        id: String,
        output: OutputFormat,
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
        output: OutputFormat,
    },
    AdrGet {
        id: String,
        field: Option<String>,
    },
    AdrEdit {
        id: String,
        path: String,
        action: OwnedEditAction,
        pro: Vec<String>,
        con: Vec<String>,
        reject_reason: Option<String>,
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
        pro: Vec<String>,
        con: Vec<String>,
        reject_reason: Option<String>,
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
        force: bool,
    },
    AdrSupersede {
        id: String,
        by: String,
        force: bool,
    },
    AdrRender {
        id: String,
        dry_run: bool,
    },
    AdrShow {
        id: String,
        output: OutputFormat,
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
        output: OutputFormat,
    },
    WorkGet {
        id: String,
        field: Option<String>,
    },
    WorkEdit {
        id: String,
        path: String,
        action: OwnedEditAction,
        category: Option<ChangelogCategory>,
        scope: Option<String>,
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
        scope: Option<String>,
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
    WorkRender {
        id: String,
        dry_run: bool,
    },
    WorkShow {
        id: String,
        output: OutputFormat,
    },

    // ========================================
    // Guard Commands
    // ========================================
    GuardNew {
        title: String,
    },
    GuardList {
        filter: Option<String>,
        limit: Option<usize>,
        output: OutputFormat,
    },
    GuardGet {
        id: String,
        field: Option<String>,
    },
    GuardEdit {
        id: String,
        path: String,
        action: OwnedEditAction,
    },
    GuardSet {
        id: String,
        field: String,
        value: Option<String>,
        stdin: bool,
    },
    GuardAdd {
        id: String,
        field: String,
        value: String,
    },
    GuardRemove {
        id: String,
        field: String,
        match_opts: OwnedMatchOptions,
    },
    GuardDelete {
        id: String,
        force: bool,
    },
    GuardShow {
        id: String,
        output: OutputFormat,
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
    /// Returns true if this command modifies gov/ or writes to docs/ (per RFC-0004 C-SCOPE).
    /// Such commands must acquire the gov-root exclusive lock before running.
    pub fn is_write_command(&self) -> bool {
        use CanonicalCommand::*;
        matches!(
            self,
            Init { .. }
                | InitSkills { .. }
                | Render { .. }
                | Migrate
                | RfcNew { .. }
                | RfcSet { .. }
                | RfcAdd { .. }
                | RfcRemove { .. }
                | RfcBump { .. }
                | RfcFinalize { .. }
                | RfcAdvance { .. }
                | RfcDeprecate { .. }
                | RfcSupersede { .. }
                | RfcRender { .. }
                | ClauseNew { .. }
                | ClauseEdit { .. }
                | ClauseSet { .. }
                | ClauseDelete { .. }
                | ClauseDeprecate { .. }
                | ClauseSupersede { .. }
                | AdrNew { .. }
                | AdrSet { .. }
                | AdrAdd { .. }
                | AdrRemove { .. }
                | AdrAccept { .. }
                | AdrReject { .. }
                | AdrDeprecate { .. }
                | AdrSupersede { .. }
                | AdrRender { .. }
                | WorkNew { .. }
                | WorkSet { .. }
                | WorkAdd { .. }
                | WorkRemove { .. }
                | WorkMove { .. }
                | WorkTick { .. }
                | WorkDelete { .. }
                | WorkRender { .. }
                | GuardNew { .. }
                | GuardSet { .. }
                | GuardAdd { .. }
                | GuardRemove { .. }
                | GuardDelete { .. }
                | ReleaseCut { .. }
        )
    }

    /// Convert parsed CLI commands to canonical form.
    ///
    /// This is where both old (deprecated) and new (resource-first) syntaxes
    /// are unified into a single representation.
    pub fn from_parsed(cmd: &Commands) -> anyhow::Result<Self> {
        Ok(match cmd {
            // Global commands
            Commands::Init { force } => Self::Init { force: *force },
            Commands::InitSkills { force } => Self::InitSkills { force: *force },
            Commands::Check {
                deny_warnings,
                has_active,
            } => Self::Check {
                deny_warnings: *deny_warnings,
                has_active: *has_active,
            },
            Commands::Status => Self::Status,
            Commands::Render {
                target,
                dry_run,
                force,
            } => Self::Render {
                target: *target,
                dry_run: *dry_run,
                force: *force,
            },
            Commands::Migrate => Self::Migrate,
            Commands::Verify { guard_ids, work } => Self::Verify {
                guard_ids: guard_ids.clone(),
                work: work.clone(),
            },
            Commands::Describe { context, output } => Self::Describe {
                context: *context,
                output: output.clone(),
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
            Commands::Guard { command } => Self::from_guard_command(command)?,

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
            Self::InitSkills { force } => cmd::new::sync_skills(config, *force, op),
            Self::Check {
                deny_warnings: _,
                has_active: true,
            } => cmd::check::check_has_active(config),
            Self::Check {
                deny_warnings: _,
                has_active: false,
            } => cmd::check::check_all(config),
            Self::Status => cmd::status::show_status(config),
            Self::Render {
                target,
                dry_run,
                force,
            } => {
                let mut all_diags = vec![];
                match target {
                    RenderTarget::Rfc => {
                        all_diags.extend(cmd::render::render(config, None, *dry_run)?);
                    }
                    RenderTarget::Adr => {
                        all_diags.extend(cmd::render::render_adrs(config, None, *dry_run)?);
                    }
                    RenderTarget::Work => {
                        all_diags.extend(cmd::render::render_work_items(config, None, *dry_run)?);
                    }
                    RenderTarget::Changelog => {
                        all_diags.extend(cmd::render::render_changelog(config, *dry_run, *force)?);
                    }
                    RenderTarget::All => {
                        all_diags.extend(cmd::render::render(config, None, *dry_run)?);
                        all_diags.extend(cmd::render::render_adrs(config, None, *dry_run)?);
                        all_diags.extend(cmd::render::render_work_items(config, None, *dry_run)?);
                    }
                }
                Ok(all_diags)
            }
            Self::Migrate => cmd::migrate::migrate(config, op),
            Self::Verify { guard_ids, work } => {
                cmd::verify::verify(config, guard_ids, work.as_deref())
            }
            Self::Describe { context, output: _ } => cmd::describe::describe(config, *context),
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
            Self::RfcList {
                filter,
                limit,
                output,
            } => cmd::list::list(config, ListTarget::Rfc, filter.as_deref(), *limit, *output),
            Self::RfcGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::RfcEdit { id, path, action } => {
                cmd::edit::edit_field(config, id, path, action, None, None, None, None, None, op)
            }
            Self::RfcSet {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::set_field(config, id, field, value.as_deref(), *stdin, op),
            Self::RfcAdd {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::add_to_field(
                config,
                id,
                field,
                value.as_deref(),
                *stdin,
                None,
                None,
                None,
                None,
                None,
                op,
            ),
            Self::RfcRemove {
                id,
                field,
                match_opts,
            } => {
                cmd::edit::remove_from_field(config, id, field, &match_opts.as_match_options(), op)
            }
            Self::RfcBump {
                id,
                level,
                summary,
                changes,
            } => cmd::lifecycle::bump(config, id, *level, summary.as_deref(), changes, op),
            Self::RfcFinalize { id, status } => cmd::lifecycle::finalize(config, id, *status, op),
            Self::RfcAdvance { id, phase } => cmd::lifecycle::advance(config, id, *phase, op),
            Self::RfcDeprecate { id, force } => cmd::lifecycle::deprecate(config, id, *force, op),
            Self::RfcSupersede { id, by, force } => {
                cmd::lifecycle::supersede(config, id, by, *force, op)
            }
            Self::RfcRender { id, dry_run } => cmd::render::render(config, Some(id), *dry_run),
            Self::RfcShow { id, output } => cmd::render::show_rfc(config, id, *output),

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
            Self::ClauseList {
                rfc_id,
                limit,
                output,
            } => cmd::list::list(
                config,
                ListTarget::Clause,
                rfc_id.as_deref(),
                *limit,
                *output,
            ),
            Self::ClauseGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::ClauseEdit { id, path, action } => {
                cmd::edit::edit_field(config, id, path, action, None, None, None, None, None, op)
            }
            Self::ClauseLegacyEdit {
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
            Self::ClauseDeprecate { id, force } => {
                cmd::lifecycle::deprecate(config, id, *force, op)
            }
            Self::ClauseSupersede { id, by, force } => {
                cmd::lifecycle::supersede(config, id, by, *force, op)
            }
            Self::ClauseShow { id, output } => cmd::render::show_clause(config, id, *output),

            // ADR commands
            Self::AdrNew { title } => {
                let target = NewTarget::Adr {
                    title: title.clone(),
                };
                cmd::new::create(config, &target, op)
            }
            Self::AdrList {
                status,
                limit,
                output,
            } => cmd::list::list(config, ListTarget::Adr, status.as_deref(), *limit, *output),
            Self::AdrGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::AdrEdit {
                id,
                path,
                action,
                pro,
                con,
                reject_reason,
            } => cmd::edit::edit_field(
                config,
                id,
                path,
                action,
                None,
                None,
                Some(pro.clone()),
                Some(con.clone()),
                reject_reason.clone(),
                op,
            ),
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
                pro,
                con,
                reject_reason,
            } => cmd::edit::add_to_field(
                config,
                id,
                field,
                value.as_deref(),
                *stdin,
                None,
                None,
                Some(pro.clone()),
                Some(con.clone()),
                reject_reason.clone(),
                op,
            ),
            Self::AdrRemove {
                id,
                field,
                match_opts,
            } => {
                cmd::edit::remove_from_field(config, id, field, &match_opts.as_match_options(), op)
            }
            Self::AdrAccept { id } => cmd::lifecycle::accept_adr(config, id, op),
            Self::AdrReject { id } => cmd::lifecycle::reject_adr(config, id, op),
            Self::AdrDeprecate { id, force } => cmd::lifecycle::deprecate(config, id, *force, op),
            Self::AdrSupersede { id, by, force } => {
                cmd::lifecycle::supersede(config, id, by, *force, op)
            }
            Self::AdrRender { id, dry_run } => cmd::render::render_adrs(config, Some(id), *dry_run),
            Self::AdrShow { id, output } => cmd::render::show_adr(config, id, *output),

            // Work item commands
            Self::WorkNew { title, active } => {
                let target = NewTarget::Work {
                    title: title.clone(),
                    active: *active,
                };
                cmd::new::create(config, &target, op)
            }
            Self::WorkList {
                status,
                limit,
                output,
            } => cmd::list::list(config, ListTarget::Work, status.as_deref(), *limit, *output),
            Self::WorkGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::WorkEdit {
                id,
                path,
                action,
                category,
                scope,
            } => cmd::edit::edit_field(
                config,
                id,
                path,
                action,
                *category,
                scope.as_deref(),
                None,
                None,
                None,
                op,
            ),
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
                scope,
            } => cmd::edit::add_to_field(
                config,
                id,
                field,
                value.as_deref(),
                *stdin,
                *category,
                scope.as_deref(),
                None,
                None,
                None,
                op,
            ),
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
            Self::WorkRender { id, dry_run } => {
                cmd::render::render_work_items(config, Some(id), *dry_run)
            }
            Self::WorkShow { id, output } => cmd::render::show_work(config, id, *output),

            // Guard commands
            Self::GuardNew { title } => cmd::guard::new_guard(config, title, op),
            Self::GuardList {
                filter,
                limit,
                output,
            } => cmd::list::list(
                config,
                ListTarget::Guard,
                filter.as_deref(),
                *limit,
                *output,
            ),
            Self::GuardGet { id, field } => cmd::edit::get_field(config, id, field.as_deref()),
            Self::GuardEdit { id, path, action } => {
                cmd::edit::edit_field(config, id, path, action, None, None, None, None, None, op)
            }
            Self::GuardSet {
                id,
                field,
                value,
                stdin,
            } => cmd::edit::set_field(config, id, field, value.as_deref(), *stdin, op),
            Self::GuardAdd { id, field, value } => cmd::edit::add_to_field(
                config,
                id,
                field,
                Some(value.as_str()),
                false,
                None,
                None,
                None,
                None,
                None,
                op,
            ),
            Self::GuardRemove {
                id,
                field,
                match_opts,
            } => {
                cmd::edit::remove_from_field(config, id, field, &match_opts.as_match_options(), op)
            }
            Self::GuardDelete { id, force } => cmd::guard::delete_guard(config, id, *force, op),
            Self::GuardShow { id, output } => cmd::guard::show_guard(config, id, *output),

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
            RfcCommand::List {
                filter,
                limit,
                output,
            } => Self::RfcList {
                filter: filter.clone(),
                limit: *limit,
                output: *output,
            },
            RfcCommand::Get { id, field } => Self::RfcGet {
                id: id.clone(),
                field: field.clone(),
            },
            RfcCommand::Edit { id, path, action } => Self::RfcEdit {
                id: id.clone(),
                path: path.clone(),
                action: owned_edit_action(action)?,
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
            RfcCommand::Add {
                id,
                field,
                value,
                stdin,
            } => Self::RfcAdd {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            RfcCommand::Remove {
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
                Self::RfcRemove {
                    id: id.clone(),
                    field: field.clone(),
                    match_opts,
                }
            }
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
            RfcCommand::Deprecate { id, force } => Self::RfcDeprecate {
                id: id.clone(),
                force: *force,
            },
            RfcCommand::Supersede { id, by, force } => Self::RfcSupersede {
                id: id.clone(),
                by: by.clone(),
                force: *force,
            },
            RfcCommand::Render { id, dry_run } => Self::RfcRender {
                id: id.clone(),
                dry_run: *dry_run,
            },
            RfcCommand::Show { id, output } => Self::RfcShow {
                id: id.clone(),
                output: *output,
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
            ClauseCommand::List {
                filter,
                limit,
                output,
            } => Self::ClauseList {
                rfc_id: filter.clone(),
                limit: *limit,
                output: *output,
            },
            ClauseCommand::Get { id, field } => Self::ClauseGet {
                id: id.clone(),
                field: field.clone(),
            },
            ClauseCommand::Edit {
                id,
                path,
                set,
                add,
                remove,
                tick,
                stdin,
                at,
                exact,
                regex,
                all,
                text,
                text_file,
            } => {
                let uses_canonical = path.is_some()
                    || set.is_some()
                    || add.is_some()
                    || remove.is_some()
                    || tick.is_some();
                if uses_canonical {
                    let path = path.clone().ok_or_else(|| {
                        anyhow::anyhow!(
                            "canonical clause edit requires a field path before --set/--add/--remove/--tick"
                        )
                    })?;
                    let action = owned_edit_action(&EditActionArgs {
                        set: set.clone(),
                        add: add.clone(),
                        remove: remove.clone(),
                        tick: *tick,
                        stdin: *stdin,
                        at: *at,
                        exact: *exact,
                        regex: *regex,
                        all: *all,
                    })?;
                    Self::ClauseEdit {
                        id: id.clone(),
                        path,
                        action,
                    }
                } else {
                    Self::ClauseLegacyEdit {
                        id: id.clone(),
                        text: text.clone(),
                        text_file: text_file.clone(),
                        stdin: *stdin,
                    }
                }
            }
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
            ClauseCommand::Deprecate { id, force } => Self::ClauseDeprecate {
                id: id.clone(),
                force: *force,
            },
            ClauseCommand::Supersede { id, by, force } => Self::ClauseSupersede {
                id: id.clone(),
                by: by.clone(),
                force: *force,
            },
            ClauseCommand::Show { id, output } => Self::ClauseShow {
                id: id.clone(),
                output: *output,
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
            AdrCommand::List {
                filter,
                limit,
                output,
            } => Self::AdrList {
                status: filter.clone(),
                limit: *limit,
                output: *output,
            },
            AdrCommand::Get { id, field } => Self::AdrGet {
                id: id.clone(),
                field: field.clone(),
            },
            AdrCommand::Edit {
                id,
                path,
                action,
                pro,
                con,
                reject_reason,
            } => Self::AdrEdit {
                id: id.clone(),
                path: path.clone(),
                action: owned_edit_action(action)?,
                pro: pro.clone(),
                con: con.clone(),
                reject_reason: reject_reason.clone(),
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
                pro: adr_pro,
                con: adr_con,
                reject_reason,
            } => Self::AdrAdd {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
                pro: adr_pro.to_vec(),
                con: adr_con.to_vec(),
                reject_reason: reject_reason.clone(),
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
            AdrCommand::Deprecate { id, force } => Self::AdrDeprecate {
                id: id.clone(),
                force: *force,
            },
            AdrCommand::Supersede { id, by, force } => Self::AdrSupersede {
                id: id.clone(),
                by: by.clone(),
                force: *force,
            },
            AdrCommand::Render { id, dry_run } => Self::AdrRender {
                id: id.clone(),
                dry_run: *dry_run,
            },
            AdrCommand::Show { id, output } => Self::AdrShow {
                id: id.clone(),
                output: *output,
            },
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
            WorkCommand::List {
                filter,
                limit,
                output,
            } => Self::WorkList {
                status: filter.clone(),
                limit: *limit,
                output: *output,
            },
            WorkCommand::Get { id, field } => Self::WorkGet {
                id: id.clone(),
                field: field.clone(),
            },
            WorkCommand::Edit {
                id,
                path,
                action,
                category,
                scope,
            } => Self::WorkEdit {
                id: id.clone(),
                path: path.clone(),
                action: owned_edit_action(action)?,
                category: *category,
                scope: scope.clone(),
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
                scope,
            } => Self::WorkAdd {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
                category: *category,
                scope: scope.clone(),
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
            WorkCommand::Render { id, dry_run } => Self::WorkRender {
                id: id.clone(),
                dry_run: *dry_run,
            },
            WorkCommand::Show { id, output } => Self::WorkShow {
                id: id.clone(),
                output: *output,
            },
        })
    }

    /// Convert Guard subcommand to canonical form.
    fn from_guard_command(cmd: &GuardCommand) -> anyhow::Result<Self> {
        Ok(match cmd {
            GuardCommand::New { title } => Self::GuardNew {
                title: title.clone(),
            },
            GuardCommand::List {
                filter,
                limit,
                output,
            } => Self::GuardList {
                filter: filter.clone(),
                limit: *limit,
                output: *output,
            },
            GuardCommand::Get { id, field } => Self::GuardGet {
                id: id.clone(),
                field: field.clone(),
            },
            GuardCommand::Edit { id, path, action } => Self::GuardEdit {
                id: id.clone(),
                path: path.clone(),
                action: owned_edit_action(action)?,
            },
            GuardCommand::Set {
                id,
                field,
                value,
                stdin,
            } => Self::GuardSet {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
                stdin: *stdin,
            },
            GuardCommand::Add { id, field, value } => Self::GuardAdd {
                id: id.clone(),
                field: field.clone(),
                value: value.clone(),
            },
            GuardCommand::Remove {
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
                Self::GuardRemove {
                    id: id.clone(),
                    field: field.clone(),
                    match_opts,
                }
            }
            GuardCommand::Delete { id, force } => Self::GuardDelete {
                id: id.clone(),
                force: *force,
            },
            GuardCommand::Show { id, output } => Self::GuardShow {
                id: id.clone(),
                output: *output,
            },
        })
    }
}
