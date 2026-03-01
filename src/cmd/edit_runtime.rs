use crate::cmd::edit::ArtifactType;
use crate::cmd::edit_rules::{self, FieldKind, NestedRootRule, Verb};
use crate::cmd::path::{self, FieldPath};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
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
    OptionalString {
        empty_as_null: bool,
    },
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
    };
    Diagnostic::new(DiagnosticCode::E0803UnknownField, msg, id)
}

fn render_field(doc: &Value, spec: SimpleFieldSpec, id: &str) -> anyhow::Result<String> {
    let v = value_at_path(doc, spec.path);
    match spec.render {
        RenderMode::Scalar => Ok(render_scalar(v)),
        RenderMode::CsvStrings => render_string_array(v, ", ", id),
        RenderMode::LineStrings => render_string_array(v, "\n", id),
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

fn apply_set(doc: &mut Value, spec: SimpleSetSpec, value: &str, id: &str) -> anyhow::Result<()> {
    let slot = value_at_path_mut(doc, spec.path).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot resolve field path '{}'", spec.path.join(".")),
            id,
        )
    })?;

    match spec.mode {
        SetMode::String => *slot = Value::String(value.to_string()),
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
                return Err(anyhow::anyhow!("{invalid_msg}: {value}"));
            }
            *slot = Value::String(value.to_string());
        }
    }

    Ok(())
}

fn value_at_path_mut<'a>(v: &'a mut Value, path: &[&str]) -> Option<&'a mut Value> {
    let mut cur = v;
    for key in path {
        cur = cur.get_mut(*key)?;
    }
    Some(cur)
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

/// Resolve a nested root rule from the SSOT, returning an error if not found.
fn resolve_nested_root(
    artifact: ArtifactType,
    root: &str,
    id: &str,
) -> anyhow::Result<&'static NestedRootRule> {
    edit_rules::nested_root_rule(artifact.rule_key(), root).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0815PathFieldNotFound,
            format!(
                "Unknown nested root '{}' for {}",
                root,
                artifact.rule_key()
            ),
            id,
        )
        .into()
    })
}

/// Validate a nested field path against SSOT rules for a given verb.
///
/// Validates depth, index requirements, and subfield verb support.
/// Returns the resolved root index.
fn validate_nested_path(
    artifact: ArtifactType,
    rule: &NestedRootRule,
    fp: &FieldPath,
    verb: Verb,
    root_array_len: usize,
    id: &str,
) -> anyhow::Result<usize> {
    let root_name = &fp.segments[0].name;

    // Validate depth
    if fp.segments.len() > rule.max_depth {
        return Err(Diagnostic::new(
            DiagnosticCode::E0814InvalidPath,
            format!(
                "Path '{}' exceeds max depth {} for {}.{}",
                fp,
                rule.max_depth,
                artifact.rule_key(),
                root_name
            ),
            id,
        )
        .into());
    }

    // Resolve root index
    let root_idx = path::require_index(&fp.segments[0], root_array_len)?;

    // Validate subfield if present
    if fp.segments.len() >= 2 {
        let subfield = &fp.segments[1].name;
        let field_rule =
            edit_rules::nested_field_rule(artifact.rule_key(), root_name, subfield).ok_or_else(
                || {
                    Diagnostic::new(
                        DiagnosticCode::E0815PathFieldNotFound,
                        format!(
                            "Unknown field '{}' under {}.{}",
                            subfield,
                            artifact.rule_key(),
                            root_name
                        ),
                        id,
                    )
                },
            )?;
        if !edit_rules::nested_field_supports_verb(
            artifact.rule_key(),
            root_name,
            subfield,
            verb,
        ) {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!(
                    "Field '{}.{}' does not support verb '{}'",
                    root_name,
                    subfield,
                    verb.as_str()
                ),
                id,
            )
            .into());
        }
        // Ensure unused binding is consumed for the existence check
        let _ = field_rule;
    }

    Ok(root_idx)
}

