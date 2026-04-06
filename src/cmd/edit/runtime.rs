use super::ArtifactType;
use super::path::{self, FieldPath};
use super::rules::{
    self as edit_rules, NestedNodeKind, NestedNodeRule, NestedRootRule, NestedScalarMode, Verb,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use serde_json::Value;

#[derive(Clone, Copy)]
enum RenderMode {
    Scalar,
    CsvStrings,
    LineStrings,
    TextLines {
        text_key: &'static str,
    },
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
    #[allow(dead_code)]
    OptionalString {
        empty_as_null: bool,
    },
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
) -> anyhow::Result<String> {
    let Some(spec) = simple_field_spec(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id).into());
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
) -> anyhow::Result<()> {
    let Some(spec) = simple_set_spec(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id).into());
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
) -> anyhow::Result<()> {
    if let Some(spec) = simple_set_spec(artifact, field) {
        return apply_set(doc, spec, value, id);
    }

    let Some(spec) = simple_field_spec(artifact, field) else {
        return Err(unknown_field_error(artifact, field, id).into());
    };
    let slot = ensure_value_path_mut(doc, spec.path, id)?;
    *slot = Value::String(value.to_string());
    Ok(())
}

pub fn supports_simple_set_field(artifact: ArtifactType, field: &str) -> bool {
    simple_set_spec(artifact, field).is_some()
}

pub fn add_simple_list_value(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    value: &str,
    id: &str,
) -> anyhow::Result<bool> {
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

pub fn remove_simple_list_values_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    id: &str,
    resolve: F,
) -> anyhow::Result<Option<Vec<String>>>
where
    F: FnOnce(&[&str]) -> anyhow::Result<Vec<usize>>,
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
    let mut sorted = indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let mut removed = Vec::with_capacity(sorted.len());
    for idx in sorted {
        let item = items.remove(idx);
        let text = item.as_str().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected string entries in array",
                id,
            )
        })?;
        removed.push(text.to_string());
    }
    removed.reverse();
    Ok(Some(removed))
}

pub fn remove_simple_status_list_values_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    id: &str,
    resolve: F,
) -> anyhow::Result<Option<Vec<String>>>
where
    F: FnOnce(&[&str]) -> anyhow::Result<Vec<usize>>,
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
    let mut sorted = indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let mut removed = Vec::with_capacity(sorted.len());
    for idx in sorted {
        let item = items.remove(idx);
        removed.push(status_list_text(&item, spec.text_key, id)?.to_string());
    }
    removed.reverse();
    Ok(Some(removed))
}

pub fn tick_simple_status_list_item_with_matcher<F>(
    artifact: ArtifactType,
    doc: &mut Value,
    field: &str,
    id: &str,
    new_status: &str,
    resolve: F,
) -> anyhow::Result<Option<String>>
where
    F: FnOnce(&[&str]) -> anyhow::Result<Vec<usize>>,
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

fn array_items_mut<'a>(
    doc: &'a mut Value,
    path: &[&str],
    id: &str,
) -> anyhow::Result<&'a mut Vec<Value>> {
    ensure_array_path_mut(doc, path, id)?
        .as_array_mut()
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected an array value",
                id,
            )
            .into()
        })
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

fn render_field(doc: &Value, spec: SimpleFieldSpec, id: &str) -> anyhow::Result<String> {
    let v = value_at_path(doc, spec.path);
    match spec.render {
        RenderMode::Scalar => Ok(render_scalar(v)),
        RenderMode::CsvStrings => render_string_array(v, ", ", id),
        RenderMode::LineStrings => render_string_array(v, "\n", id),
        RenderMode::TextLines { text_key } => render_text_lines(v, text_key, id),
        RenderMode::StatusLines {
            status_key,
            text_key,
        } => render_status_lines(v, status_key, text_key, id),
    }
}

fn value_at_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut cur = v;
    for key in path {
        cur = cur.get(*key)?;
    }
    Some(cur)
}

fn render_scalar(v: Option<&Value>) -> String {
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
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            "Expected an array value",
            id,
        )
        .into());
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

fn render_status_lines(
    v: Option<&Value>,
    status_key: &str,
    text_key: &str,
    id: &str,
) -> anyhow::Result<String> {
    let Some(v) = v else {
        return Ok(String::new());
    };
    let Some(items) = v.as_array() else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            "Expected an array value",
            id,
        )
        .into());
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(obj) = item.as_object() else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "Expected object entries in array",
                id,
            )
            .into());
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

