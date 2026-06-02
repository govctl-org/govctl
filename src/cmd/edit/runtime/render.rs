use super::spec::{RenderMode, SimpleFieldSpec};
use super::support::{
    joined_scalar_list_text, status_list_entry_line, type_mismatch, value_at_path,
};
use crate::diagnostic::DiagnosticResult;
use serde_json::Value;

pub(super) fn render_field(
    doc: &Value,
    spec: SimpleFieldSpec,
    id: &str,
) -> DiagnosticResult<String> {
    let v = value_at_path(doc, spec.path);
    match spec.render {
        RenderMode::Scalar => Ok(render_scalar(v)),
        RenderMode::CsvStrings => render_string_array(v, ", ", id),
        RenderMode::LineStrings => render_string_array(v, "\n", id),
        RenderMode::StatusLines {
            status_key,
            text_key,
        } => render_status_lines(v, status_key, text_key, id),
    }
}

pub(super) fn render_scalar(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return String::new();
    };
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => v.to_string(),
    }
}

fn render_string_array(v: Option<&Value>, sep: &str, id: &str) -> DiagnosticResult<String> {
    let Some(v) = v else {
        return Ok(String::new());
    };
    let Some(items) = v.as_array() else {
        return Err(type_mismatch("Expected an array value", id));
    };

    Ok(joined_scalar_list_text(items, sep))
}

pub(super) fn render_status_lines(
    v: Option<&Value>,
    status_key: &str,
    text_key: &str,
    id: &str,
) -> DiagnosticResult<String> {
    let Some(v) = v else {
        return Ok(String::new());
    };
    let Some(items) = v.as_array() else {
        return Err(type_mismatch("Expected an array value", id));
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(status_list_entry_line(item, status_key, text_key, id)?);
    }
    Ok(out.join("\n"))
}
