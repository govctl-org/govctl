use super::*;

#[test]
fn test_owned_edit_action_requires_exactly_one_action() -> Result<(), Box<dyn std::error::Error>> {
    let result = owned_edit_action(&EditActionArgs {
        set: None,
        add: None,
        remove: None,
        tick: None,
        stdin: false,
        at: None,
        exact: false,
        regex: false,
        all: false,
    });
    assert!(result.is_err(), "missing action should fail");
    let err = result.err().ok_or("expected Err")?;
    let diag = err
        .downcast_ref::<Diagnostic>()
        .ok_or("expected Diagnostic")?;
    assert_eq!(diag.code, DiagnosticCode::E0801MissingRequiredArg);
    Ok(())
}

#[test]
fn test_owned_edit_action_builds_tick_match_options() -> Result<(), Box<dyn std::error::Error>> {
    let action = owned_edit_action(&EditActionArgs {
        set: None,
        add: None,
        remove: None,
        tick: Some(TickStatus::Done),
        stdin: false,
        at: Some(2),
        exact: true,
        regex: false,
        all: false,
    })?;

    match action {
        OwnedEditAction::Tick { match_opts, status } => {
            assert!(matches!(status, TickStatus::Done));
            assert_eq!(match_opts.at, Some(2));
            assert!(match_opts.exact);
        }
        other => return Err(format!("expected tick action, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_owned_edit_action_rejects_tick_all_combination() -> Result<(), Box<dyn std::error::Error>> {
    let result = owned_edit_action(&EditActionArgs {
        set: None,
        add: None,
        remove: None,
        tick: Some(TickStatus::Done),
        stdin: false,
        at: None,
        exact: false,
        regex: false,
        all: true,
    });
    assert!(result.is_err(), "tick with --all should fail");
    let err = result.err().ok_or("expected Err")?;
    let diag = err
        .downcast_ref::<Diagnostic>()
        .ok_or("expected Diagnostic")?;
    assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    Ok(())
}

#[test]
fn test_owned_edit_action_rejects_multiple_actions() -> Result<(), Box<dyn std::error::Error>> {
    let result = owned_edit_action(&EditActionArgs {
        set: Some(Some("x".to_string())),
        add: Some(Some("y".to_string())),
        remove: None,
        tick: None,
        stdin: false,
        at: None,
        exact: false,
        regex: false,
        all: false,
    });
    assert!(result.is_err(), "multiple actions should fail");
    let err = result.err().ok_or("expected Err")?;
    let diag = err
        .downcast_ref::<Diagnostic>()
        .ok_or("expected Diagnostic")?;
    assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    Ok(())
}

#[test]
fn test_owned_edit_action_preserves_explicit_empty_strings()
-> Result<(), Box<dyn std::error::Error>> {
    let set = owned_edit_action(&EditActionArgs {
        set: Some(Some(String::new())),
        add: None,
        remove: None,
        tick: None,
        stdin: false,
        at: None,
        exact: false,
        regex: false,
        all: false,
    })?;
    match set {
        OwnedEditAction::Set { value, stdin } => {
            assert_eq!(value.as_ref(), Some(&Some(String::new())));
            assert!(!stdin);
        }
        other => return Err(format!("expected set action, got {other:?}").into()),
    }

    let add = owned_edit_action(&EditActionArgs {
        set: None,
        add: Some(Some(String::new())),
        remove: None,
        tick: None,
        stdin: false,
        at: None,
        exact: false,
        regex: false,
        all: false,
    })?;
    match add {
        OwnedEditAction::Add { value, stdin } => {
            assert_eq!(value.as_ref(), Some(&Some(String::new())));
            assert!(!stdin);
        }
        other => return Err(format!("expected add action, got {other:?}").into()),
    }

    let remove = owned_edit_action(&EditActionArgs {
        set: None,
        add: None,
        remove: Some(Some(String::new())),
        tick: None,
        stdin: false,
        at: None,
        exact: false,
        regex: false,
        all: false,
    })?;
    match remove {
        OwnedEditAction::Remove { match_opts } => {
            assert_eq!(match_opts.pattern.as_deref(), Some(""));
        }
        other => return Err(format!("expected remove action, got {other:?}").into()),
    }
    Ok(())
}

#[test]
fn test_owned_edit_action_rejects_selector_flags_for_set() -> Result<(), Box<dyn std::error::Error>>
{
    let result = owned_edit_action(&EditActionArgs {
        set: Some(Some("x".to_string())),
        add: None,
        remove: None,
        tick: None,
        stdin: false,
        at: Some(0),
        exact: false,
        regex: false,
        all: false,
    });
    assert!(result.is_err(), "set with --at should fail");
    let err = result.err().ok_or("expected Err")?;
    let diag = err
        .downcast_ref::<Diagnostic>()
        .ok_or("expected Diagnostic")?;
    assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    Ok(())
}

#[test]
fn test_owned_edit_action_rejects_stdin_for_remove() -> Result<(), Box<dyn std::error::Error>> {
    let result = owned_edit_action(&EditActionArgs {
        set: None,
        add: None,
        remove: Some(None),
        tick: None,
        stdin: true,
        at: Some(0),
        exact: false,
        regex: false,
        all: false,
    });
    assert!(result.is_err(), "remove with --stdin should fail");
    let err = result.err().ok_or("expected Err")?;
    let diag = err
        .downcast_ref::<Diagnostic>()
        .ok_or("expected Diagnostic")?;
    assert_eq!(diag.code, DiagnosticCode::E0802ConflictingArgs);
    Ok(())
}
