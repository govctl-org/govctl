use super::super::engine as edit_engine;
use super::super::matching::{MatchOptions, MatchUse, resolve_match_indices};
use super::super::runtime as edit_runtime;
use super::super::{ArtifactType, unexpected_edit_state};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

const TICK_NESTED_PATH_ERROR: &str =
    "tick only supports checklist root paths or indexed checklist items";

pub(super) fn tick_target_in_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    status_str: &str,
) -> DiagnosticResult<String> {
    match target {
        edit_engine::ResolvedTarget::Node {
            path,
            kind: edit_engine::TargetKind::List,
            origin,
            status_list,
        } => {
            if !status_list {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    TICK_NESTED_PATH_ERROR,
                    id,
                ));
            }
            match origin {
                edit_engine::TargetOrigin::Simple => {
                    let simple = path
                        .as_simple()
                        .ok_or_else(|| unexpected_edit_state(id, "simple list target expected"))?;
                    tick_simple_target(artifact, doc, id, simple, opts, status_str)
                }
                edit_engine::TargetOrigin::Nested => {
                    let display = path.to_string();
                    tick_nested_target(artifact, doc, id, path, &display, opts, status_str)
                }
            }
        }
        edit_engine::ResolvedTarget::IndexedItem {
            container_path,
            index,
            origin,
            status_list,
            ..
        } => {
            if !status_list {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    TICK_NESTED_PATH_ERROR,
                    id,
                ));
            }
            let exact = MatchOptions::at_index(*index);
            match origin {
                edit_engine::TargetOrigin::Simple => {
                    let simple = container_path.as_simple().ok_or_else(|| {
                        unexpected_edit_state(id, "simple indexed container expected")
                    })?;
                    tick_simple_target(artifact, doc, id, simple, &exact, status_str)
                }
                edit_engine::TargetOrigin::Nested => {
                    let display = container_path.to_string();
                    tick_nested_target(
                        artifact,
                        doc,
                        id,
                        container_path,
                        &display,
                        &exact,
                        status_str,
                    )
                }
            }
        }
        _ => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            TICK_NESTED_PATH_ERROR,
            id,
        )),
    }
}

fn tick_simple_target(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    status_str: &str,
) -> DiagnosticResult<String> {
    edit_runtime::tick_simple_status_list_item_with_matcher(
        artifact,
        doc,
        field,
        id,
        status_str,
        |items| resolve_match_indices(id, field, items, opts, MatchUse::TickSingle),
    )?
    .ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0803UnknownField,
            format!("Unknown field for tick: {field}"),
            id,
        )
    })
}

fn tick_nested_target(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    path: &crate::cmd::edit::path::FieldPath,
    display: &str,
    opts: &MatchOptions,
    status_str: &str,
) -> DiagnosticResult<String> {
    edit_runtime::tick_nested_list_item_with_matcher(artifact, doc, path, id, status_str, |items| {
        resolve_match_indices(id, display, items, opts, MatchUse::TickSingle)
    })
}
