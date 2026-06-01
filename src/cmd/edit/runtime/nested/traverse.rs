use super::super::type_mismatch;
use crate::cmd::edit::path::{self, PathSegment};
use crate::cmd::edit::rules::{NestedNodeKind, NestedNodeRule, Verb};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

pub(super) fn descend_get<'a>(
    node: &'static NestedNodeRule,
    value: Option<&'a Value>,
    root_segment: &PathSegment,
    rest: &[PathSegment],
    verb: Verb,
    id: &str,
) -> DiagnosticResult<(&'static NestedNodeRule, Option<&'a Value>)> {
    let root_path = format_segment(root_segment);
    let (node, value) = apply_optional_index(
        node,
        value,
        root_segment.index,
        id,
        &root_path,
        &root_segment.name,
    )?;
    descend_get_rest(node, value, rest, verb, id, &root_path)
}

fn descend_get_rest<'a>(
    node: &'static NestedNodeRule,
    value: Option<&'a Value>,
    rest: &[PathSegment],
    verb: Verb,
    id: &str,
    current_path: &str,
) -> DiagnosticResult<(&'static NestedNodeRule, Option<&'a Value>)> {
    if rest.is_empty() {
        ensure_verb_supported(node, verb, id)?;
        return Ok((node, value));
    }
    let seg = &rest[0];
    let child_node = resolve_child_node(node, seg, current_path, id)?;
    let child_value = value.and_then(|value| value.get(seg.name.as_str()));
    let next_path = append_segment(current_path, seg);
    let (child_node, child_value) = apply_optional_index(
        child_node,
        child_value,
        seg.index,
        id,
        &next_path,
        &seg.name,
    )?;
    descend_get_rest(child_node, child_value, &rest[1..], verb, id, &next_path)
}

fn apply_optional_index<'a>(
    node: &'static NestedNodeRule,
    value: Option<&'a Value>,
    index: Option<i32>,
    id: &str,
    path: &str,
    field_name: &str,
) -> DiagnosticResult<(&'static NestedNodeRule, Option<&'a Value>)> {
    let Some(index) = index else {
        return Ok((node, value));
    };
    ensure_indexable_list(node, id, path, field_name)?;
    let item = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    let selected = match value.and_then(Value::as_array) {
        Some(items) => Some(&items[path::resolve_index(index, items.len())?]),
        None => None,
    };
    Ok((item, selected))
}

pub(super) fn descend_mut<'a>(
    node: &'static NestedNodeRule,
    value: &'a mut Value,
    root_segment: &PathSegment,
    rest: &[PathSegment],
    verb: Verb,
    id: &str,
) -> DiagnosticResult<(&'static NestedNodeRule, &'a mut Value)> {
    let root_path = format_segment(root_segment);
    let (node, value) = apply_optional_index_mut(
        node,
        value,
        root_segment.index,
        id,
        &root_path,
        &root_segment.name,
    )?;
    descend_mut_rest(node, value, rest, verb, id, &root_path)
}

fn descend_mut_rest<'a>(
    node: &'static NestedNodeRule,
    value: &'a mut Value,
    rest: &[PathSegment],
    verb: Verb,
    id: &str,
    current_path: &str,
) -> DiagnosticResult<(&'static NestedNodeRule, &'a mut Value)> {
    if rest.is_empty() {
        ensure_verb_supported(node, verb, id)?;
        return Ok((node, value));
    }
    let seg = &rest[0];
    let child_node = resolve_child_node(node, seg, current_path, id)?;
    let obj = value
        .as_object_mut()
        .ok_or_else(|| type_mismatch("Expected object value", id))?;
    let child_value = obj
        .entry(seg.name.clone())
        .or_insert_with(|| default_value_for_node(child_node));
    let next_path = append_segment(current_path, seg);
    let (child_node, child_value) = apply_optional_index_mut(
        child_node,
        child_value,
        seg.index,
        id,
        &next_path,
        &seg.name,
    )?;
    descend_mut_rest(child_node, child_value, &rest[1..], verb, id, &next_path)
}

fn apply_optional_index_mut<'a>(
    node: &'static NestedNodeRule,
    value: &'a mut Value,
    index: Option<i32>,
    id: &str,
    path: &str,
    field_name: &str,
) -> DiagnosticResult<(&'static NestedNodeRule, &'a mut Value)> {
    let Some(index) = index else {
        return Ok((node, value));
    };
    ensure_indexable_list(node, id, path, field_name)?;
    let arr = value
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array value", id))?;
    let resolved = path::resolve_index(index, arr.len())?;
    let item = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    Ok((item, &mut arr[resolved]))
}

fn ensure_verb_supported(node: &NestedNodeRule, verb: Verb, id: &str) -> DiagnosticResult<()> {
    if node.verbs.contains(&verb.as_str()) {
        return Ok(());
    }
    Err(Diagnostic::new(
        DiagnosticCode::E0817PathTypeMismatch,
        format!("Path does not support verb '{}'", verb.as_str()),
        id,
    ))
}

fn resolve_child_node(
    node: &'static NestedNodeRule,
    seg: &PathSegment,
    current_path: &str,
    id: &str,
) -> DiagnosticResult<&'static NestedNodeRule> {
    if node.kind != NestedNodeKind::Object {
        return Err(type_mismatch(
            &format!(
                "Cannot descend into non-object path '{}'",
                append_segment(current_path, seg)
            ),
            id,
        ));
    }
    node.fields
        .iter()
        .find(|field| field.name == seg.name)
        .map(|field| field.node)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0815PathFieldNotFound,
                format!("Unknown nested field '{}'", seg.name),
                id,
            )
        })
}

fn ensure_indexable_list(
    node: &NestedNodeRule,
    id: &str,
    path: &str,
    field_name: &str,
) -> DiagnosticResult<()> {
    if node.kind == NestedNodeKind::List {
        return Ok(());
    }
    Err(type_mismatch(
        &format!("Cannot index into non-list field '{field_name}' at '{path}'"),
        id,
    ))
}

pub(super) fn ensure_node_path_mut<'a>(
    doc: &'a mut Value,
    path: &[&str],
    node: &'static NestedNodeRule,
    id: &str,
) -> DiagnosticResult<&'a mut Value> {
    let mut cur = doc;
    for (idx, key) in path.iter().enumerate() {
        let is_leaf = idx + 1 == path.len();
        let obj = cur.as_object_mut().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Cannot resolve field path '{}'", path.join(".")),
                id,
            )
        })?;
        if !obj.contains_key(*key) {
            obj.insert(
                (*key).to_string(),
                if is_leaf {
                    default_value_for_node(node)
                } else {
                    Value::Object(serde_json::Map::new())
                },
            );
        }
        cur = obj.get_mut(*key).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Cannot resolve field path '{}'", path.join(".")),
                id,
            )
        })?;
    }
    Ok(cur)
}

pub(super) fn default_value_for_node(node: &NestedNodeRule) -> Value {
    match node.kind {
        NestedNodeKind::Scalar => Value::Null,
        NestedNodeKind::Object => Value::Object(serde_json::Map::new()),
        NestedNodeKind::List => Value::Array(Vec::new()),
    }
}

fn format_segment(seg: &PathSegment) -> String {
    match seg.index {
        Some(idx) => format!("{}[{idx}]", seg.name),
        None => seg.name.clone(),
    }
}

fn append_segment(prefix: &str, seg: &PathSegment) -> String {
    format!("{prefix}.{}", format_segment(seg))
}
