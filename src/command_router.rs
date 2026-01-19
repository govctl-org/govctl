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
use crate::model::{ClauseKind, RfcPhase, WorkItemStatus};
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
    Check {
        deny_warnings: bool,
    },
    Status,
    Render {
        target: RenderTarget,
        rfc_id: Option<String>,
        dry_run: bool,
    },
    Describe {
        context: bool,
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
            Commands::Check { deny_warnings } => Self::Check {
                deny_warnings: *deny_warnings,
            },
            Commands::Status => Self::Status,
            Commands::Render {
                target,
                rfc_id,
                dry_run,
            } => Self::Render {
                target: *target,
                rfc_id: rfc_id.clone(),
                dry_run: *dry_run,
            },
            Commands::Describe { context, format } => Self::Describe {
                context: *context,
                format: format.clone(),
            },
            Commands::Completions { shell } => Self::Completions { shell: *shell },
            #[cfg(feature = "tui")]
            Commands::Tui => Self::Tui,

            // Old verb-first commands (deprecated) - mapped to canonical form
            Commands::New { target } => match target {
                NewTarget::Rfc { title, id } => Self::RfcNew {
                    title: title.clone(),
                    id: id.clone(),
                },
                NewTarget::Clause {
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
                NewTarget::Adr { title } => Self::AdrNew {
                    title: title.clone(),
                },
                NewTarget::Work { title, active } => Self::WorkNew {
                    title: title.clone(),
                    active: *active,
                },
            },
            Commands::List { target, filter } => match target {
                ListTarget::Rfc => Self::RfcList {
                    filter: filter.clone(),
                },
                ListTarget::Clause => Self::ClauseList {
                    rfc_id: filter.clone(),
                },
                ListTarget::Adr => Self::AdrList {
                    status: filter.clone(),
                },
                ListTarget::Work => Self::WorkList {
                    status: filter.clone(),
                },
            },
            Commands::Get { id, field } => Self::classify_and_route_get(id, field.clone())?,
            Commands::Set {
                id,
                field,
                value,
                stdin,
            } => Self::classify_and_route_set(id, field, value.clone(), *stdin)?,
            Commands::Edit {
                clause_id,
                text,
                text_file,
                stdin,
            } => Self::ClauseEdit {
                id: clause_id.clone(),
                text: text.clone(),
                text_file: text_file.clone(),
                stdin: *stdin,
            },
            Commands::Add {
                id,
                field,
                value,
                stdin,
            } => Self::classify_and_route_add(id, field, value.clone(), *stdin)?,
            Commands::Remove {
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
                Self::classify_and_route_remove(id, field, match_opts)?
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
                    (true, false, false) => Some(BumpLevel::Patch),
                    (false, true, false) => Some(BumpLevel::Minor),
                    (false, false, true) => Some(BumpLevel::Major),
                    (false, false, false) => None,
                    _ => unreachable!("clap arg group ensures mutual exclusivity"),
                };
                Self::RfcBump {
                    id: rfc_id.clone(),
                    level,
                    summary: summary.clone(),
                    changes: changes.clone(),
                }
            }
            Commands::Finalize { rfc_id, status } => Self::RfcFinalize {
                id: rfc_id.clone(),
                status: *status,
            },
            Commands::Advance { rfc_id, phase } => Self::RfcAdvance {
                id: rfc_id.clone(),
                phase: *phase,
            },
            Commands::Accept { adr } => Self::AdrAccept { id: adr.clone() },
            Commands::Reject { adr } => Self::AdrReject { id: adr.clone() },
            Commands::Deprecate { id } => Self::classify_and_route_deprecate(id)?,
            Commands::Supersede { id, by } => Self::classify_and_route_supersede(id, by)?,
            Commands::Delete {
                id,
                force,
                clause,
                work,
            } => {
                // Auto-detect artifact type from ID format if not explicitly specified
                let is_clause = if *clause || *work {
                    *clause
                } else {
                    id.contains(':')
                };

                if is_clause {
                    Self::ClauseDelete {
                        id: id.clone(),
                        force: *force,
                    }
                } else {
                    Self::WorkDelete {
                        id: id.clone(),
                        force: *force,
                    }
                }
            }
            Commands::Move { file, status } => Self::WorkMove {
                file_or_id: file.clone(),
                status: *status,
            },
            Commands::Tick {
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
                Self::classify_and_route_tick(id, field, match_opts, *status)?
            }
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
            Self::Check { deny_warnings: _ } => cmd::check::check_all(config),
            Self::Status => cmd::status::show_status(config),
            Self::Render {
                target,
                rfc_id,
                dry_run,
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
                        all_diags.extend(cmd::render::render_changelog(config, *dry_run)?);
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
            Self::RfcList { filter } => cmd::list::list(config, ListTarget::Rfc, filter.as_deref()),
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
            Self::ClauseList { rfc_id } => {
                cmd::list::list(config, ListTarget::Clause, rfc_id.as_deref())
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
            Self::AdrList { status } => cmd::list::list(config, ListTarget::Adr, status.as_deref()),
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
            } => cmd::edit::add_to_field(config, id, field, value.as_deref(), *stdin, op),
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
            Self::WorkList { status } => {
                cmd::list::list(config, ListTarget::Work, status.as_deref())
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
            } => cmd::edit::add_to_field(config, id, field, value.as_deref(), *stdin, op),
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
    // Artifact Type Classification Helpers
    // ========================================

    /// Classify artifact ID and route to appropriate Get variant.
    fn classify_and_route_get(id: &str, field: Option<String>) -> anyhow::Result<Self> {
        Ok(if id.contains(':') {
            Self::ClauseGet {
                id: id.to_string(),
                field,
            }
        } else if id.starts_with("RFC-") {
            Self::RfcGet {
                id: id.to_string(),
                field,
            }
        } else if id.starts_with("ADR-") {
            Self::AdrGet {
                id: id.to_string(),
                field,
            }
        } else if id.starts_with("WI-") {
            Self::WorkGet {
                id: id.to_string(),
                field,
            }
        } else {
            anyhow::bail!("Cannot determine artifact type from ID: {}", id)
        })
    }

    /// Classify artifact ID and route to appropriate Set variant.
    fn classify_and_route_set(
        id: &str,
        field: &str,
        value: Option<String>,
        stdin: bool,
    ) -> anyhow::Result<Self> {
        Ok(if id.contains(':') {
            Self::ClauseSet {
                id: id.to_string(),
                field: field.to_string(),
                value,
                stdin,
            }
        } else if id.starts_with("RFC-") {
            Self::RfcSet {
                id: id.to_string(),
                field: field.to_string(),
                value,
                stdin,
            }
        } else if id.starts_with("ADR-") {
            Self::AdrSet {
                id: id.to_string(),
                field: field.to_string(),
                value,
                stdin,
            }
        } else if id.starts_with("WI-") {
            Self::WorkSet {
                id: id.to_string(),
                field: field.to_string(),
                value,
                stdin,
            }
        } else {
            anyhow::bail!("Cannot determine artifact type from ID: {}", id)
        })
    }

    /// Classify artifact ID and route to appropriate Add variant.
    fn classify_and_route_add(
        id: &str,
        field: &str,
        value: Option<String>,
        stdin: bool,
    ) -> anyhow::Result<Self> {
        // Add only applies to ADR and Work items (not RFC or Clause)
        Ok(if id.starts_with("ADR-") {
            Self::AdrAdd {
                id: id.to_string(),
                field: field.to_string(),
                value,
                stdin,
            }
        } else if id.starts_with("WI-") {
            Self::WorkAdd {
                id: id.to_string(),
                field: field.to_string(),
                value,
                stdin,
            }
        } else {
            anyhow::bail!(
                "Add command only applies to ADR-* and WI-* artifacts, got: {}",
                id
            )
        })
    }

    /// Classify artifact ID and route to appropriate Remove variant.
    fn classify_and_route_remove(
        id: &str,
        field: &str,
        match_opts: OwnedMatchOptions,
    ) -> anyhow::Result<Self> {
        // Remove only applies to ADR and Work items
        Ok(if id.starts_with("ADR-") {
            Self::AdrRemove {
                id: id.to_string(),
                field: field.to_string(),
                match_opts,
            }
        } else if id.starts_with("WI-") {
            Self::WorkRemove {
                id: id.to_string(),
                field: field.to_string(),
                match_opts,
            }
        } else {
            anyhow::bail!(
                "Remove command only applies to ADR-* and WI-* artifacts, got: {}",
                id
            )
        })
    }

    /// Classify artifact ID and route to appropriate Deprecate variant.
    fn classify_and_route_deprecate(id: &str) -> anyhow::Result<Self> {
        Ok(if id.contains(':') {
            Self::ClauseDeprecate { id: id.to_string() }
        } else if id.starts_with("RFC-") {
            Self::RfcDeprecate { id: id.to_string() }
        } else if id.starts_with("ADR-") {
            Self::AdrDeprecate { id: id.to_string() }
        } else {
            anyhow::bail!("Deprecate only applies to RFC, Clause, or ADR, got: {}", id)
        })
    }

    /// Classify artifact ID and route to appropriate Supersede variant.
    fn classify_and_route_supersede(id: &str, by: &str) -> anyhow::Result<Self> {
        Ok(if id.contains(':') {
            Self::ClauseSupersede {
                id: id.to_string(),
                by: by.to_string(),
            }
        } else if id.starts_with("RFC-") {
            Self::RfcSupersede {
                id: id.to_string(),
                by: by.to_string(),
            }
        } else if id.starts_with("ADR-") {
            Self::AdrSupersede {
                id: id.to_string(),
                by: by.to_string(),
            }
        } else {
            anyhow::bail!("Supersede only applies to RFC, Clause, or ADR, got: {}", id)
        })
    }

    /// Classify artifact ID and route to appropriate Tick variant.
    fn classify_and_route_tick(
        id: &str,
        field: &str,
        match_opts: OwnedMatchOptions,
        status: TickStatus,
    ) -> anyhow::Result<Self> {
        // Tick only applies to ADR and Work items
        Ok(if id.starts_with("ADR-") {
            Self::AdrTick {
                id: id.to_string(),
                field: field.to_string(),
                match_opts,
                status,
            }
        } else if id.starts_with("WI-") {
            Self::WorkTick {
                id: id.to_string(),
                field: field.to_string(),
                match_opts,
                status,
            }
        } else {
            anyhow::bail!(
                "Tick command only applies to ADR-* and WI-* artifacts, got: {}",
                id
            )
        })
    }
}
