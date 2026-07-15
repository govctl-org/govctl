use super::*;

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
        global(Op::Builtin(BuiltinOp::LoopList {
            filter: None,
            limit: None,
            output: OutputFormat::Table,
        }))
        .lock_disposition(),
        LockDisposition::None
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::LoopShow {
            loop_id: "LOOP-2026-04-07-001".to_string(),
        }))
        .lock_disposition(),
        LockDisposition::None
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::LoopResume {
            loop_id: "LOOP-2026-04-07-001".to_string(),
        }))
        .lock_disposition(),
        LockDisposition::None
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::TagList {
            output: OutputFormat::Table,
        }))
        .lock_disposition(),
        LockDisposition::None
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Search {
            query: vec!["cache".to_string()],
            types: vec![],
            tags: vec![],
            limit: None,
            output: OutputFormat::Table,
            reindex: false,
        }))
        .lock_disposition(),
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
        global(Op::Builtin(BuiltinOp::ReleaseUndo {
            expected_version: "0.2.0".to_string(),
        }))
        .lock_disposition(),
        LockDisposition::GovRootExclusive
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Init { force: false })).lock_disposition(),
        LockDisposition::GovRootExclusive
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::LoopRun {
            loop_id: "LOOP-2026-04-07-001".to_string(),
            target_work_ids: vec![],
        }))
        .lock_disposition(),
        LockDisposition::GovRootExclusive
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::LoopReplan {
            loop_id: "LOOP-2026-04-07-001".to_string(),
        }))
        .lock_disposition(),
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
fn test_self_update_is_lock_free() {
    assert_eq!(
        global(Op::Builtin(BuiltinOp::SelfUpdate { check: true })).lock_disposition(),
        LockDisposition::None
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::SelfUpdate { check: false })).lock_disposition(),
        LockDisposition::None
    );
}
