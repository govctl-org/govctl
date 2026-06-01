use super::ArtifactType;
use super::engine as edit_engine;
use super::matching::{MatchOptions, MatchUse, resolve_match_indices};
use super::runtime as edit_runtime;
use super::unexpected_edit_state;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::ui;
use crate::write::WriteOp;

#[derive(Debug, Clone, Copy)]
pub(super) enum NestedGetMode<'a> {
    Allow,
    Reject(&'a str),
}

pub(super) fn cannot_add_to_field_error(id: &str, field: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0810CannotAddToField,
        format!("Cannot add to field: {field} (not an array or unsupported)"),
        id,
    )
}

fn cannot_remove_from_field_error(id: &str, field: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0811CannotRemoveFromField,
        format!("Cannot remove from field: {field}"),
        id,
    )
}

pub(super) fn render_target_from_doc(
    artifact: ArtifactType,
    doc: &serde_json::Value,
    target: &edit_engine::ResolvedTarget,
    id: &str,
    nested: NestedGetMode<'_>,
) -> DiagnosticResult<String> {
    match target {
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Simple,
            path,
            ..
        } => {
            let simple = simple_get_path(
                path,
                id,
                nested,
                "simple node target should use a simple path",
            )?;
            edit_runtime::get_simple_field(artifact, doc, simple, id)
        }
        edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Simple,
            container_path,
            index,
            ..
        } => {
            let simple = simple_get_path(
                container_path,
                id,
                nested,
                "simple indexed target should use a simple container path",
            )?;
            edit_runtime::get_simple_list_item(artifact, doc, simple, *index, id)
        }
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Nested,
            path,
            ..
        }
        | edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Nested,
            path,
            ..
        } => match nested {
            NestedGetMode::Allow => edit_runtime::get_nested_field(artifact, doc, path, id),
            NestedGetMode::Reject(message) => Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                message,
                id,
            )),
        },
    }
}

fn simple_get_path<'a>(
    path: &'a super::path::FieldPath,
    id: &str,
    nested: NestedGetMode<'_>,
    unexpected_message: &str,
) -> DiagnosticResult<&'a str> {
    path.as_simple().ok_or_else(|| match nested {
        NestedGetMode::Allow => unexpected_edit_state(id, unexpected_message),
        NestedGetMode::Reject(message) => {
            Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, message, id)
        }
    })
}

pub(super) fn add_to_target_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let edit_engine::ResolvedTarget::Node {
        path,
        kind: edit_engine::TargetKind::List,
        origin,
        ..
    } = target
    else {
        return match target {
            edit_engine::ResolvedTarget::IndexedItem { .. } => Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                format!(
                    "Cannot add to indexed path '{}' (use set/remove for a specific element)",
                    target.display_path()
                ),
                id,
            )),
            _ => Err(cannot_add_to_field_error(id, &target.display_path())),
        };
    };

    match origin {
        edit_engine::TargetOrigin::Simple => {
            let simple = path
                .as_simple()
                .ok_or_else(|| unexpected_edit_state(id, "simple list target expected"))?;
            if !edit_runtime::add_simple_list_value(artifact, doc, simple, value, id)? {
                return Err(cannot_add_to_field_error(id, simple));
            }
        }
        edit_engine::TargetOrigin::Nested => {
            edit_runtime::add_nested_list_value(artifact, doc, path, value, id)?;
        }
    }

    Ok(())
}

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
            edit_engine::TargetOrigin::Simple => {
                let simple = path
                    .as_simple()
                    .ok_or_else(|| unexpected_edit_state(id, "simple list target expected"))?;
                let removed = remove_simple_values_from_doc(artifact, doc, simple, id, opts)?
                    .ok_or_else(|| cannot_remove_from_field_error(id, simple))?;
                Ok((simple.to_string(), removed))
            }
            edit_engine::TargetOrigin::Nested => {
                let display = path.to_string();
                let removed =
                    edit_runtime::remove_nested_list_values(artifact, doc, path, id, |items| {
                        resolve_match_indices(id, &display, items, opts, MatchUse::Remove)
                    })?;
                Ok((display, removed))
            }
        },
        edit_engine::ResolvedTarget::IndexedItem {
            container_path,
            index,
            origin,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => {
                let simple = container_path.as_simple().ok_or_else(|| {
                    unexpected_edit_state(id, "simple indexed container expected")
                })?;
                let exact = MatchOptions::at_index(*index);
                let removed = remove_simple_values_from_doc(artifact, doc, simple, id, &exact)?
                    .ok_or_else(|| cannot_remove_from_field_error(id, simple))?;
                Ok((simple.to_string(), removed))
            }
            edit_engine::TargetOrigin::Nested => {
                let display = container_path.to_string();
                let removed = edit_runtime::remove_nested_list_values(
                    artifact,
                    doc,
                    container_path,
                    id,
                    |items| {
                        let resolved = super::path::resolve_index(*index, items.len())?;
                        Ok(vec![resolved])
                    },
                )?;
                Ok((display, removed))
            }
        },
        _ => Err(cannot_remove_from_field_error(id, &target.display_path())),
    }
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
