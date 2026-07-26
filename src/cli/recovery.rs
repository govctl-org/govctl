use super::{Cli, Commands, RfcCommand};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use std::ffi::{OsStr, OsString};

pub(crate) fn misrouted_nested_clause(args: &[OsString]) -> Option<Diagnostic> {
    let tokens = utf8_args(args)?;
    let rfc_index = root_command_index(&tokens)?;
    if tokens.get(rfc_index)? != "rfc" {
        return None;
    }

    let nested_index = next_command_index(&tokens, rfc_index + 1)?;
    if tokens.get(nested_index)? != "clause" {
        return None;
    }

    let suggestion = display_command(&tokens, Some(rfc_index), None);
    Some(namespace_diagnostic(
        "The nested `govctl rfc clause ...` form is invalid.",
        &suggestion,
    ))
}

pub(crate) fn misrouted_clause_after_parse(cli: &Cli, args: &[OsString]) -> Option<Diagnostic> {
    let Commands::Rfc { command } = &cli.command else {
        return None;
    };

    if let Some(id) = command.clause_id_in_rfc_position() {
        let message = if command.has_shared_clause_verb() {
            let tokens = utf8_args(args)?;
            let rfc_index = root_command_index(&tokens)?;
            let suggestion = display_command(&tokens, None, Some(rfc_index));
            format!(
                "{id} identifies a Clause, not an RFC. Use the canonical Clause command: `{suggestion}`"
            )
        } else {
            format!(
                "{id} identifies a Clause, not an RFC. Use the `govctl clause` namespace for Clause operations."
            )
        };
        return Some(Diagnostic::new(
            DiagnosticCode::E0821InvalidCommandScope,
            message,
            id,
        ));
    }

    let RfcCommand::Edit(edit) = command else {
        return None;
    };
    if !is_clause_owned_path(&edit.path) {
        return None;
    }

    Some(Diagnostic::new(
        DiagnosticCode::E0821InvalidCommandScope,
        "Clause content is not an RFC edit path. Use the `govctl clause edit <RFC-ID:C-NAME> ...` namespace; the rejected path does not identify a Clause.",
        &edit.path,
    ))
}

impl RfcCommand {
    fn clause_id_in_rfc_position(&self) -> Option<&str> {
        let id = match self {
            Self::Get(args) => &args.id,
            Self::Show(args) => &args.id,
            Self::Edit(args) => &args.id,
            Self::Bump { id, .. } | Self::Finalize { id, .. } | Self::Advance { id, .. } => id,
            Self::Deprecate(args) => &args.id,
            Self::Supersede(args) => {
                if is_clause_reference(&args.id) {
                    &args.id
                } else {
                    &args.by
                }
            }
            Self::Render(args) => &args.id,
            Self::New { id: Some(id), .. } => id,
            Self::List(_) | Self::New { id: None, .. } => return None,
        };
        is_clause_reference(id).then_some(id.as_str())
    }

    fn has_shared_clause_verb(&self) -> bool {
        matches!(self, Self::Get(_) | Self::Show(_) | Self::Edit(_))
    }
}

fn namespace_diagnostic(prefix: &str, suggestion: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0821InvalidCommandScope,
        format!("{prefix} Use the canonical Clause command: `{suggestion}`"),
        "rfc command",
    )
}

fn is_clause_reference(id: &str) -> bool {
    crate::load::split_clause_id(id).is_some()
}

fn is_clause_owned_path(path: &str) -> bool {
    matches!(path.split(['.', '[']).next(), Some("clause" | "clauses"))
}

fn utf8_args(args: &[OsString]) -> Option<Vec<String>> {
    args.iter()
        .map(|arg| arg.to_str().map(str::to_owned))
        .collect()
}

fn root_command_index(tokens: &[String]) -> Option<usize> {
    next_command_index(tokens, 1)
}

fn next_command_index(tokens: &[String], mut index: usize) -> Option<usize> {
    while index < tokens.len() {
        match tokens[index].as_str() {
            "-C" | "--config" => index += 2,
            "--dry-run" => index += 1,
            value if value.starts_with("--config=") || value.starts_with("-C") => index += 1,
            _ => return Some(index),
        }
    }
    None
}

fn display_command(
    tokens: &[String],
    remove_index: Option<usize>,
    replace_index: Option<usize>,
) -> String {
    let mut rendered = vec!["govctl".to_string()];
    for (index, token) in tokens.iter().enumerate().skip(1) {
        if Some(index) == remove_index {
            continue;
        }
        if Some(index) == replace_index {
            rendered.push("clause".to_string());
        } else {
            rendered.push(display_arg(OsStr::new(token)));
        }
    }
    rendered.join(" ")
}

fn display_arg(arg: &OsStr) -> String {
    let value = arg.to_string_lossy();
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "-_./:[]=,@".contains(ch))
    {
        value.into_owned()
    } else {
        format!("{value:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_clause_references_only() {
        assert!(is_clause_reference("RFC-0001:C-ONE"));
        assert!(!is_clause_reference("RFC-0001"));
        assert!(!is_clause_reference("ADR-0001:C-ONE"));
        assert!(!is_clause_reference("RFC-X:C-ONE"));
        assert!(!is_clause_reference("RFC-0001:C-"));
    }

    #[test]
    fn identifies_only_clause_owned_rfc_paths() {
        assert!(is_clause_owned_path("clauses"));
        assert!(is_clause_owned_path("clauses[0].text"));
        assert!(is_clause_owned_path("clause.text"));
        assert!(!is_clause_owned_path("changelog.changed"));
    }

    #[test]
    fn nested_recovery_removes_only_rfc_resource_token() -> Result<(), Box<dyn std::error::Error>> {
        let args = [
            "govctl",
            "--config",
            "gov/config.toml",
            "rfc",
            "clause",
            "show",
            "RFC-0001:C-ONE",
        ]
        .map(OsString::from);
        let diagnostic = misrouted_nested_clause(&args)
            .ok_or_else(|| std::io::Error::other("expected nested clause recovery"))?;
        assert!(
            diagnostic
                .message
                .contains("govctl --config gov/config.toml clause show RFC-0001:C-ONE")
        );
        Ok(())
    }
}
