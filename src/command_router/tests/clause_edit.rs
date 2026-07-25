use super::*;

#[test]
fn test_from_clause_command_uses_canonical_edit_when_path_is_present()
-> Result<(), Box<dyn std::error::Error>> {
    let cmd = ClauseCommand::Edit(CommonEditArgs {
        id: "RFC-0001:C-TEST".to_string(),
        path: "text".to_string(),
        action: EditActionArgs {
            set: Some(Some("Updated".to_string())),
            add: None,
            remove: None,
            tick: None,
            stdin: false,
            regex: false,
            all: false,
        },
    });

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
