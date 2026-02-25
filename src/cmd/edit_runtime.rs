use crate::cmd::edit::ArtifactType;
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