fn render_text_lines(v: Option<&Value>, text_key: &str, id: &str) -> anyhow::Result<String> {
    let Some(v) = v else {
        return Ok(String::new());
    };
    let Some(items) = v.as_array() else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            "Expected an array value",
            id,
        )
        .into());
    };

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let Some(text) = item
            .as_object()
            .and_then(|obj| obj.get(text_key))
            .and_then(Value::as_str)
        else {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Expected object array items with '{text_key}' field"),
                id,
            )
            .into());
        };
        out.push(text.to_string());
    }
    Ok(out.join("\n"))
}

fn apply_set(doc: &mut Value, spec: SimpleSetSpec, value: &str, id: &str) -> anyhow::Result<()> {
    let slot = ensure_value_path_mut(doc, spec.path, id)?;

    match spec.mode {
        SetMode::String => *slot = Value::String(value.to_string()),
        SetMode::Integer => {
            let n: i64 = value.parse().map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0820InvalidFieldValue,
                    format!("Invalid integer value for {}: {value}", id),
                    id,
                )
            })?;
            *slot = Value::Number(serde_json::Number::from(n));
        }
        SetMode::OptionalString { empty_as_null } => {
            if empty_as_null && value.is_empty() {
                *slot = Value::Null;
            } else {
                *slot = Value::String(value.to_string());
            }
        }
        SetMode::Enum {
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

fn ensure_value_path_mut<'a>(
    mut cur: &'a mut Value,
    path: &[&str],
    id: &str,
) -> anyhow::Result<&'a mut Value> {
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
                    Value::Null
                } else {
                    Value::Object(serde_json::Map::new())
                },
            );
        }
        cur = obj.get_mut(*key).expect("inserted above");
    }
    Ok(cur)
}

fn ensure_array_path_mut<'a>(
    mut cur: &'a mut Value,
    path: &[&str],
    id: &str,
) -> anyhow::Result<&'a mut Value> {
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
                    Value::Array(Vec::new())
                } else {
                    Value::Object(serde_json::Map::new())
                },
            );
        }
        cur = obj.get_mut(*key).expect("inserted above");
    }
    Ok(cur)
}

// ===========================================================================
// Generic nested field operations (ADR-0031 V2)
// ===========================================================================

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

pub fn remove_nested_root_item(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    if rule.node.kind != NestedNodeKind::List {
        return Err(type_mismatch("Expected list root", id).into());
    }
    let root_value = ensure_node_path_mut(doc, rule.content_path, rule.node, id)?;
    let arr = root_value
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array at root path", id))?;
    let root_idx = path::require_index(&fp.segments[0], arr.len())?;
    let removed = arr.remove(root_idx);

    let text = rule
        .node
        .text_key
        .and_then(|key| removed.get(key).and_then(Value::as_str))
        .or_else(|| removed.as_str())
        .unwrap_or("<item>")
        .to_string();
    Ok(text)
}

fn descend_get<'a>(
    node: &'static NestedNodeRule,
    value: Option<&'a Value>,
    root_segment: &super::path::PathSegment,
    rest: &[super::path::PathSegment],
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
    rest: &[super::path::PathSegment],
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
    root_segment: &super::path::PathSegment,
    rest: &[super::path::PathSegment],
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
    rest: &[super::path::PathSegment],
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
        cur = obj.get_mut(*key).expect("inserted above");
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

fn render_nested_node(
    node: &'static NestedNodeRule,
    value: Option<&Value>,
    id: &str,
) -> anyhow::Result<String> {
    match node.kind {
        NestedNodeKind::Scalar => Ok(render_scalar(value)),
        NestedNodeKind::List => render_nested_list(node, value, id),
        NestedNodeKind::Object => render_nested_object(node, value, id),
    }
}

fn render_nested_list(
    node: &'static NestedNodeRule,
    value: Option<&Value>,
    id: &str,
) -> anyhow::Result<String> {
    let Some(value) = value else {
        return Ok(String::new());
    };
    let arr = value
        .as_array()
        .ok_or_else(|| type_mismatch("Expected array value", id))?;
    let item = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    if item.kind == NestedNodeKind::Scalar {
        let rendered: Vec<String> = arr.iter().map(|item| render_scalar(Some(item))).collect();
        return Ok(rendered.join("\n"));
    }
    let mut rendered = Vec::new();
    for item_value in arr {
        rendered.push(render_nested_node(item, Some(item_value), id)?);
    }
    Ok(rendered.join("\n\n"))
}

