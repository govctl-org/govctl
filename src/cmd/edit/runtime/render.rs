use super::{RenderMode, SimpleFieldSpec, type_mismatch, value_at_path};
use serde_json::Value;

pub(super) fn render_field(doc: &Value, spec: SimpleFieldSpec, id: &str) -> anyhow::Result<String> {
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

fn render_string_array(v: Option<&Value>, sep: &str, id: &str) -> anyhow::Result<String> {
    let Some(v) = v else {
        return Ok(String::new());
    };
    let Some(items) = v.as_array() else {
        return Err(type_mismatch("Expected an array value", id).into());
    };

    let rendered: Vec<String> = items
        .iter()
        .map(|item| match item {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => item.to_string(),
        })
        .collect();
    Ok(rendered.join(sep))
}

pub(super) fn render_status_lines(
    v: Option<&Value>,
    status_key: &str,
    text_key: &str,
    id: &str,
) -> anyhow::Result<String> {
    let Some(v) = v else {
        return Ok(String::new());
    };
    let Some(items) = v.as_array() else {
        return Err(type_mismatch("Expected an array value", id).into());
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(obj) = item.as_object() else {
            return Err(type_mismatch("Expected object entries in array", id).into());
        };
        let status = obj
            .get(status_key)
            .and_then(Value::as_str)
            .unwrap_or_default();
        let text = obj
            .get(text_key)
            .and_then(Value::as_str)
            .unwrap_or_default();
        out.push(format!("[{status}] {text}"));
    }
    Ok(out.join("\n"))
}
