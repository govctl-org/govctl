use super::super::{ArtifactType, path};
use super::mutate::{array_items_mut, ensure_array_path_mut};
use super::{
    remove_indices_preserving_order, simple_runtime_list_path, simple_status_list_spec,
    status_list_text, type_mismatch, unknown_field_error, value_at_path,
};
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
        let Some(items) = value_at_path(doc, path).and_then(Value::as_array) else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected an array value",
                id,
            ));
        };
        let resolved = path::resolve_index(index, items.len())?;
        let item = &items[resolved];
        return Ok(match item {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            _ => item.to_string(),
        });
    }

    if let Some(spec) = simple_status_list_spec(artifact, field) {
        let Some(items) = value_at_path(doc, spec.path).and_then(Value::as_array) else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected an array value",
                id,
            ));
        };
        let resolved = path::resolve_index(index, items.len())?;
        let item = &items[resolved];
        let Some(obj) = item.as_object() else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected object entries in array",
                id,
            ));
        };
        let status = obj
            .get(spec.status_key)
            .and_then(Value::as_str)
            .unwrap_or_default();
        let text = obj
            .get(spec.text_key)
            .and_then(Value::as_str)
            .unwrap_or_default();
        return Ok(format!("[{status}] {text}"));
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

    let texts: Vec<&str> = items
        .iter()
        .map(|item| {
            item.as_str().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    "Expected string entries in array",
                    id,
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let indices = resolve(&texts)?;
    let removed = remove_indices_preserving_order(items, indices, |item| {
        Ok(item
            .as_str()
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    "Expected string entries in array",
                    id,
                )
            })?
            .to_string())
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

    let texts: Vec<&str> = items
        .iter()
        .map(|item| status_list_text(item, spec.text_key, id))
        .collect::<Result<Vec<_>, _>>()?;
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
    let texts: Vec<&str> = items
        .iter()
        .map(|item| status_list_text(item, spec.text_key, id))
        .collect::<Result<Vec<_>, _>>()?;
    let idx = resolve(&texts)?[0];
    let text = texts[idx].to_string();
    let obj = items[idx].as_object_mut().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            "Expected object entries in array",
            id,
        )
    })?;
    obj.insert(
        spec.status_key.to_string(),
        Value::String(new_status.to_string()),
    );
    Ok(Some(text))
}
