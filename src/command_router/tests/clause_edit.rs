use super::*;

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