/// Navigate to the root array in a JSON document using the SSOT content_path.
fn root_array<'a>(doc: &'a Value, rule: &NestedRootRule, id: &str) -> anyhow::Result<&'a Vec<Value>> {
    value_at_path(doc, rule.content_path)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Expected array at path '{}'", rule.content_path.join(".")),
                id,
            )
            .into()
        })
}

/// Navigate to the root array (mutable) in a JSON document.
fn root_array_mut<'a>(
    doc: &'a mut Value,
    rule: &NestedRootRule,
    id: &str,
) -> anyhow::Result<&'a mut Vec<Value>> {
    ensure_array_path_mut(doc, rule.content_path, id)?
        .as_array_mut()
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!("Expected array at path '{}'", rule.content_path.join(".")),
                id,
            )
            .into()
        })
}

/// GET a nested field value. Handles:
/// - `root[i]` → render all fields of the item
/// - `root[i].scalar` → render scalar value
/// - `root[i].list` → render all list items
/// - `root[i].list[j]` → render one list item
pub fn get_nested_field(
    artifact: ArtifactType,
    doc: &Value,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let arr = root_array(doc, rule, id)?;
    let root_idx = validate_nested_path(artifact, rule, fp, Verb::Get, arr.len(), id)?;
    let item = &arr[root_idx];

    if fp.segments.len() == 1 {
        // Render whole item
        return render_nested_item(item, rule, id);
    }

    let subfield = &fp.segments[1].name;
    let field_rule =
        edit_rules::nested_field_rule(artifact.rule_key(), root_name, subfield).unwrap();

    match field_rule.kind {
        FieldKind::Scalar => {
            if fp.segments[1].index.is_some() {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    format!("Cannot index into scalar field '{subfield}'"),
                    id,
                )
                .into());
            }
            Ok(render_scalar(item.get(subfield)))
        }
        FieldKind::List => {
            let list = item
                .get(subfield)
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Some(sub_idx) = fp.segments[1].index {
                let resolved = path::resolve_index(sub_idx, list.len())?;
                Ok(render_scalar(list.get(resolved)))
            } else {
                let strs: Vec<String> = list
                    .iter()
                    .map(|v| render_scalar(Some(v)))
                    .collect();
                Ok(strs.join("\n"))
            }
        }
    }
}

/// SET a nested field value. Handles:
/// - `root[i].scalar "value"` → set scalar field
/// - `root[i].list[j] "value"` → replace list item at index
pub fn set_nested_field(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let arr = root_array_mut(doc, rule, id)?;
    let root_idx = validate_nested_path(artifact, rule, fp, Verb::Set, arr.len(), id)?;

    if fp.segments.len() < 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!(
                "Cannot set entire '{}' item directly; specify a subfield",
                root_name
            ),
            id,
        )
        .into());
    }

    let subfield = &fp.segments[1].name;
    let field_rule =
        edit_rules::nested_field_rule(artifact.rule_key(), root_name, subfield).unwrap();
    let item = arr[root_idx]
        .as_object_mut()
        .ok_or_else(|| type_mismatch("Expected object item in array", id))?;

    match field_rule.kind {
        FieldKind::Scalar => {
            // For optional fields (current value is null), empty → null
            let json_value = if value.is_empty()
                && item
                    .get(subfield)
                    .is_none_or(|v| v.is_null())
            {
                Value::Null
            } else {
                Value::String(value.to_string())
            };
            item.insert(subfield.to_string(), json_value);
        }
        FieldKind::List => {
            let sub_idx = fp.segments[1].index.ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    format!("Field '{subfield}' is a list; use an index to set a specific item, or use 'add'/'remove'"),
                    id,
                )
            })?;
            let list = item
                .entry(subfield)
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .ok_or_else(|| type_mismatch("Expected array for list field", id))?;
            let resolved = path::resolve_index(sub_idx, list.len())?;
            list[resolved] = Value::String(value.to_string());
        }
    }
    Ok(())
}

