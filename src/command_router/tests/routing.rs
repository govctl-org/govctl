use super::*;

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
fn test_loop_commands_route_to_builtin_ops() -> Result<(), Box<dyn std::error::Error>> {
    let list_plan = CommandPlan::from_parsed(
        &Commands::Loop {
            command: crate::LoopCommand::List {
                filter: Some("open".to_string()),
                limit: Some(3),
                output: crate::OutputFormat::Json,
            },
        },
        false,
    )?;
    assert!(matches!(list_plan.scope, Scope::Global));
    assert!(matches!(
        list_plan.op,
        Op::Builtin(BuiltinOp::LoopList {
            filter: Some(ref filter),
            limit: Some(3),
            output: crate::OutputFormat::Json,
        }) if filter == "open"
    ));

    let run_plan = CommandPlan::from_parsed(
        &Commands::Loop {
            command: crate::LoopCommand::Run {
                id: "LOOP-2026-06-02-001".to_string(),
                target_work_ids: vec!["WI-2026-06-02-058".to_string()],
            },
        },
        false,
    )?;
    assert!(matches!(
        run_plan.op,
        Op::Builtin(BuiltinOp::LoopRun {
            loop_id: ref id,
            target_work_ids: ref work,
        }) if id == "LOOP-2026-06-02-001" && work == &vec!["WI-2026-06-02-058".to_string()]
    ));

    Ok(())
}

#[test]
fn test_tag_commands_route_to_builtin_ops() -> Result<(), Box<dyn std::error::Error>> {
    let new_plan = CommandPlan::from_parsed(
        &Commands::Tag {
            command: crate::TagCommand::New {
                tag: "cleanup".to_string(),
            },
        },
        false,
    )?;
    assert!(matches!(new_plan.scope, Scope::Global));
    assert!(matches!(
        new_plan.op,
        Op::Builtin(BuiltinOp::TagNew { ref tag }) if tag == "cleanup"
    ));

    let list_plan = CommandPlan::from_parsed(
        &Commands::Tag {
            command: crate::TagCommand::List {
                output: crate::OutputFormat::Plain,
            },
        },
        false,
    )?;
    assert!(matches!(
        list_plan.op,
        Op::Builtin(BuiltinOp::TagList {
            output: crate::OutputFormat::Plain,
        })
    ));

    Ok(())
}

#[test]
fn test_artifact_render_rejects_unsupported_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    for (artifact, id) in [
        (cmd::edit::ArtifactType::Clause, "RFC-0001:C-SCOPE"),
        (cmd::edit::ArtifactType::Guard, "GUARD-CHECK"),
    ] {
        let plan = plan_artifact_render(artifact, id, false);
        let err = match plan.execute(
            &crate::config::Config::default(),
            crate::write::WriteOp::Execute,
        ) {
            Ok(_) => return Err("unsupported artifact render should fail".into()),
            Err(err) => err,
        };

        assert_eq!(err.code, DiagnosticCode::E0822UnsupportedOperation);
        assert_eq!(err.message, "render is not supported for this artifact");
        assert_eq!(err.file, id);
    }
    Ok(())
}
