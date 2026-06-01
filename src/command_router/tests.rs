use super::*;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::WorkItemStatus;
use crate::resource_plan::ToPlan;
use crate::{ClauseCommand, Commands, EditActionArgs, TickStatus, WorkTickStatus};
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
    // Self-update replaces the binary, not governance files - no gov-root lock needed.
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
fn test_target_resolves_get_and_edit_to_same_field_target() -> Result<(), Box<dyn std::error::Error>>
{
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
