use super::{OwnedEditAction, OwnedMatchOptions};
use crate::EditActionArgs;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

fn conflicting_edit_flag_error(action: &str, flag: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0802ConflictingArgs,
        format!("Cannot use {flag} with --{action}"),
        "edit action",
    )
}

fn reject_selector_flags_for_value_action(
    action: &str,
    args: &EditActionArgs,
) -> DiagnosticResult<()> {
    if args.regex {
        return Err(conflicting_edit_flag_error(action, "--regex"));
    }
    if args.all {
        return Err(conflicting_edit_flag_error(action, "--all"));
    }
    Ok(())
}

pub(crate) fn owned_edit_action(args: &EditActionArgs) -> DiagnosticResult<OwnedEditAction> {
    let action_count = usize::from(args.set.is_some())
        + usize::from(args.add.is_some())
        + usize::from(args.tick.is_some())
        + usize::from(args.remove.is_some());

    if action_count == 0 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "exactly one edit action is required",
            "edit action",
        ));
    }

    if action_count > 1 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use multiple edit actions at once",
            "edit action",
        ));
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
            ));
        }
        if args.regex {
            return Err(conflicting_edit_flag_error("tick", "--regex"));
        }
        return Ok(OwnedEditAction::Tick {
            match_opts: OwnedMatchOptions {
                pattern: None,
                at: None,
                exact: false,
                regex: false,
                all: false,
            },
            status,
        });
    }
    if let Some(remove) = &args.remove {
        if args.stdin {
            return Err(conflicting_edit_flag_error("remove", "--stdin"));
        }
        if args.all && remove.is_some() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0802ConflictingArgs,
                "Cannot combine --all with a remove value",
                "edit action",
            ));
        }
        if args.all && args.regex {
            return Err(Diagnostic::new(
                DiagnosticCode::E0802ConflictingArgs,
                "Cannot combine --all with --regex",
                "edit action",
            ));
        }
        if args.regex && remove.is_none() {
            return Err(Diagnostic::new(
                DiagnosticCode::E0801MissingRequiredArg,
                "--regex requires a remove pattern",
                "edit action",
            ));
        }
        return Ok(OwnedEditAction::Remove {
            match_opts: OwnedMatchOptions {
                pattern: remove.clone(),
                at: None,
                exact: !args.regex,
                regex: args.regex,
                all: args.all,
            },
        });
    }
    unreachable!("action_count guarantees exactly one action branch")
}
