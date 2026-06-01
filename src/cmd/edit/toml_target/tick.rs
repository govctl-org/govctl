use super::super::ArtifactType;
use super::super::engine as edit_engine;
use super::super::matching::{MatchOptions, MatchUse, resolve_match_indices};
use super::super::runtime as edit_runtime;
use crate::diagnostic::{Diagnostic, DiagnosticCode};

const TICK_NESTED_PATH_ERROR: &str =
    "tick only supports checklist root paths or indexed checklist items";

pub(super) fn tick_target_in_doc(
    artifact: ArtifactType,
    doc: &mut serde_json::Value,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    status_str: &str,
) -> anyhow::Result<String> {
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
                )
                .into());
            }
            match origin {
                edit_engine::TargetOrigin::Simple => {
                    let simple = path.as_simple().ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0901IoError,
                            "simple list target expected",
                            id,
                        )
                    })?;
                    edit_runtime::tick_simple_status_list_item_with_matcher(
                        artifact,
                        doc,
                        simple,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(id, simple, items, opts, MatchUse::TickSingle)
                        },
                    )?
                    .ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown field for tick: {simple}"),
                            id,
                        )
                        .into()
                    })
                }
                edit_engine::TargetOrigin::Nested => {
                    let display = path.to_string();
                    edit_runtime::tick_nested_list_item_with_matcher(
                        artifact,
                        doc,
                        path,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(id, &display, items, opts, MatchUse::TickSingle)
                        },
                    )
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
                )
                .into());
            }
            let exact = MatchOptions {
                pattern: None,
                at: Some(*index),
                exact: false,
                regex: false,
                all: false,
            };
            match origin {
                edit_engine::TargetOrigin::Simple => {
                    let simple = container_path.as_simple().ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0901IoError,
                            "simple indexed container expected",
                            id,
                        )
                    })?;
                    edit_runtime::tick_simple_status_list_item_with_matcher(
                        artifact,
                        doc,
                        simple,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(id, simple, items, &exact, MatchUse::TickSingle)
                        },
                    )?
                    .ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::E0803UnknownField,
                            format!("Unknown field for tick: {simple}"),
                            id,
                        )
                        .into()
                    })
                }
                edit_engine::TargetOrigin::Nested => {
                    edit_runtime::tick_nested_list_item_with_matcher(
                        artifact,
                        doc,
                        container_path,
                        id,
                        status_str,
                        |items| {
                            resolve_match_indices(
                                id,
                                &container_path.to_string(),
                                items,
                                &exact,
                                MatchUse::TickSingle,
                            )
                        },
                    )
                }
            }
        }
        _ => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            TICK_NESTED_PATH_ERROR,
            id,
        )
        .into()),
    }
}
