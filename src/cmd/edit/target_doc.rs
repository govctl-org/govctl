use super::ArtifactType;
use super::engine as edit_engine;
use super::runtime as edit_runtime;
use super::unexpected_edit_state;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

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
