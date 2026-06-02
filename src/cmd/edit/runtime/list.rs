use super::super::{ArtifactType, path};
use super::mutate::{array_items_mut, ensure_array_path_mut};
use super::support::{
    remove_indices_preserving_order, scalar_list_item_text, set_object_string_field,
    status_list_entry_line, status_list_text, status_list_texts, string_list_item_text,
    string_list_item_texts, type_mismatch, unknown_field_error, value_at_path,
};
use super::{simple_runtime_list_path, simple_status_list_spec};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

pub fn get_simple_list_item(
    artifact: ArtifactType,
    doc: &Value,
    field: &str,
    index: i32,
    id: &str,
) -> DiagnosticResult<String> {
    if let Some(path) = simple_runtime_list_path(artifact, field) {
        let items = array_items(doc, path, id)?;
        let resolved = path::resolve_index(index, items.len())?;
        return Ok(scalar_list_item_text(&items[resolved]));
    }

    if let Some(spec) = simple_status_list_spec(artifact, field) {
        let items = array_items(doc, spec.path, id)?;
        let resolved = path::resolve_index(index, items.len())?;
        return status_list_entry_line(&items[resolved], spec.status_key, spec.text_key, id);
    }

    Err(unknown_field_error(artifact, field, id))
}

pub fn add_simple_list_value(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    value: &str,
    id: &str,
) -> DiagnosticResult<bool> {
    let Some(path) = simple_runtime_list_path(artifact, field) else {
        return Ok(false);
    };
    let slot = ensure_array_path_mut(doc, path, id)?;
    let items = slot.as_array_mut().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            "Expected an array value",
            id,
        )
    })?;
    if !items.iter().any(|item| item.as_str() == Some(value)) {
        items.push(Value::String(value.to_string()));
    }
    Ok(true)
}

pub fn set_simple_list_item(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    index: i32,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let Some(path) = simple_runtime_list_path(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id));
    };
    let items = array_items_mut(doc, path, id)?;
    let resolved = path::resolve_index(index, items.len())?;
    let slot = &mut items[resolved];
    if !slot.is_string() && !slot.is_null() {
        return Err(type_mismatch("Expected string item in list", id));
    }
    *slot = Value::String(value.to_string());
    Ok(())
}

pub fn remove_simple_list_values_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    id: &str,
    resolve: F,
) -> DiagnosticResult<Option<Vec<String>>>
where
    F: FnOnce(&[&str]) -> DiagnosticResult<Vec<usize>>,
{
    let Some(path) = simple_runtime_list_path(artifact, field) else {
        return Ok(None);
    };
    let items = array_items_mut(doc, path, id)?;

    let texts = string_list_item_texts(items, "Expected string entries in array", id)?;
    let indices = resolve(&texts)?;
    let removed = remove_indices_preserving_order(items, indices, |item| {
        Ok(string_list_item_text(item, "Expected string entries in array", id)?.to_string())
    })?;
    Ok(Some(removed))
}

pub fn remove_simple_status_list_values_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    id: &str,
    resolve: F,
) -> DiagnosticResult<Option<Vec<String>>>
where
    F: FnOnce(&[&str]) -> DiagnosticResult<Vec<usize>>,
{
    let Some(spec) = simple_status_list_spec(artifact, field) else {
        return Ok(None);
    };
    let items = array_items_mut(doc, spec.path, id)?;

    let texts = status_list_texts(items, spec.text_key, id)?;
    let indices = resolve(&texts)?;
    let removed = remove_indices_preserving_order(items, indices, |item| {
        Ok(status_list_text(item, spec.text_key, id)?.to_string())
    })?;
    Ok(Some(removed))
}

pub fn tick_simple_status_list_item_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    id: &str,
    new_status: &str,
    resolve: F,
) -> DiagnosticResult<Option<String>>
where
    F: FnOnce(&[&str]) -> DiagnosticResult<Vec<usize>>,
{
    let Some(spec) = simple_status_list_spec(artifact, field) else {
        return Ok(None);
    };
    let items = array_items_mut(doc, spec.path, id)?;
    let texts = status_list_texts(items, spec.text_key, id)?;
    let idx = resolve(&texts)?[0];
    let text = texts[idx].to_string();
    set_object_string_field(
        &mut items[idx],
        spec.status_key,
        new_status,
        "Expected object entries in array",
        id,
    )?;
    Ok(Some(text))
}

fn array_items<'a>(doc: &'a Value, path: &[&str], id: &str) -> DiagnosticResult<&'a [Value]> {
    value_at_path(doc, path)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected an array value",
                id,
            )
        })
}