fn render_nested_object(
    node: &'static NestedNodeRule,
    value: Option<&Value>,
    id: &str,
) -> anyhow::Result<String> {
    let Some(value) = value else {
        return Ok(String::new());
    };
    let obj = value
        .as_object()
        .ok_or_else(|| type_mismatch("Expected object value", id))?;
    let mut lines = Vec::new();
    for field in node.fields {
        if let Some(field_value) = obj.get(field.name) {
            let rendered = if field.node.kind == NestedNodeKind::List
                && field
                    .node
                    .item
                    .is_some_and(|item| item.kind == NestedNodeKind::Scalar)
            {
                let items = field_value
                    .as_array()
                    .ok_or_else(|| type_mismatch("Expected array value", id))?;
                items
                    .iter()
                    .map(|item| render_scalar(Some(item)))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                render_nested_node(field.node, Some(field_value), id)?
            };
            lines.push(format!("{}: {}", field.name, rendered));
        }
    }
    Ok(lines.join("\n"))
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

fn type_mismatch(msg: &str, id: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, msg, id)
}

fn format_segment(seg: &super::path::PathSegment) -> String {
    match seg.index {
        Some(idx) => format!("{}[{idx}]", seg.name),
        None => seg.name.clone(),
    }
}

fn append_segment(prefix: &str, seg: &super::path::PathSegment) -> String {
    format!("{prefix}.{}", format_segment(seg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path(input: &str) -> FieldPath {
        path::parse_field_path(input)
            .expect("valid path")
            .collapse_legacy_prefixes()
    }

    #[test]
    fn test_add_nested_object_list_value_deduplicates_by_text() {
        let mut doc = json!({
            "content": {
                "consequences": {
                    "negative": [
                        { "text": "Higher memory use", "mitigations": [] }
                    ]
                }
            }
        });

        add_nested_list_value(
            ArtifactType::Adr,
            &mut doc,
            &path("consequences.negative"),
            "Higher memory use",
            "ADR-0001",
        )
        .unwrap();
        add_nested_list_value(
            ArtifactType::Adr,
            &mut doc,
            &path("consequences.negative"),
            "Slower warmup",
            "ADR-0001",
        )
        .unwrap();

        let negatives = doc["content"]["consequences"]["negative"]
            .as_array()
            .unwrap();
        assert_eq!(negatives.len(), 2);
        assert_eq!(negatives[1]["text"], "Slower warmup");
    }

    #[test]
    fn test_set_nested_field_rejects_list_path_without_index() {
        let mut doc = json!({
            "content": {
                "consequences": {
                    "negative": []
                }
            }
        });

        let err = set_nested_field(
            ArtifactType::Adr,
            &mut doc,
            &path("consequences.negative"),
            "oops",
            "ADR-0001",
        )
        .expect_err("list path should reject set");

        let diag = err.downcast_ref::<Diagnostic>().expect("diagnostic");
        assert_eq!(diag.code, DiagnosticCode::E0817PathTypeMismatch);
    }

    #[test]
    fn test_remove_nested_root_item_returns_removed_text() {
        let mut doc = json!({
            "content": {
                "alternatives": [
                    { "text": "Option A", "pros": [], "cons": [] },
                    { "text": "Option B", "pros": [], "cons": [] }
                ]
            }
        });

        let removed = remove_nested_root_item(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives[0]"),
            "ADR-0001",
        )
        .unwrap();

        assert_eq!(removed, "Option A");
        let alternatives = doc["content"]["alternatives"].as_array().unwrap();
        assert_eq!(alternatives.len(), 1);
        assert_eq!(alternatives[0]["text"], "Option B");
    }

    #[test]
    fn test_get_nested_field_renders_object_item_with_scalar_lists() {
        let doc = json!({
            "content": {
                "consequences": {
                    "negative": [
                        {
                            "text": "Higher memory use",
                            "mitigations": ["Cap cache size", "Reduce retention"]
                        }
                    ]
                }
            }
        });

        let rendered = get_nested_field(
            ArtifactType::Adr,
            &doc,
            &path("consequences.negative[0]"),
            "ADR-0001",
        )
        .unwrap();

        assert!(rendered.contains("text: Higher memory use"));
        assert!(rendered.contains("mitigations: Cap cache size, Reduce retention"));
    }
}
