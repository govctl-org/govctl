use crate::GetOutputFormat;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use comfy_table::{Attribute, Cell, ContentArrangement, Table, presets::UTF8_FULL};
use serde::Serialize;
use std::fmt::Display;
use std::io::IsTerminal;

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

pub(crate) fn print_get_output(
    value: &serde_json::Value,
    field_selected: bool,
    output: Option<GetOutputFormat>,
    error_code: DiagnosticCode,
    scope: &str,
) -> DiagnosticResult<()> {
    let output = output.unwrap_or_else(|| {
        if field_selected {
            GetOutputFormat::Plain
        } else if std::io::stdout().is_terminal() {
            GetOutputFormat::Table
        } else {
            GetOutputFormat::Json
        }
    });
    if field_selected && matches!(output, GetOutputFormat::Table | GetOutputFormat::Toml) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            "Field retrieval supports only plain, json, or yaml output",
            scope,
        ));
    }
    if !field_selected && output == GetOutputFormat::Plain {
        return Err(Diagnostic::new(
            DiagnosticCode::E0820InvalidFieldValue,
            "Complete retrieval supports only table, json, yaml, or toml output",
            scope,
        ));
    }
    match output {
        GetOutputFormat::Table => print_value_table(value),
        GetOutputFormat::Json => print_json(
            value,
            error_code,
            "Failed to serialize get output as JSON",
            scope,
        )?,
        GetOutputFormat::Yaml => print_yaml(
            value,
            error_code,
            "Failed to serialize get output as YAML",
            scope,
        )?,
        GetOutputFormat::Toml => print_toml(
            value,
            error_code,
            "Failed to serialize get output as TOML",
            scope,
        )?,
        GetOutputFormat::Plain => print_plain_value(value),
    }
    Ok(())
}

fn print_value_table(value: &serde_json::Value) {
    let mut table = table_with_bold_headers(&["Field", "Value"]);
    if let Some(object) = value.as_object() {
        for (field, value) in object {
            table.add_row([field.clone(), human_value(value)]);
        }
    } else {
        table.add_row(["value".to_string(), human_value(value)]);
    }
    println!("{table}");
}

fn print_plain_value(value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => {}
        serde_json::Value::Array(items) => {
            for item in items {
                println!("{}", human_value(item));
            }
        }
        _ => print!("{}", human_value(value)),
    }
}

fn human_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::Array(values) => values
            .iter()
            .map(human_value)
            .collect::<Vec<_>>()
            .join("\n"),
        serde_json::Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string())
        }
    }
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

pub(crate) fn print_yaml<T: Serialize>(
    value: &T,
    error_code: DiagnosticCode,
    error_message: &str,
    scope: impl Into<String>,
) -> DiagnosticResult<()> {
    print_serialized(
        value,
        serde_yaml::to_string,
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
