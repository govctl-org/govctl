use super::ArtifactType;
use super::engine as edit_engine;
use super::matching::{MatchOptions, MatchUse, resolve_match_indices};
use super::runtime as edit_runtime;
use super::unexpected_edit_state;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use crate::write::WriteOp;

pub(super) fn notify_removed(id: &str, field: &str, removed: &[String], op: WriteOp) {
    if !op.is_preview() {
        for item in removed {
            ui::field_removed(id, field, item);
        }
    }
}

pub(super) fn remove_target_from_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
) -> DiagnosticResult<(String, Vec<String>)> {
    match target {
        edit_engine::ResolvedTarget::Node {
            path,
            kind: edit_engine::TargetKind::List,
            origin,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => remove_simple_target_values(
                artifact,
                doc,
                id,
                path,
                opts,
                "simple list target expected",
            ),
            edit_engine::TargetOrigin::Nested => {
                remove_nested_target_values(artifact, doc, id, path, |display, items| {
                    resolve_match_indices(id, display, items, opts, MatchUse::Remove)
                })
            }
        },
        edit_engine::ResolvedTarget::IndexedItem {
            container_path,
            index,
            origin,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => {
                let exact = MatchOptions::at_index(*index);
                remove_simple_target_values(
                    artifact,
                    doc,
                    id,
                    container_path,
                    &exact,
                    "simple indexed container expected",
                )
            }
            edit_engine::TargetOrigin::Nested => {
                remove_nested_target_values(artifact, doc, id, container_path, |_display, items| {
                    let resolved = super::path::resolve_index(*index, items.len())?;
                    Ok(vec![resolved])
                })
            }
        },
        _ => Err(cannot_remove_from_field_error(id, &target.display_path())),
    }
}

fn cannot_remove_from_field_error(id: &str, field: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0811CannotRemoveFromField,
        format!("Cannot remove from field: {field}"),
        id,
    )
}

fn remove_simple_target_values(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    path: &super::path::FieldPath,
    opts: &MatchOptions,
    unexpected_message: &str,
) -> DiagnosticResult<(String, Vec<String>)> {
    let simple = path
        .as_simple()
        .ok_or_else(|| unexpected_edit_state(id, unexpected_message))?;
    let removed = remove_simple_values_from_doc(artifact, doc, simple, id, opts)?
        .ok_or_else(|| cannot_remove_from_field_error(id, simple))?;
    Ok((simple.to_string(), removed))
}

fn remove_nested_target_values<F>(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    path: &super::path::FieldPath,
    resolve: F,
) -> DiagnosticResult<(String, Vec<String>)>
where
    F: FnOnce(&str, &[&str]) -> DiagnosticResult<Vec<usize>>,
{
    let display = path.to_string();
    let removed = edit_runtime::remove_nested_list_values(artifact, doc, path, id, |items| {
        resolve(&display, items)
    })?;
    Ok((display, removed))
}

fn remove_simple_values_from_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    field: &str,
    id: &str,
    opts: &MatchOptions,
) -> DiagnosticResult<Option<Vec<String>>> {
    if let Some(removed) =
        edit_runtime::remove_simple_list_values_with_matcher(artifact, doc, field, id, |items| {
            resolve_match_indices(id, field, items, opts, MatchUse::Remove)
        })?
    {
        return Ok(Some(removed));
    }
    edit_runtime::remove_simple_status_list_values_with_matcher(artifact, doc, field, id, |items| {
        resolve_match_indices(id, field, items, opts, MatchUse::Remove)
    })
}
