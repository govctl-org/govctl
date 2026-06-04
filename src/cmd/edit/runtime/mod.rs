mod list;
mod mutate;
mod nested;
mod render;
mod spec;
mod support;

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
use self::spec::{
    RenderMode, RuntimeFieldEntry, SetMode, SimpleFieldSpec, SimpleSetSpec, StatusListSpec,
};
use self::support::unknown_field_error;
use super::ArtifactType;
use crate::diagnostic::{DiagnosticCode, DiagnosticResult};
use serde_json::Value;

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