/// ADD a value to a nested list field: `root[i].list "value"`.
pub fn add_nested_list_value(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let arr = root_array_mut(doc, rule, id)?;
    let root_idx = validate_nested_path(artifact, rule, fp, Verb::Add, arr.len(), id)?;

    if fp.segments.len() < 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot add to '{}' without specifying a list subfield", root_name),
            id,
        )
        .into());
    }

    // Reject indexed terminal paths: e.g., alt[0].pros[999]
    if fp.segments[1].index.is_some() {
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

    let subfield = &fp.segments[1].name;
    let field_rule =
        edit_rules::nested_field_rule(artifact.rule_key(), root_name, subfield).unwrap();
    if field_rule.kind != FieldKind::List {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Field '{subfield}' is not a list; cannot add to it"),
            id,
        )
        .into());
    }

    let item = arr[root_idx]
        .as_object_mut()
        .ok_or_else(|| type_mismatch("Expected object item in array", id))?;
    let list = item
        .entry(subfield)
        .or_insert_with(|| Value::Array(Vec::new()))
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;

    // Duplicate check
    if !list.iter().any(|v| v.as_str() == Some(value)) {
        list.push(Value::String(value.to_string()));
    }
    Ok(())
}

/// REMOVE values from a nested list field: `root[i].list` with matcher.
/// Returns the removed values.
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
    let arr = root_array_mut(doc, rule, id)?;
    let root_idx = validate_nested_path(artifact, rule, fp, Verb::Remove, arr.len(), id)?;

    if fp.segments.len() < 2 {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot remove from '{}' without specifying a list subfield", root_name),
            id,
        )
        .into());
    }

    let subfield = &fp.segments[1].name;
    let item = arr[root_idx]
        .as_object_mut()
        .ok_or_else(|| type_mismatch("Expected object item in array", id))?;
    let list = item
        .get_mut(subfield)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| type_mismatch("Expected array for list field", id))?;

    let texts: Vec<&str> = list
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
        .collect::<Result<Vec<_>, _>>()?;

    let indices = resolve(&texts)?;
    let mut sorted = indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    let mut removed = Vec::with_capacity(sorted.len());
    for idx in sorted {
        let val = list.remove(idx);
        removed.push(val.as_str().unwrap_or_default().to_string());
    }
    removed.reverse();
    Ok(removed)
}

/// REMOVE an entire item from the root array by index: `root[i]`.
/// Returns a display text for the removed item.
pub fn remove_nested_root_item(
    artifact: ArtifactType,
    doc: &mut Value,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
    let root_name = &fp.segments[0].name;
    let rule = resolve_nested_root(artifact, root_name, id)?;
    let arr = root_array_mut(doc, rule, id)?;
    let root_idx = path::require_index(&fp.segments[0], arr.len())?;
    let removed = arr.remove(root_idx);

    // Extract display text using SSOT text_key, falling back to bare string items
    let text = rule
        .text_key
        .and_then(|key| removed.get(key).and_then(Value::as_str))
        .or_else(|| removed.as_str())
        .unwrap_or("<item>")
        .to_string();
    Ok(text)
}

/// Render all fields of a nested object item for display.
fn render_nested_item(item: &Value, rule: &NestedRootRule, _id: &str) -> anyhow::Result<String> {
    let mut lines = Vec::new();
    if let Some(obj) = item.as_object() {
        for field in rule.fields {
            if let Some(val) = obj.get(field.name) {
                match field.kind {
                    FieldKind::Scalar => {
                        lines.push(format!("{}: {}", field.name, render_scalar(Some(val))));
                    }
                    FieldKind::List => {
                        if let Some(arr) = val.as_array() {
                            let items: Vec<&str> = arr
                                .iter()
                                .filter_map(Value::as_str)
                                .collect();
                            lines.push(format!("{}: {}", field.name, items.join(", ")));
                        }
                    }
                }
            }
        }
    } else if let Some(s) = item.as_str() {
        // Simple string items (e.g., notes)
        lines.push(s.to_string());
    }
    Ok(lines.join("\n"))
}

fn type_mismatch(msg: &str, id: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, msg, id)
}
