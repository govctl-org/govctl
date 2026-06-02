use crate::cmd::edit::ArtifactType;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

pub(super) fn status_list_text<'a>(
    item: &'a Value,
    text_key: &str,
    id: &str,
) -> DiagnosticResult<&'a str> {
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

pub(super) fn status_list_texts<'a>(
    items: &'a [Value],
    text_key: &str,
    id: &str,
) -> DiagnosticResult<Vec<&'a str>> {
    items
        .iter()
        .map(|item| status_list_text(item, text_key, id))
        .collect()
}

pub(super) fn scalar_list_item_text(item: &Value) -> String {
    match item {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => item.to_string(),
    }
}

pub(super) fn string_list_item_text<'a>(
    item: &'a Value,
    expected: &str,
    id: &str,
) -> DiagnosticResult<&'a str> {
    item.as_str().ok_or_else(|| type_mismatch(expected, id))
}

pub(super) fn string_list_item_texts<'a>(
    items: &'a [Value],
    expected: &str,
    id: &str,
) -> DiagnosticResult<Vec<&'a str>> {
    items
        .iter()
        .map(|item| string_list_item_text(item, expected, id))
        .collect()
}

pub(super) fn joined_scalar_list_text(items: &[Value], sep: &str) -> String {
    items
        .iter()
        .map(scalar_list_item_text)
        .collect::<Vec<_>>()
        .join(sep)
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

pub(super) fn set_object_string_field(
    item: &mut Value,
    field: &str,
    value: &str,
    expected: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let obj = item
        .as_object_mut()
        .ok_or_else(|| type_mismatch(expected, id))?;
    obj.insert(field.to_string(), Value::String(value.to_string()));
    Ok(())
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

pub(super) fn ensure_path_mut_with_leaf<'a>(
    mut cur: &'a mut Value,
    path: &[&str],
    id: &str,
    mut leaf_default: impl FnMut() -> Value,
) -> DiagnosticResult<&'a mut Value> {
    for (idx, key) in path.iter().enumerate() {
        let is_leaf = idx + 1 == path.len();
        let obj = cur.as_object_mut().ok_or_else(|| path_mismatch(path, id))?;
        if !obj.contains_key(*key) {
            obj.insert(
                (*key).to_string(),
                if is_leaf {
                    leaf_default()
                } else {
                    Value::Object(serde_json::Map::new())
                },
            );
        }
        cur = obj.get_mut(*key).ok_or_else(|| path_mismatch(path, id))?;
    }
    Ok(cur)
}

fn path_mismatch(path: &[&str], id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0817PathTypeMismatch,
        format!("Cannot resolve field path '{}'", path.join(".")),
        id,
    )
}

pub(super) fn type_mismatch(msg: &str, id: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, msg, id)
}
