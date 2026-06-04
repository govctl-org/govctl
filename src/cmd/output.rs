use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;
use std::fmt::Display;

pub(crate) fn command_table() -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
}

pub(crate) fn table_with_bold_headers(headers: &[&str]) -> Table {
    let mut table = command_table();
    table.set_header(
        headers
            .iter()
            .map(|header| Cell::new(*header).add_attribute(Attribute::Bold))
            .collect::<Vec<_>>(),
    );
    table
}

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
