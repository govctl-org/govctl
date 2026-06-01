use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde::Serialize;
use std::fmt::Display;

pub(crate) fn print_json_array<T: Serialize>(items: &[T]) {
    println!(
        "{}",
        serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string())
    );
}

pub(crate) fn print_json<T: Serialize>(
    value: &T,
    error_code: DiagnosticCode,
    error_message: &str,
    scope: impl Into<String>,
) -> DiagnosticResult<()> {
    print_serialized(
        value,
        serde_json::to_string_pretty,
        error_code,
        error_message,
        scope,
    )
}

pub(crate) fn print_toml<T: Serialize>(
    value: &T,
    error_code: DiagnosticCode,
    error_message: &str,
    scope: impl Into<String>,
) -> DiagnosticResult<()> {
    print_serialized(
        value,
        toml::to_string_pretty,
        error_code,
        error_message,
        scope,
    )
}

fn print_serialized<T, E>(
    value: &T,
    serialize: impl FnOnce(&T) -> Result<String, E>,
    error_code: DiagnosticCode,
    error_message: &str,
    scope: impl Into<String>,
) -> DiagnosticResult<()>
where
    T: Serialize,
    E: Display,
{
    let output = serialize(value).map_err(|err| {
        Diagnostic::new(error_code, format!("{error_message}: {err}"), scope.into())
    })?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;
