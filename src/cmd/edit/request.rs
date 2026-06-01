use super::matching::MatchOptionsOwned;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::ChangelogCategory;
use crate::write::WriteOp;
use std::io::Read;

#[derive(Debug, Clone)]
pub enum OwnedEditAction {
    Set {
        value: Option<Option<String>>,
        stdin: bool,
    },
    Add {
        value: Option<Option<String>>,
        stdin: bool,
    },
    Remove {
        match_opts: MatchOptionsOwned,
    },
    Tick {
        match_opts: MatchOptionsOwned,
        status: crate::TickStatus,
    },
}

pub struct EditFieldRequest<'a> {
    pub config: &'a Config,
    pub id: &'a str,
    pub path: &'a str,
    pub action: &'a OwnedEditAction,
    pub category_override: Option<ChangelogCategory>,
    pub pros: Option<Vec<String>>,
    pub cons: Option<Vec<String>>,
    pub reject_reason: Option<String>,
    pub op: WriteOp,
}

pub(super) fn read_stdin() -> DiagnosticResult<String> {
    let mut buffer = String::new();
    std::io::stdin()
        .read_to_string(&mut buffer)
        .map_err(|err| Diagnostic::io_error("read from stdin", err, "stdin"))?;
    Ok(buffer.trim_end_matches('\n').to_string())
}

pub(super) fn resolve_owned_value(
    value: Option<&Option<String>>,
    stdin: bool,
) -> DiagnosticResult<String> {
    match (value, stdin) {
        (Some(Some(v)), false) => Ok(v.clone()),
        (Some(None), true) => read_stdin(),
        (Some(None), false) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "input",
        )),
        (Some(Some(_)), true) => Err(Diagnostic::new(
            DiagnosticCode::E0802ConflictingArgs,
            "Cannot use both value and --stdin",
            "input",
        )),
        (None, _) => Err(Diagnostic::new(
            DiagnosticCode::E0801MissingRequiredArg,
            "Provide a value or use --stdin",
            "input",
        )),
    }
}
