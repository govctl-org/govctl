mod render;

use self::render::render_nested_node;
use super::{status_list_text, type_mismatch, value_at_path};
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::path::{self, FieldPath, PathSegment};
use crate::cmd::edit::rules::{
    self as edit_rules, NestedNodeKind, NestedNodeRule, NestedRootRule, NestedScalarMode, Verb,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use serde_json::Value;

fn resolve_nested_root(
    artifact: ArtifactType,
    root: &str,
    id: &str,
) -> anyhow::Result<&'static NestedRootRule> {
    edit_rules::nested_root_rule(artifact.rule_key(), root).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0815PathFieldNotFound,
            format!("Unknown nested root '{}' for {}", root, artifact.rule_key()),
            id,
        )
        .into()
    })
}

pub fn get_nested_field(
    artifact: ArtifactType,
    doc: &Value,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let root_value = value_at_path(doc, rule.content_path);
    let (node, value) = descend_get(
        rule.node,
        root_value,
        &fp.segments[0],
        &fp.segments[1..],
        Verb::Get,
        id,
    )?;
    render_nested_node(node, value, id)
}

pub fn set_nested_field(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
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
    match node.kind {
        NestedNodeKind::Scalar => apply_nested_scalar_set(slot, node.set_mode, value, id),
        NestedNodeKind::List => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Field '{}' is a list; use an index to set a specific item, or use 'add'/'remove'",
                fp
            ),
            id,
        )
        .into()),
        NestedNodeKind::Object => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot set object path '{}' directly", fp),
            id,
        )
        .into()),
    }?;
    Ok(())
}

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
    let mut sorted = indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let mut removed = Vec::with_capacity(sorted.len());
    for idx in sorted {
        let val = list.remove(idx);
        let text = match item_rule.kind {
            NestedNodeKind::Scalar => val.as_str().unwrap_or_default().to_string(),
            NestedNodeKind::Object => {
                let text_key = node
                    .text_key
                    .ok_or_else(|| type_mismatch("Expected text_key for object list", id))?;
                status_list_text(&val, text_key, id)?.to_string()
            }
            NestedNodeKind::List => unreachable!("guarded above"),
        };
        removed.push(text);
    }
    removed.reverse();
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

fn descend_get<'a>(
    node: &'static NestedNodeRule,
    value: Option<&'a Value>,
    root_segment: &PathSegment,
    rest: &[PathSegment],
    verb: Verb,
    id: &str,
) -> anyhow::Result<(&'static NestedNodeRule, Option<&'a Value>)> {
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
) -> anyhow::Result<(&'static NestedNodeRule, Option<&'a Value>)> {
    if rest.is_empty() {
        if !node.verbs.contains(&verb.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Path does not support verb '{}'", verb.as_str()),
                id,
            )
            .into());
        }
        return Ok((node, value));
    }
    let seg = &rest[0];
    if node.kind != NestedNodeKind::Object {
        return Err(type_mismatch(
            &format!(
                "Cannot descend into non-object path '{}'",
                append_segment(current_path, seg)
            ),
            id,
        )
        .into());
    }
    let child = node
        .fields
        .iter()
        .find(|field| field.name == seg.name)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0815PathFieldNotFound,
                format!("Unknown nested field '{}'", seg.name),
                id,
            )
        })?;
    let child_value = value.and_then(|value| value.get(seg.name.as_str()));
    let next_path = append_segment(current_path, seg);
    let (child_node, child_value) = apply_optional_index(
        child.node,
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
) -> anyhow::Result<(&'static NestedNodeRule, Option<&'a Value>)> {
    let Some(index) = index else {
        return Ok((node, value));
    };
    if node.kind != NestedNodeKind::List {
        return Err(type_mismatch(
            &format!("Cannot index into non-list field '{field_name}' at '{path}'"),
            id,
        )
        .into());
    }
    let item = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    let selected = match value.and_then(Value::as_array) {
        Some(items) => Some(&items[path::resolve_index(index, items.len())?]),
        None => None,
    };
    Ok((item, selected))
}

