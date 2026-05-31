use super::{OwnedEditAction, OwnedMatchOptions};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::{EditActionArgs, TickStatus};

fn conflicting_edit_flag_error(action: &str, flag: &str) -> anyhow::Error {
    Diagnostic::new(
        DiagnosticCode::E0802ConflictingArgs,
        format!("Cannot use {flag} with --{action}"),
        "edit action",
    )
    .into()
}

fn reject_selector_flags_for_value_action(
    action: &str,
    args: &EditActionArgs,
) -> anyhow::Result<()> {
    if args.at.is_some() {
        return Err(conflicting_edit_flag_error(action, "--at"));
    }
    if args.exact {
        return Err(conflicting_edit_flag_error(action, "--exact"));
    }
    if args.regex {
        return Err(conflicting_edit_flag_error(action, "--regex"));
    }
    if args.all {
        return Err(conflicting_edit_flag_error(action, "--all"));
    }
    Ok(())
}

pub(crate) fn owned_edit_action(args: &EditActionArgs) -> anyhow::Result<OwnedEditAction> {
    let action_count = usize::from(args.set.is_some())
        + usize::from(args.add.is_some())
        + usize::from(args.tick.is_some())
        + usize::from(args.remove.is_some());

    if action_count == 0 {
        // When --stdin is present with no explicit action, infer --set.
        if args.stdin {
            reject_selector_flags_for_value_action("set (inferred from --stdin)", args)?;
            return Ok(OwnedEditAction::Set {
                value: Some(None),
                stdin: true,
            });
        }
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "exactly one edit action is required",
            "edit action",
        )
        .into());
    }

    if action_count > 1 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use multiple edit actions at once",
            "edit action",
        )
        .into());
    }

    if let Some(value) = &args.set {
        reject_selector_flags_for_value_action("set", args)?;
        return Ok(OwnedEditAction::Set {
            value: Some(value.clone()),
            stdin: args.stdin,
        });
    }
    if let Some(value) = &args.add {
        reject_selector_flags_for_value_action("add", args)?;
        return Ok(OwnedEditAction::Add {
            value: Some(value.clone()),
            stdin: args.stdin,
        });
    }
    if let Some(status) = args.tick {
        if args.stdin {
            return Err(conflicting_edit_flag_error("tick", "--stdin"));
        }
        if args.all {
            return Err(Diagnostic::new(
                DiagnosticCode::E0802ConflictingArgs,
                "Cannot use --all with --tick; tick requires a single target",
                "edit action",
            )
            .into());
        }
        return Ok(OwnedEditAction::Tick {
            match_opts: OwnedMatchOptions {
                pattern: None,
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: args.all,
            },
            status,
        });
    }
    if args.remove.is_some() {
        if args.stdin {
            return Err(conflicting_edit_flag_error("remove", "--stdin"));
        }
        return Ok(OwnedEditAction::Remove {
            match_opts: OwnedMatchOptions {
                pattern: args.remove.clone().flatten(),
                at: args.at,
                exact: args.exact,
                regex: args.regex,
                all: args.all,
            },
        });
    }
    unreachable!("action_count guarantees exactly one action branch")
}

pub(crate) fn set_action(value: Option<String>, stdin: bool) -> OwnedEditAction {
    OwnedEditAction::Set {
        value: Some(value),
        stdin,
    }
}

pub(crate) fn add_action(value: Option<String>, stdin: bool) -> OwnedEditAction {
    OwnedEditAction::Add {
        value: Some(value),
        stdin,
    }
}

pub(crate) fn remove_action(match_opts: OwnedMatchOptions) -> OwnedEditAction {
    OwnedEditAction::Remove { match_opts }
}

pub(crate) fn tick_action(match_opts: OwnedMatchOptions, status: TickStatus) -> OwnedEditAction {
    OwnedEditAction::Tick { match_opts, status }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owned_edit_action_requires_exactly_one_action() -> Result<(), Box<dyn std::error::Error>>
    {
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
    fn test_owned_edit_action_builds_tick_match_options() -> Result<(), Box<dyn std::error::Error>>
    {
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
    fn test_owned_edit_action_rejects_tick_all_combination()
    -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_owned_edit_action_rejects_selector_flags_for_set()
    -> Result<(), Box<dyn std::error::Error>> {
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
}
