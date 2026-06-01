mod list;
mod mutate;
mod nested;
mod render;

pub use self::list::{
    add_simple_list_value, get_simple_list_item, remove_simple_list_values_with_matcher,
    remove_simple_status_list_values_with_matcher, set_simple_list_item,
    tick_simple_status_list_item_with_matcher,
};
pub use nested::{
    add_nested_list_value, get_nested_field, remove_nested_list_values, set_nested_field,
    set_nested_list_item, tick_nested_list_item_with_matcher,
};

use self::mutate::{apply_set, ensure_value_path_mut};
use self::render::render_field;
use super::ArtifactType;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

#[derive(Clone, Copy)]
enum RenderMode {
    Scalar,
    CsvStrings,
    LineStrings,
    StatusLines {
        status_key: &'static str,
        text_key: &'static str,
    },
}

#[derive(Clone, Copy)]
struct SimpleFieldSpec {
    path: &'static [&'static str],
    render: RenderMode,
}

#[derive(Clone, Copy)]
enum SetMode {
    String,
    Integer,
    Enum {
        allowed: &'static [&'static str],
        invalid_msg: &'static str,
        code: Option<DiagnosticCode>,
    },
}

#[derive(Clone, Copy)]
struct SimpleSetSpec {
    path: &'static [&'static str],
    mode: SetMode,
}

#[derive(Clone, Copy)]
struct StatusListSpec {
    path: &'static [&'static str],
    status_key: &'static str,
    text_key: &'static str,
}

#[derive(Clone, Copy)]
struct RuntimeFieldEntry {
    artifact: ArtifactType,
    field: &'static str,
    get: Option<SimpleFieldSpec>,
    set: Option<SimpleSetSpec>,
    list_path: Option<&'static [&'static str]>,
}

include!(concat!(env!("OUT_DIR"), "/edit_runtime_generated.rs"));

fn runtime_field_entry(artifact: ArtifactType, field: &str) -> Option<&'static RuntimeFieldEntry> {
    RUNTIME_FIELDS
        .iter()
        .find(|entry| entry.artifact == artifact && entry.field == field)
}

/// Read a simple field from a serialized artifact document.
pub fn get_simple_field(
    artifact: ArtifactType,
    doc: &Value,
    field: &str,
    id: &str,
) -> DiagnosticResult<String> {
    let Some(spec) = simple_field_spec(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id));
    };
    render_field(doc, spec, id)
}

/// Set a simple field on a serialized artifact document.
pub fn set_simple_field(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let Some(spec) = simple_set_spec(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id));
    };
    apply_set(doc, spec, value, id)
}

/// Set a simple field bypassing user-facing verb ownership checks.
///
/// Used by dedicated lifecycle verbs that still need to mutate fields
/// whose generic `set` entry points are intentionally banned.
pub fn set_simple_field_forced(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    if let Some(spec) = simple_set_spec(artifact, field) {
        return apply_set(doc, spec, value, id);
    }

    let Some(spec) = simple_field_spec(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id));
    };
    let slot = ensure_value_path_mut(doc, spec.path, id)?;
    *slot = Value::String(value.to_string());
    Ok(())
}

pub fn supports_simple_set_field(artifact: ArtifactType, field: &str) -> bool {
    simple_set_spec(artifact, field).is_some()
}

fn simple_field_spec(artifact: ArtifactType, field: &str) -> Option<SimpleFieldSpec> {
    runtime_field_entry(artifact, field).and_then(|entry| entry.get)
}

fn simple_set_spec(artifact: ArtifactType, field: &str) -> Option<SimpleSetSpec> {
    runtime_field_entry(artifact, field).and_then(|entry| entry.set)
}

fn simple_runtime_list_path(
    artifact: ArtifactType,
    field: &str,
) -> Option<&'static [&'static str]> {
    runtime_field_entry(artifact, field).and_then(|entry| entry.list_path)
}

fn simple_status_list_spec(artifact: ArtifactType, field: &str) -> Option<StatusListSpec> {
    let entry = runtime_field_entry(artifact, field)?;
    let get = entry.get?;
    let RenderMode::StatusLines {
        status_key,
        text_key,
    } = get.render
    else {
        return None;
    };
    Some(StatusListSpec {
        path: get.path,
        status_key,
        text_key,
    })
}

fn status_list_text<'a>(item: &'a Value, text_key: &str, id: &str) -> Result<&'a str, Diagnostic> {
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

fn scalar_list_item_text(item: &Value) -> String {
    match item {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => item.to_string(),
    }
}

fn status_list_entry_line(
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

fn remove_indices_preserving_order<F>(
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

fn unknown_field_error(artifact: ArtifactType, field: &str, id: &str) -> Diagnostic {
    let msg = match artifact {
        ArtifactType::Rfc => format!("Unknown RFC field: {field}"),
        ArtifactType::Clause => format!("Unknown clause field: {field}"),
        ArtifactType::Adr => format!("Unknown ADR field: {field}"),
        ArtifactType::WorkItem => format!("Unknown work item field: {field}"),
        ArtifactType::Guard => format!("Unknown guard field: {field}"),
    };
    Diagnostic::new(DiagnosticCode::E0803UnknownField, msg, id)
}

fn value_at_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for key in path {
        cur = cur.get(*key)?;
    }
    Some(cur)
}

fn type_mismatch(msg: &str, id: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, msg, id)
}