fn descend_mut<'a>(
    node: &'static NestedNodeRule,
    value: &'a mut Value,
    root_segment: &PathSegment,
    rest: &[PathSegment],
    verb: Verb,
    id: &str,
) -> anyhow::Result<(&'static NestedNodeRule, &'a mut Value)> {
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
) -> anyhow::Result<(&'static NestedNodeRule, &'a mut Value)> {
    if rest.is_empty() {
        if !node.verbs.contains(&verb.as_str()) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Path does not support verb '{}'", verb.as_str()),
                id,
            )
            .into());
        }
        return Ok((node, value));
    }
    let seg = &rest[0];
    if node.kind != NestedNodeKind::Object {
        return Err(type_mismatch(
            &format!(
                "Cannot descend into non-object path '{}'",
                append_segment(current_path, seg)
            ),
            id,
        )
        .into());
    }
    let child = node
        .fields
        .iter()
        .find(|field| field.name == seg.name)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0815PathFieldNotFound,
                format!("Unknown nested field '{}'", seg.name),
                id,
            )
        })?;
    let obj = value
        .as_object_mut()
        .ok_or_else(|| type_mismatch("Expected object value", id))?;
    let child_value = obj
        .entry(seg.name.clone())
        .or_insert_with(|| default_value_for_node(child.node));
    let next_path = append_segment(current_path, seg);
    let (child_node, child_value) = apply_optional_index_mut(
        child.node,
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
) -> anyhow::Result<(&'static NestedNodeRule, &'a mut Value)> {
    let Some(index) = index else {
        return Ok((node, value));
    };
    if node.kind != NestedNodeKind::List {
        return Err(type_mismatch(
            &format!("Cannot index into non-list field '{field_name}' at '{path}'"),
            id,
        )
        .into());
    }
    let arr = value
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array value", id))?;
    let resolved = path::resolve_index(index, arr.len())?;
    let item = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    Ok((item, &mut arr[resolved]))
}

fn ensure_node_path_mut<'a>(
    doc: &'a mut Value,
    path: &[&str],
    node: &'static NestedNodeRule,
    id: &str,
) -> anyhow::Result<&'a mut Value> {
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

fn default_value_for_node(node: &NestedNodeRule) -> Value {
    match node.kind {
        NestedNodeKind::Scalar => Value::Null,
        NestedNodeKind::Object => Value::Object(serde_json::Map::new()),
        NestedNodeKind::List => Value::Array(Vec::new()),
    }
}

fn apply_nested_scalar_set(
    slot: &mut Value,
    mode: Option<NestedScalarMode>,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    match mode.unwrap_or(NestedScalarMode::String) {
        NestedScalarMode::String => *slot = Value::String(value.to_string()),
        NestedScalarMode::OptionalString { empty_as_null } => {
            if empty_as_null && value.is_empty() {
                *slot = Value::Null;
            } else {
                *slot = Value::String(value.to_string());
            }
        }
        NestedScalarMode::Integer => {
            let n: i64 = value.parse().map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0820InvalidFieldValue,
                    format!("Invalid integer value for {}: {value}", id),
                    id,
                )
            })?;
            *slot = Value::Number(serde_json::Number::from(n));
        }
        NestedScalarMode::Enum {
            allowed,
            invalid_msg,
            code,
        } => {
            if !allowed.contains(&value) {
                if let Some(code) = code {
                    return Err(Diagnostic::new(code, format!("{invalid_msg}: {value}"), id).into());
                }
                return Err(Diagnostic::new(
                    DiagnosticCode::E0820InvalidFieldValue,
                    format!("{invalid_msg}: {value}"),
                    id,
                )
                .into());
            }
            *slot = Value::String(value.to_string());
        }
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path(input: &str) -> Result<FieldPath, Box<dyn std::error::Error>> {
        Ok(path::parse_field_path(input)?.collapse_legacy_prefixes())
    }

    #[test]
    fn test_add_nested_object_list_value_deduplicates_by_text()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = json!({
            "content": {
                "alternatives": [
                    { "text": "Option A", "status": "considered", "pros": [], "cons": [] }
                ]
            }
        });

        add_nested_list_value(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives")?,
            "Option A",
            "ADR-0001",
        )?;
        add_nested_list_value(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives")?,
            "Option B",
            "ADR-0001",
        )?;

        let alternatives = doc["content"]["alternatives"]
            .as_array()
            .ok_or("expected array")?;
        assert_eq!(alternatives.len(), 2);
        assert_eq!(alternatives[1]["text"], "Option B");
        Ok(())
    }

    #[test]
    fn test_set_nested_field_rejects_list_path_without_index()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = json!({
            "content": {
                "alternatives": [
                    { "text": "Option A", "status": "considered", "pros": [], "cons": [] }
                ]
            }
        });

        let result = set_nested_field(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives[0].pros")?,
            "oops",
            "ADR-0001",
        );
        assert!(result.is_err());
        let err = result.err().ok_or("expected Err")?;
        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::E0817PathTypeMismatch);
        Ok(())
    }

    #[test]
    fn test_get_nested_field_renders_object_item_with_scalar_lists()
    -> Result<(), Box<dyn std::error::Error>> {
        let doc = json!({
            "content": {
                "alternatives": [
                    {
                        "text": "Option A",
                        "status": "accepted",
                        "pros": ["Readable", "Simple"],
                        "cons": ["More maintenance"],
                        "rejection_reason": null
                    }
                ]
            }
        });

        let rendered = get_nested_field(
            ArtifactType::Adr,
            &doc,
            &path("alternatives[0]")?,
            "ADR-0001",
        )?;

        assert!(rendered.contains("text: Option A"));
        assert!(rendered.contains("status: accepted"));
        assert!(rendered.contains("pros: Readable, Simple"));
        assert!(rendered.contains("cons: More maintenance"));
        Ok(())
    }
}
