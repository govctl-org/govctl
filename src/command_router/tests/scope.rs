use super::*;

/// [[RFC-0010:C-COMMAND-SCOPE]]: exactly release and migrate are trunk-scoped.
#[test]
fn test_release_and_migrate_are_trunk_scoped() {
    assert_eq!(
        global(Op::Builtin(BuiltinOp::ReleaseCut {
            version: "0.2.0".to_string(),
            date: None,
        }))
        .command_scope(),
        CommandScope::Trunk
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::ReleaseUndo {
            expected_version: "0.2.0".to_string(),
        }))
        .command_scope(),
        CommandScope::Trunk
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Migrate)).command_scope(),
        CommandScope::Trunk
    );
}

/// Mutating governed-artifact commands are branch-content scoped.
#[test]
fn test_mutating_commands_are_branch_content_scoped() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Init { force: false })).command_scope(),
        CommandScope::BranchContent
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::RenderGlobal {
            target: crate::RenderTarget::All,
            dry_run: false,
            force: false,
        }))
        .command_scope(),
        CommandScope::BranchContent
    );
    assert_eq!(
        plan_edit(
            "WI-2026-09-08-001",
            "acceptance_criteria[0]",
            OwnedEditAction::Tick {
                match_opts: OwnedMatchOptions::default(),
                status: TickStatus::Done,
            },
        )?
        .command_scope(),
        CommandScope::BranchContent
    );
    assert_eq!(
        plan_lifecycle(
            cmd::edit::ArtifactType::WorkItem,
            "WI-2026-09-08-001",
            LifecycleOp::MoveWork {
                file_or_id: std::path::PathBuf::from("WI-2026-09-08-001"),
                status: WorkItemStatus::Done,
            },
        )
        .command_scope(),
        CommandScope::BranchContent
    );
    Ok(())
}

/// Read-only and project-independent commands are workspace scoped.
#[test]
fn test_read_only_commands_are_workspace_scoped() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Status)).command_scope(),
        CommandScope::Workspace
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Check)).command_scope(),
        CommandScope::Workspace
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Describe { context: false })).command_scope(),
        CommandScope::Workspace
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::Completions {
            shell: clap_complete::Shell::Bash,
        }))
        .command_scope(),
        CommandScope::Workspace
    );
    assert_eq!(
        global(Op::Builtin(BuiltinOp::SelfUpdate { check: true })).command_scope(),
        CommandScope::Workspace
    );
    assert_eq!(
        plan_show(
            cmd::edit::ArtifactType::Rfc,
            "RFC-0010",
            crate::ShowOutputFormat::Table,
            false,
        )
        .command_scope(),
        CommandScope::Workspace
    );
    assert_eq!(
        plan_get("RFC-0010", Some("title"), None)?.command_scope(),
        CommandScope::Workspace
    );
    Ok(())
}

/// [[RFC-0010:C-COMMAND-SCOPE]]: loop execution and `claim list` are
/// workspace scoped; `claim release` and `claim steal` are branch-content
/// scoped but never acquire the gov-root write lock.
#[test]
fn test_loop_and_claim_commands_match_command_scope_definitions() {
    let loop_ops = [
        BuiltinOp::LoopStart {
            loop_id: None,
            work_ids: vec!["WI-2026-09-08-001".to_string()],
        },
        BuiltinOp::LoopList {
            filter: None,
            limit: None,
            output: crate::OutputFormat::Table,
        },
        BuiltinOp::LoopShow {
            loop_id: "LOOP-2026-09-08-001".to_string(),
        },
        BuiltinOp::LoopResume {
            loop_id: "LOOP-2026-09-08-001".to_string(),
        },
        BuiltinOp::LoopReplan {
            loop_id: "LOOP-2026-09-08-001".to_string(),
        },
        BuiltinOp::LoopAdd {
            loop_id: "LOOP-2026-09-08-001".to_string(),
            field: "work".to_string(),
            value: "WI-2026-09-08-001".to_string(),
        },
        BuiltinOp::LoopRemove {
            loop_id: "LOOP-2026-09-08-001".to_string(),
            field: "work".to_string(),
            value: "WI-2026-09-08-001".to_string(),
        },
        BuiltinOp::LoopRun {
            loop_id: "LOOP-2026-09-08-001".to_string(),
            target_work_ids: vec![],
        },
    ];
    for op in loop_ops {
        assert_eq!(
            global(Op::Builtin(op)).command_scope(),
            CommandScope::Workspace,
            "loop execution must be workspace scoped"
        );
    }
    assert_eq!(
        global(Op::Builtin(BuiltinOp::ClaimList)).command_scope(),
        CommandScope::Workspace
    );
    for op in [
        BuiltinOp::ClaimRelease {
            id: "RFC-0001".to_string(),
        },
        BuiltinOp::ClaimSteal {
            id: "RFC-0001".to_string(),
        },
    ] {
        let plan = global(Op::Builtin(op));
        assert_eq!(
            plan.command_scope(),
            CommandScope::BranchContent,
            "claim mutation is branch-content scoped"
        );
        assert_eq!(
            plan.lock_disposition(),
            LockDisposition::None,
            "claim commands must not acquire the gov-root write lock"
        );
    }
}
