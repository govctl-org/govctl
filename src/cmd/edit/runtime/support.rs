use crate::cmd::edit::ArtifactType;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

pub(super) fn status_list_text<'a>(
    item: &'a Value,
    text_key: &str,
    id: &str,
) -> Result<&'a str, Diagnostic> {
    item.as_object()
        .and_then(|obj| obj.get(text_key))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected object entries in array",
                id,
            )
        })
}

pub(super) fn scalar_list_item_text(item: &Value) -> String {
    match item {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => item.to_string(),
    }
}

pub(super) fn status_list_entry_line(
    item: &Value,
    status_key: &str,
    text_key: &str,
    id: &str,
) -> DiagnosticResult<String> {
    let Some(obj) = item.as_object() else {
        return Err(type_mismatch("Expected object entries in array", id));
    };
    let status = obj
        .get(status_key)
        .and_then(Value::as_str)
        .unwrap_or_default();
    let text = obj
        .get(text_key)
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(format!("[{status}] {text}"))
}

pub(super) fn remove_indices_preserving_order<F>(
    items: &mut Vec<Value>,
    indices: Vec<usize>,
    mut removed_text: F,
) -> DiagnosticResult<Vec<String>>
where
    F: FnMut(&Value) -> DiagnosticResult<String>,
{
    let mut sorted = indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let mut removed = Vec::with_capacity(sorted.len());
    for idx in sorted {
        let item = items.remove(idx);
        removed.push(removed_text(&item)?);
    }
    removed.reverse();
    Ok(removed)
}

pub(super) fn unknown_field_error(artifact: ArtifactType, field: &str, id: &str) -> Diagnostic {
    let msg = match artifact {
        ArtifactType::Rfc => format!("Unknown RFC field: {field}"),
        ArtifactType::Clause => format!("Unknown clause field: {field}"),
        ArtifactType::Adr => format!("Unknown ADR field: {field}"),
        ArtifactType::WorkItem => format!("Unknown work item field: {field}"),
        ArtifactType::Guard => format!("Unknown guard field: {field}"),
    };
    Diagnostic::new(DiagnosticCode::E0803UnknownField, msg, id)
}

pub(super) fn value_at_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for key in path {
        cur = cur.get(*key)?;
    }
    Some(cur)
}

pub(super) fn type_mismatch(msg: &str, id: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, msg, id)
}
