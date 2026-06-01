use super::super::support::{remove_indices_preserving_order, status_list_text, type_mismatch};
use super::resolve_nested_root;
use super::traverse::{default_value_for_node, descend_mut, ensure_node_path_mut};
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::path::{self, FieldPath};
use crate::cmd::edit::rules::{NestedNodeKind, NestedNodeRule, Verb};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

struct NestedListTarget<'a> {
    node: &'static NestedNodeRule,
    item_rule: &'static NestedNodeRule,
    list: &'a mut Vec<Value>,
}

pub fn add_nested_list_value(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    if fp.has_terminal_index() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Cannot add to indexed path '{}' (use set/remove for a specific element)",
                fp
            ),
            id,
        ));
    }

    let NestedListTarget {
        node,
        item_rule,
        list,
    } = nested_list_target_mut(
        artifact,
        doc,
        fp,
        Verb::Add,
        id,
        Some(format!("Field '{}' is not a list; cannot add to it", fp)),
    )?;

    match item_rule.kind {
        NestedNodeKind::Scalar => {
            if !list.iter().any(|v| v.as_str() == Some(value)) {
                list.push(Value::String(value.to_string()));
            }
        }
        NestedNodeKind::Object => {
            let Some(text_key) = node.text_key else {
                return Err(plain_string_for_structured_list(fp, id));
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
            return Err(plain_string_for_structured_list(fp, id));
        }
    }
    Ok(())
}

fn plain_string_for_structured_list(fp: &FieldPath, id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0817PathTypeMismatch,
        format!(
            "Field '{fp}' requires structured list items and cannot be appended with a plain string"
        ),
        id,
    )
}

pub fn remove_nested_list_values<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    id: &str,
    resolve: F,
) -> DiagnosticResult<Vec<String>>
where
    F: FnOnce(&[&str]) -> DiagnosticResult<Vec<usize>>,
{
    let NestedListTarget {
        node,
        item_rule,
        list,
    } = nested_list_target_mut(artifact, doc, fp, Verb::Remove, id, None)?;

    let texts = list_item_texts(node, item_rule, list, id)?;
    let indices = resolve(&texts)?;
    let removed = remove_indices_preserving_order(list, indices, |val| {
        Ok(list_item_text(node, item_rule, val, id)?.to_string())
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
) -> DiagnosticResult<String>
where
    F: FnOnce(&[&str]) -> DiagnosticResult<Vec<usize>>,
{
    let NestedListTarget {
        node,
        item_rule,
        list,
    } = nested_list_target_mut(artifact, doc, fp, Verb::Tick, id, None)?;
    if item_rule.kind != NestedNodeKind::Object {
        return Err(type_mismatch(
            "Expected object entries in tickable list",
            id,
        ));
    }
    let _text_key = node
        .text_key
        .ok_or_else(|| type_mismatch("Expected text_key for tickable list", id))?;
    let texts = list_item_texts(node, item_rule, list, id)?;
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

fn list_item_texts<'a>(
    node: &NestedNodeRule,
    item_rule: &NestedNodeRule,
    list: &'a [Value],
    id: &str,
) -> DiagnosticResult<Vec<&'a str>> {
    list.iter()
        .map(|item| list_item_text(node, item_rule, item, id))
        .collect()
}

fn list_item_text<'a>(
    node: &NestedNodeRule,
    item_rule: &NestedNodeRule,
    item: &'a Value,
    id: &str,
) -> DiagnosticResult<&'a str> {
    match item_rule.kind {
        NestedNodeKind::Scalar => item.as_str().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected string items in list",
                id,
            )
        }),
        NestedNodeKind::Object => {
            let text_key = node
                .text_key
                .ok_or_else(|| type_mismatch("Expected text_key for object list", id))?;
            status_list_text(item, text_key, id)
        }
        NestedNodeKind::List => Err(type_mismatch("Expected scalar or object items in list", id)),
    }
}

pub fn set_nested_list_item(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    index: i32,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let NestedListTarget {
        item_rule, list, ..
    } = nested_list_target_mut(artifact, doc, fp, Verb::Set, id, None)?;
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
        )),
        NestedNodeKind::List => Err(type_mismatch("Expected scalar list item", id)),
    }
}

fn nested_list_target_mut<'a>(
    artifact: ArtifactType,
    doc: &'a mut Value,
    fp: &FieldPath,
    verb: Verb,
    id: &str,
    not_list_message: Option<String>,
) -> DiagnosticResult<NestedListTarget<'a>> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let root_value = ensure_node_path_mut(doc, rule.content_path, rule.node, id)?;
    let (node, slot) = descend_mut(
        rule.node,
        root_value,
        &fp.segments[0],
        &fp.segments[1..],
        verb,
        id,
    )?;
    if node.kind != NestedNodeKind::List {
        let message =
            not_list_message.unwrap_or_else(|| "Expected array for list field".to_string());
        return Err(type_mismatch(&message, id));
    }
    let item_rule = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    let list = slot
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;
    Ok(NestedListTarget {
        node,
        item_rule,
        list,
    })
}
