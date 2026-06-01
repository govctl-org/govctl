use super::super::{remove_indices_preserving_order, status_list_text, type_mismatch};
use super::resolve_nested_root;
use super::traverse::{default_value_for_node, descend_mut, ensure_node_path_mut};
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::path::{self, FieldPath};
use crate::cmd::edit::rules::{NestedNodeKind, Verb};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use serde_json::Value;

pub fn add_nested_list_value(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    if fp.has_terminal_index() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Cannot add to indexed path '{}' (use set/remove for a specific element)",
                fp
            ),
            id,
        )
        .into());
    }
    let root_value = ensure_node_path_mut(doc, rule.content_path, rule.node, id)?;
    let (node, slot) = descend_mut(
        rule.node,
        root_value,
        &fp.segments[0],
        &fp.segments[1..],
        Verb::Add,
        id,
    )?;
    if node.kind != NestedNodeKind::List {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Field '{}' is not a list; cannot add to it", fp),
            id,
        )
        .into());
    }
    let item_rule = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    let list = slot
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;

    match item_rule.kind {
        NestedNodeKind::Scalar => {
            if !list.iter().any(|v| v.as_str() == Some(value)) {
                list.push(Value::String(value.to_string()));
            }
        }
        NestedNodeKind::Object => {
            let Some(text_key) = node.text_key else {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    format!(
                        "Field '{}' requires structured list items and cannot be appended with a plain string",
                        fp
                    ),
                    id,
                )
                .into());
            };
            let duplicate = list.iter().any(|item| {
                item.as_object()
                    .and_then(|obj| obj.get(text_key))
                    .and_then(Value::as_str)
                    == Some(value)
            });
            if !duplicate {
                let mut item = default_value_for_node(item_rule);
                let obj = item
                    .as_object_mut()
                    .ok_or_else(|| type_mismatch("Expected object list item", id))?;
                obj.insert(text_key.to_string(), Value::String(value.to_string()));
                list.push(item);
            }
        }
        NestedNodeKind::List => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!(
                    "Field '{}' requires structured list items and cannot be appended with a plain string",
                    fp
                ),
                id,
            )
            .into());
        }
    }
    Ok(())
}

pub fn remove_nested_list_values<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    id: &str,
    resolve: F,
) -> anyhow::Result<Vec<String>>
where
    F: FnOnce(&[&str]) -> anyhow::Result<Vec<usize>>,
{
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let root_value = ensure_node_path_mut(doc, rule.content_path, rule.node, id)?;
    let (node, slot) = descend_mut(
        rule.node,
        root_value,
        &fp.segments[0],
        &fp.segments[1..],
        Verb::Remove,
        id,
    )?;
    if node.kind != NestedNodeKind::List {
        return Err(type_mismatch("Expected array for list field", id).into());
    }
    let item_rule = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    let list = slot
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;

    let texts: Vec<&str> = match item_rule.kind {
        NestedNodeKind::Scalar => list
            .iter()
            .map(|v| {
                v.as_str().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::E0817PathTypeMismatch,
                        "Expected string items in list",
                        id,
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        NestedNodeKind::Object => {
            let text_key = node
                .text_key
                .ok_or_else(|| type_mismatch("Expected text_key for object list", id))?;
            list.iter()
                .map(|v| status_list_text(v, text_key, id))
                .collect::<Result<Vec<_>, _>>()?
        }
        NestedNodeKind::List => {
            return Err(type_mismatch("Expected scalar or object items in list", id).into());
        }
    };

    let indices = resolve(&texts)?;
    let removed = remove_indices_preserving_order(list, indices, |val| {
        Ok(match item_rule.kind {
            NestedNodeKind::Scalar => val.as_str().unwrap_or_default().to_string(),
            NestedNodeKind::Object => {
                let text_key = node
                    .text_key
                    .ok_or_else(|| type_mismatch("Expected text_key for object list", id))?;
                status_list_text(val, text_key, id)?.to_string()
            }
            NestedNodeKind::List => unreachable!("guarded above"),
        })
    })?;
    Ok(removed)
}

pub fn tick_nested_list_item_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    id: &str,
    new_status: &str,
    resolve: F,
) -> anyhow::Result<String>
where
    F: FnOnce(&[&str]) -> anyhow::Result<Vec<usize>>,
{
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let root_value = ensure_node_path_mut(doc, rule.content_path, rule.node, id)?;
    let (node, slot) = descend_mut(
        rule.node,
        root_value,
        &fp.segments[0],
        &fp.segments[1..],
        Verb::Tick,
        id,
    )?;
    if node.kind != NestedNodeKind::List {
        return Err(type_mismatch("Expected array for list field", id).into());
    }
    let item_rule = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    if item_rule.kind != NestedNodeKind::Object {
        return Err(type_mismatch("Expected object entries in tickable list", id).into());
    }
    let list = slot
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;
    let text_key = node
        .text_key
        .ok_or_else(|| type_mismatch("Expected text_key for tickable list", id))?;
    let texts: Vec<&str> = list
        .iter()
        .map(|item| status_list_text(item, text_key, id))
        .collect::<Result<Vec<_>, _>>()?;
    let idx = resolve(&texts)?[0];
    let text = texts[idx].to_string();
    let status_key = item_rule
        .fields
        .iter()
        .find(|field| field.name == "status")
        .map(|field| field.name)
        .ok_or_else(|| type_mismatch("Expected status field for tickable list", id))?;
    let obj = list[idx]
        .as_object_mut()
        .ok_or_else(|| type_mismatch("Expected object entries in tickable list", id))?;
    obj.insert(
        status_key.to_string(),
        Value::String(new_status.to_string()),
    );
    Ok(text)
}

pub fn set_nested_list_item(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    index: i32,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let root_value = ensure_node_path_mut(doc, rule.content_path, rule.node, id)?;
    let (node, slot) = descend_mut(
        rule.node,
        root_value,
        &fp.segments[0],
        &fp.segments[1..],
        Verb::Set,
        id,
    )?;
    if node.kind != NestedNodeKind::List {
        return Err(type_mismatch("Expected array for list field", id).into());
    }
    let item_rule = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    let list = slot
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;
    let resolved = path::resolve_index(index, list.len())?;
    match item_rule.kind {
        NestedNodeKind::Scalar => {
            list[resolved] = Value::String(value.to_string());
            Ok(())
        }
        NestedNodeKind::Object => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot set object path '{}[{}]' directly", fp, index),
            id,
        )
        .into()),
        NestedNodeKind::List => Err(type_mismatch("Expected scalar list item", id).into()),
    }
}
