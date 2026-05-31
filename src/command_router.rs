//! Command planning for unified routing semantics.
//!
//! This module compiles parsed CLI syntax into semantic execution plans built
//! from `Scope + Op`. The planner is the single normalization point for both
//! canonical and compatibility command forms.

mod edit_action;
mod execute;

use crate::cmd;
use crate::config::Config;
use crate::diagnostic::Diagnostic;
use crate::model::{ChangelogCategory, ClauseKind, RfcPhase, WorkItemStatus};
use crate::write::{BumpLevel, WriteOp};
use crate::{Commands, FinalizeStatus, ListTarget, OutputFormat, RenderTarget};
use std::path::PathBuf;

pub(crate) type OwnedMatchOptions = cmd::edit::MatchOptionsOwned;
pub(crate) type OwnedEditAction = cmd::edit::OwnedEditAction;

pub(crate) use edit_action::{
    add_action, owned_edit_action, remove_action, set_action, tick_action,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Global,
    Collection {
        target: ListTarget,
    },
    Artifact {
        artifact: cmd::edit::ArtifactType,
        id: String,
    },
    Target {
        artifact: cmd::edit::ArtifactType,
        id: String,
        target: cmd::edit::engine::ResolvedTarget,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditExtras {
    pub category: Option<ChangelogCategory>,
    pub scope: Option<String>,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub reject_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BuiltinOp {
    Init {
        force: bool,
    },
    InitSkills {
        force: bool,
        format: crate::SkillFormat,
        dir: Option<std::path::PathBuf>,
    },
    Check {
        has_active: bool,
    },
    Status,
    RenderGlobal {
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
    },
    Completions {
        shell: clap_complete::Shell,
    },
    SelfUpdate {
        check: bool,
    },
    #[cfg(feature = "tui")]
    Tui,
    ReleaseCut {
        version: String,
        date: Option<String>,
    },
    TagNew {
        tag: String,
    },
    TagDelete {
        tag: String,
    },
    TagList {
        output: crate::OutputFormat,
    },
    LoopStart {
        loop_id: Option<String>,
        work_items: Vec<String>,
    },
    LoopShow {
        loop_id: String,
    },
    LoopResume {
        loop_id: Option<String>,
        work_items: Vec<String>,
    },
    LoopRun {
        loop_id: Option<String>,
        work_items: Vec<String>,
        max_rounds: u32,
    },
}

#[derive(Debug, Clone)]
pub enum CreateOp {
    Rfc {
        title: String,
        id: Option<String>,
    },
    Clause {
        clause_id: String,
        title: String,
        section: String,
        kind: ClauseKind,
    },
    Adr {
        title: String,
    },
    Work {
        title: String,
        active: bool,
    },
    Guard {
        title: String,
    },
}

#[derive(Debug, Clone)]
pub enum EditOp {
    Field {
        action: OwnedEditAction,
        extras: EditExtras,
    },
    ClauseLegacy {
        text: Option<String>,
        text_file: Option<PathBuf>,
        stdin: bool,
    },
}

#[derive(Debug, Clone)]
pub enum LifecycleOp {
    Bump {
        level: Option<BumpLevel>,
        summary: Option<String>,
        changes: Vec<String>,
    },
    Finalize {
        status: FinalizeStatus,
    },
    Advance {
        phase: RfcPhase,
    },
    Deprecate {
        force: bool,
    },
    Supersede {
        by: String,
        force: bool,
    },
    AcceptAdr {
        force: bool,
    },
    RejectAdr,
    MoveWork {
        file_or_id: PathBuf,
        status: WorkItemStatus,
    },
}

#[derive(Debug, Clone)]
pub enum Op {
    Builtin(BuiltinOp),
    Create(CreateOp),
    List {
        filter: Option<String>,
        limit: Option<usize>,
        output: OutputFormat,
        /// Tags to filter by (artifact must have ALL specified tags) — [[RFC-0002:C-CRUD-VERBS]]
        tags: Vec<String>,
    },
    Get,
    Show {
        output: OutputFormat,
    },
    Edit(EditOp),
    Lifecycle(LifecycleOp),
    Delete {
        force: bool,
    },
    RenderArtifact {
        dry_run: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockDisposition {
    None,
    GovRootExclusive,
}

#[derive(Debug, Clone)]
pub struct CommandPlan {
    pub scope: Scope,
    pub op: Op,
}

impl CommandPlan {
    fn new(scope: Scope, op: Op) -> Self {
        Self { scope, op }
    }

    pub fn lock_disposition(&self) -> LockDisposition {
        match &self.op {
            Op::Builtin(BuiltinOp::Check { .. })
            | Op::Builtin(BuiltinOp::Status)
            | Op::Builtin(BuiltinOp::Verify { .. })
            | Op::Builtin(BuiltinOp::Describe { .. })
            | Op::Builtin(BuiltinOp::Completions { .. })
            | Op::Builtin(BuiltinOp::SelfUpdate { .. })
            | Op::Builtin(BuiltinOp::TagList { .. })
            | Op::Builtin(BuiltinOp::LoopShow { .. })
            | Op::Builtin(BuiltinOp::LoopResume { .. })
            | Op::Get
            | Op::List { .. }
            | Op::Show { .. } => LockDisposition::None,
            #[cfg(feature = "tui")]
            Op::Builtin(BuiltinOp::Tui) => LockDisposition::None,
            _ => LockDisposition::GovRootExclusive,
        }
    }

    pub fn execute(&self, config: &Config, op: WriteOp) -> anyhow::Result<Vec<Diagnostic>> {
        execute::execute_plan(self, config, op)
    }
}

fn artifact_scope(artifact: cmd::edit::ArtifactType, id: &str) -> Scope {
    Scope::Artifact {
        artifact,
        id: id.to_string(),
    }
}

fn resolve_scope(id: &str, field: Option<&str>) -> anyhow::Result<Scope> {
    let plan = cmd::edit::engine::plan_request(id, field)?;
    Ok(match plan.target {
        Some(target) => Scope::Target {
            artifact: plan.artifact,
            id: id.to_string(),
            target,
        },
        None => artifact_scope(plan.artifact, id),
    })
}

fn global(op: Op) -> CommandPlan {
    CommandPlan::new(Scope::Global, op)
}

fn collection(target: ListTarget, op: Op) -> CommandPlan {
    CommandPlan::new(Scope::Collection { target }, op)
}

pub(crate) fn artifact(artifact: cmd::edit::ArtifactType, id: &str, op: Op) -> CommandPlan {
    CommandPlan::new(artifact_scope(artifact, id), op)
}

fn target(id: &str, field: Option<&str>, op: Op) -> anyhow::Result<CommandPlan> {
    Ok(CommandPlan::new(resolve_scope(id, field)?, op))
}

fn edit_op_with_extras(action: OwnedEditAction, extras: EditExtras) -> Op {
    Op::Edit(EditOp::Field { action, extras })
}

pub(crate) fn plan_create(collection_target: ListTarget, create: CreateOp) -> CommandPlan {
    collection(collection_target, Op::Create(create))
}

pub(crate) fn plan_list(
    target_kind: ListTarget,
    filter: Option<String>,
    limit: Option<usize>,
    output: OutputFormat,
    tags: Vec<String>,
) -> CommandPlan {
    collection(
        target_kind,
        Op::List {
            filter,
            limit,
            output,
            tags,
        },
    )
}

pub(crate) fn plan_get(id: &str, field: Option<&str>) -> anyhow::Result<CommandPlan> {
    target(id, field, Op::Get)
}

pub(crate) fn plan_show(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    output: OutputFormat,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Show { output })
}

pub(crate) fn plan_edit(
    id: &str,
    field: &str,
    action: OwnedEditAction,
    extras: EditExtras,
) -> anyhow::Result<CommandPlan> {
    target(id, Some(field), edit_op_with_extras(action, extras))
}

pub(crate) fn plan_lifecycle(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    lifecycle: LifecycleOp,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Lifecycle(lifecycle))
}

pub(crate) fn plan_artifact_render(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    dry_run: bool,
) -> CommandPlan {
    artifact(artifact_type, id, Op::RenderArtifact { dry_run })
}

pub(crate) fn plan_delete(
    artifact_type: cmd::edit::ArtifactType,
    id: &str,
    force: bool,
) -> CommandPlan {
    artifact(artifact_type, id, Op::Delete { force })
}

impl CommandPlan {
    pub fn from_parsed(cmd: &Commands, global_dry_run: bool) -> anyhow::Result<Self> {
        use crate::resource_plan::ToPlan;

        match cmd {
            Commands::Init { force } => Ok(global(Op::Builtin(BuiltinOp::Init { force: *force }))),
            Commands::InitSkills { force, format, dir } => {
                Ok(global(Op::Builtin(BuiltinOp::InitSkills {
                    force: *force,
                    format: format.clone(),
                    dir: dir.clone(),
                })))
            }
            Commands::Check { has_active, .. } => Ok(global(Op::Builtin(BuiltinOp::Check {
                has_active: *has_active,
            }))),
            Commands::Status => Ok(global(Op::Builtin(BuiltinOp::Status))),
            Commands::Render {
                target,
                dry_run,
                force,
            } => Ok(global(Op::Builtin(BuiltinOp::RenderGlobal {
                target: *target,
                dry_run: global_dry_run || *dry_run,
                force: *force,
            }))),
            Commands::Migrate => Ok(global(Op::Builtin(BuiltinOp::Migrate))),
            Commands::Verify { guard_ids, work } => Ok(global(Op::Builtin(BuiltinOp::Verify {
                guard_ids: guard_ids.clone(),
                work: work.clone(),
            }))),
            Commands::Describe { context, .. } => Ok(global(Op::Builtin(BuiltinOp::Describe {
                context: *context,
            }))),
            Commands::Completions { shell } => Ok(global(Op::Builtin(BuiltinOp::Completions {
                shell: *shell,
            }))),
            Commands::SelfUpdate { check } => {
                Ok(global(Op::Builtin(BuiltinOp::SelfUpdate { check: *check })))
            }
            #[cfg(feature = "tui")]
            Commands::Tui => Ok(global(Op::Builtin(BuiltinOp::Tui))),
            Commands::Rfc { command } => command.to_plan(),
            Commands::Clause { command } => command.to_plan(),
            Commands::Adr { command } => command.to_plan(),
            Commands::Work { command } => command.to_plan(),
            Commands::Guard { command } => command.to_plan(),
            Commands::Loop { command } => match command {
                crate::LoopCommand::Start { id, work_items } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopStart {
                        loop_id: id.clone(),
                        work_items: work_items.clone(),
                    })))
                }
                crate::LoopCommand::Show { id } => Ok(global(Op::Builtin(BuiltinOp::LoopShow {
                    loop_id: id.clone(),
                }))),
                crate::LoopCommand::Resume { id, work_items } => {
                    Ok(global(Op::Builtin(BuiltinOp::LoopResume {
                        loop_id: id.clone(),
                        work_items: work_items.clone(),
                    })))
                }
                crate::LoopCommand::Run {
                    id,
                    work_items,
                    max_rounds,
                } => Ok(global(Op::Builtin(BuiltinOp::LoopRun {
                    loop_id: id.clone(),
                    work_items: work_items.clone(),
                    max_rounds: *max_rounds,
                }))),
            },
            Commands::Release { version, date } => Ok(global(Op::Builtin(BuiltinOp::ReleaseCut {
                version: version.clone(),
                date: date.clone(),
            }))),
            Commands::Tag { command } => match command {
                crate::TagCommand::New { tag } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagNew { tag: tag.clone() })))
                }
                crate::TagCommand::Delete { tag } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagDelete {
                        tag: tag.clone(),
                    })))
                }
                crate::TagCommand::List { output } => {
                    Ok(global(Op::Builtin(BuiltinOp::TagList { output: *output })))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::DiagnosticCode;
    use crate::resource_plan::ToPlan;
    use crate::{ClauseCommand, EditActionArgs, TickStatus, WorkTickStatus};
    use clap::{Parser, error::ErrorKind};

    #[test]
    fn test_from_clause_command_uses_canonical_edit_when_path_is_present()
    -> Result<(), Box<dyn std::error::Error>> {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: Some("text".to_string()),
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        };

        let plan = cmd.to_plan()?;
        assert!(matches!(
            plan.scope,
            Scope::Target {
                artifact: cmd::edit::ArtifactType::Clause,
                ..
            }
        ));
        match plan.op {
            Op::Edit(EditOp::Field { action, .. }) => match action {
                OwnedEditAction::Set { value, stdin } => {
                    assert_eq!(value.as_ref(), Some(&Some("Updated".to_string())));
                    assert!(!stdin);
                }
                other => return Err(format!("expected set action, got {other:?}").into()),
            },
            other => return Err(format!("expected field edit, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn test_from_clause_command_requires_path_for_canonical_flags()
    -> Result<(), Box<dyn std::error::Error>> {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: None,
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        };

        let result = cmd.to_plan();
        assert!(result.is_err(), "missing path should fail");
        let err = result.err().ok_or("expected Err")?;
        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::E0801MissingRequiredArg);
        Ok(())
    }

    #[test]
    fn test_from_clause_command_uses_legacy_edit_without_canonical_flags()
    -> Result<(), Box<dyn std::error::Error>> {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: None,
            set: None,
            add: None,
            remove: None,
            tick: None,
            stdin: true,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        };

        let plan = cmd.to_plan()?;
        assert!(matches!(
            plan.scope,
            Scope::Artifact {
                artifact: cmd::edit::ArtifactType::Clause,
                ..
            }
        ));
        match plan.op {
            Op::Edit(EditOp::ClauseLegacy {
                text,
                text_file,
                stdin,
            }) => {
                assert!(text.is_none());
                assert!(text_file.is_none());
                assert!(stdin);
            }
            other => return Err(format!("expected legacy clause edit, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn test_edit_plans_are_mutating() -> Result<(), Box<dyn std::error::Error>> {
        let plan = crate::RfcCommand::Edit(crate::CommonEditArgs {
            id: "RFC-0001".to_string(),
            path: "title".to_string(),
            action: EditActionArgs {
                set: Some(Some("X".to_string())),
                add: None,
                remove: None,
                tick: None,
                stdin: false,
                at: None,
                exact: false,
                regex: false,
                all: false,
            },
        })
        .to_plan()?;
        assert!(matches!(plan.scope, Scope::Target { .. }));
        assert!(matches!(plan.op, Op::Edit(EditOp::Field { .. })));
        assert_eq!(plan.lock_disposition(), LockDisposition::GovRootExclusive);

        let plan = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: None,
            set: None,
            add: None,
            remove: None,
            tick: None,
            stdin: true,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: None,
            text_file: None,
        }
        .to_plan()?;
        assert!(matches!(plan.op, Op::Edit(EditOp::ClauseLegacy { .. })));
        assert_eq!(plan.lock_disposition(), LockDisposition::GovRootExclusive);
        Ok(())
    }

    #[test]
    fn test_read_plans_are_lock_free() -> Result<(), Box<dyn std::error::Error>> {
        let status = global(Op::Builtin(BuiltinOp::Status));
        assert_eq!(status.lock_disposition(), LockDisposition::None);

        let plan = plan_get("RFC-0001", Some("title"))?;
        assert!(matches!(plan.scope, Scope::Target { .. }));
        assert!(matches!(plan.op, Op::Get));
        assert_eq!(plan.lock_disposition(), LockDisposition::None);
        Ok(())
    }

    #[test]
    fn test_lock_disposition_is_lock_free_for_inspect_commands()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            global(Op::Builtin(BuiltinOp::Status)).lock_disposition(),
            LockDisposition::None
        );
        assert_eq!(
            plan_get("RFC-0001", Some("title"))?.lock_disposition(),
            LockDisposition::None
        );
        assert_eq!(
            plan_show(
                cmd::edit::ArtifactType::Adr,
                "ADR-0038",
                OutputFormat::Table
            )
            .lock_disposition(),
            LockDisposition::None
        );
        Ok(())
    }

    #[test]
    fn test_lock_disposition_requires_lock_for_mutating_commands()
    -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            global(Op::Builtin(BuiltinOp::Init { force: false })).lock_disposition(),
            LockDisposition::GovRootExclusive
        );
        assert_eq!(
            plan_edit(
                "WI-2026-04-07-004",
                "acceptance_criteria[0]",
                tick_action(OwnedMatchOptions::default(), TickStatus::Done),
                EditExtras::default(),
            )?
            .lock_disposition(),
            LockDisposition::GovRootExclusive
        );
        assert_eq!(
            plan_lifecycle(
                cmd::edit::ArtifactType::WorkItem,
                "WI-2026-04-07-004",
                LifecycleOp::MoveWork {
                    file_or_id: std::path::PathBuf::from("WI-2026-04-07-004"),
                    status: WorkItemStatus::Done,
                },
            )
            .lock_disposition(),
            LockDisposition::GovRootExclusive
        );
        Ok(())
    }

    #[test]
    fn test_self_update_routes_to_builtin_op() -> Result<(), Box<dyn std::error::Error>> {
        let check_plan = CommandPlan::from_parsed(&Commands::SelfUpdate { check: true }, false)?;
        assert!(matches!(check_plan.scope, Scope::Global));
        assert!(matches!(
            check_plan.op,
            Op::Builtin(BuiltinOp::SelfUpdate { check: true })
        ));

        let update_plan = CommandPlan::from_parsed(&Commands::SelfUpdate { check: false }, false)?;
        assert!(matches!(update_plan.scope, Scope::Global));
        assert!(matches!(
            update_plan.op,
            Op::Builtin(BuiltinOp::SelfUpdate { check: false })
        ));
        Ok(())
    }

    #[test]
    fn test_self_update_is_lock_free() {
        // Self-update replaces the binary, not governance files — no gov-root lock needed.
        assert_eq!(
            global(Op::Builtin(BuiltinOp::SelfUpdate { check: true })).lock_disposition(),
            LockDisposition::None
        );
        assert_eq!(
            global(Op::Builtin(BuiltinOp::SelfUpdate { check: false })).lock_disposition(),
            LockDisposition::None
        );
    }

    #[test]
    fn test_target_resolves_get_and_edit_to_same_field_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let get = crate::AdrCommand::Get(crate::CommonGetArgs {
            id: "ADR-0038".to_string(),
            field: Some("alternatives[1].status".to_string()),
        })
        .to_plan()?;
        let edit = crate::AdrCommand::Edit(crate::AdrEditArgs {
            common: crate::CommonEditArgs {
                id: "ADR-0038".to_string(),
                path: "alternatives[1].status".to_string(),
                action: EditActionArgs {
                    set: None,
                    add: None,
                    remove: None,
                    tick: Some(TickStatus::Accepted),
                    stdin: false,
                    at: None,
                    exact: false,
                    regex: false,
                    all: false,
                },
            },
            pro: vec![],
            con: vec![],
            reject_reason: None,
        })
        .to_plan()?;

        match ((&get.op, &get.scope), (&edit.op, &edit.scope)) {
            (
                (
                    Op::Get,
                    Scope::Target {
                        artifact: get_artifact,
                        id: get_id,
                        target: get_target,
                    },
                ),
                (
                    Op::Edit(EditOp::Field { .. }),
                    Scope::Target {
                        artifact: edit_artifact,
                        id: edit_id,
                        target: edit_target,
                    },
                ),
            ) => {
                assert_eq!(get_artifact, edit_artifact);
                assert_eq!(get_id, edit_id);
                assert_eq!(get_target, edit_target);
            }
            other => return Err(format!("expected field targets, got {other:?}").into()),
        }
        Ok(())
    }

    #[test]
    fn test_from_clause_command_rejects_mixed_canonical_and_legacy_edit_flags()
    -> Result<(), Box<dyn std::error::Error>> {
        let cmd = ClauseCommand::Edit {
            id: "RFC-0001:C-TEST".to_string(),
            path: Some("text".to_string()),
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            at: None,
            exact: false,
            regex: false,
            all: false,
            text: Some("legacy".to_string()),
            text_file: None,
        };

        let result = cmd.to_plan();
        assert!(result.is_err(), "mixed modes should fail");
        let err = result.err().ok_or("expected Err")?;
        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
        Ok(())
    }

    #[test]
    fn test_work_tick_defaults_status_to_done() -> Result<(), Box<dyn std::error::Error>> {
        let cli = crate::Cli::parse_from([
            "govctl",
            "work",
            "tick",
            "WI-2026-04-07-001",
            "acceptance_criteria",
            "Criterion 1",
        ]);

        match cli.command {
            crate::Commands::Work {
                command: crate::WorkCommand::Tick(crate::WorkTickArgs { status, .. }),
            } => assert!(matches!(status, WorkTickStatus::Done)),
            _ => return Err("expected work tick command".into()),
        }
        Ok(())
    }

    #[test]
    fn test_rfc_get_help_restores_resource_specific_examples() {
        let err = match crate::Cli::try_parse_from(["govctl", "rfc", "get", "--help"]) {
            Ok(_) => unreachable!("help should exit"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let help = err.to_string();
        assert!(help.contains("VALID FIELDS:"), "help: {help}");
        assert!(
            help.contains("govctl rfc get RFC-0001 title"),
            "help: {help}"
        );
    }

    #[test]
    fn test_work_get_help_restores_resource_specific_examples() {
        let err = match crate::Cli::try_parse_from(["govctl", "work", "get", "--help"]) {
            Ok(_) => unreachable!("help should exit"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let help = err.to_string();
        assert!(help.contains("VALID FIELDS:"), "help: {help}");
        assert!(
            help.contains("acceptance_criteria[0].status"),
            "help: {help}"
        );
    }
}
