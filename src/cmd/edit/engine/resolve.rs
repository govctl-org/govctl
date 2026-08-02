use super::{ResolvedTarget, TargetKind, TargetOrigin};
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::path::{FieldPath, PathSegment};
use crate::cmd::edit::rules::{
    self as edit_rules, FieldKind, NestedNodeKind, NestedNodeRule, Verb,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

const SIMPLE_LIST_ITEM_VERBS: &[&str] = &["get", "set"];

pub(super) fn resolve_target(
    artifact: ArtifactType,
    fp: &FieldPath,
    id: &str,
) -> DiagnosticResult<ResolvedTarget> {
    let root = &fp.segments[0].name;
    if let Some(rule) = edit_rules::nested_root_rule(artifact.rule_key(), root) {
        return resolve_nested_target(rule.node, fp, id);
    }

    if fp.segments.len() != 1 {
        return Err(unknown_field_error(artifact, &fp.to_string(), id));
    }

    let seg = &fp.segments[0];
    let Some(rule) = edit_rules::simple_field_rule(artifact.rule_key(), &seg.name) else {
        return Err(unknown_field_error(artifact, &seg.name, id));
    };

    match (rule.kind, seg.index) {
        (FieldKind::Scalar, None) => Ok(ResolvedTarget::Node {
            origin: TargetOrigin::Simple,
            path: fp.clone(),
            kind: TargetKind::Scalar,
            status_list: false,
            verbs: rule.verbs,
        }),
        (FieldKind::Scalar, Some(_)) => Err(path_type_error(
            id,
            format!(
                "Cannot index into non-list field '{}' at '{}'",
                seg.name, fp
            ),
        )),
        (FieldKind::List, None) => Ok(ResolvedTarget::Node {
            origin: TargetOrigin::Simple,
            path: fp.clone(),
            kind: TargetKind::List,
            status_list: edit_rules::simple_field_supports_verb(
                artifact.rule_key(),
                &seg.name,
                Verb::Tick,
            ),
            verbs: rule.verbs,
        }),
        (FieldKind::List, Some(index)) => Ok(ResolvedTarget::IndexedItem {
            origin: TargetOrigin::Simple,
            path: fp.clone(),
            container_path: FieldPath {
                segments: vec![PathSegment {
                    name: seg.name.clone(),
                    index: None,
                }],
            },
            index,
            item_kind: TargetKind::Scalar,
            object_set_mode: None,
            status_list: edit_rules::simple_field_supports_verb(
                artifact.rule_key(),
                &seg.name,
                Verb::Tick,
            ),
            container_verbs: rule.verbs,
            item_verbs: SIMPLE_LIST_ITEM_VERBS,
        }),
    }
}

fn resolve_nested_target(
    mut current_node: &'static NestedNodeRule,
    fp: &FieldPath,
    id: &str,
) -> DiagnosticResult<ResolvedTarget> {
    let mut container_segments = Vec::with_capacity(fp.segments.len());

    for (idx, seg) in fp.segments.iter().enumerate() {
        let is_last = idx + 1 == fp.segments.len();

        if idx > 0 {
            if current_node.kind != NestedNodeKind::Object {
                return Err(path_type_error(
                    id,
                    format!("Cannot descend into non-object path '{}'", fp),
                ));
            }
            let child = current_node
                .fields
                .iter()
                .find(|field| field.name == seg.name)
                .ok_or_else(|| {
                    path_field_not_found(
                        id,
                        seg.name.as_str(),
                        &FieldPath {
                            segments: container_segments.clone(),
                        },
                        current_node,
                    )
                })?;
            current_node = child.node;
        }

        let mut container_seg = seg.clone();
        container_seg.index = None;
        container_segments.push(container_seg);

        if let Some(index) = seg.index {
            if current_node.kind != NestedNodeKind::List {
                return Err(path_type_error(
                    id,
                    format!(
                        "Cannot index into non-list field '{}' at '{}'",
                        seg.name, fp
                    ),
                ));
            }
            if is_last {
                let item_node = current_node
                    .item
                    .ok_or_else(|| path_type_error(id, "List node missing item rule"))?;
                return Ok(ResolvedTarget::IndexedItem {
                    origin: TargetOrigin::Nested,
                    path: fp.clone(),
                    container_path: FieldPath {
                        segments: container_segments,
                    },
                    index,
                    item_kind: map_nested_kind(item_node.kind),
                    object_set_mode: item_node.object_set_mode,
                    status_list: edit_rules::nested_status_list_spec(current_node).is_some(),
                    container_verbs: current_node.verbs,
                    item_verbs: item_node.verbs,
                });
            }
            current_node = current_node
                .item
                .ok_or_else(|| path_type_error(id, "List node missing item rule"))?;
            if let Some(last) = container_segments.last_mut() {
                last.index = Some(index);
            }
        } else if is_last {
            return Ok(ResolvedTarget::Node {
                origin: TargetOrigin::Nested,
                path: fp.clone(),
                kind: map_nested_kind(current_node.kind),
                status_list: edit_rules::nested_status_list_spec(current_node).is_some(),
                verbs: current_node.verbs,
            });
        }
    }

    Err(path_type_error(
        id,
        format!("Could not resolve path target '{}'", fp),
    ))
}

fn map_nested_kind(kind: NestedNodeKind) -> TargetKind {
    match kind {
        NestedNodeKind::Scalar => TargetKind::Scalar,
        NestedNodeKind::List => TargetKind::List,
        NestedNodeKind::Object => TargetKind::Object,
    }
}

fn path_type_error(id: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, message, id)
}

fn path_field_not_found(
    id: &str,
    field: &str,
    parent: &FieldPath,
    node: &NestedNodeRule,
) -> Diagnostic {
    let known = edit_rules::nested_child_names(node).join(", ");
    Diagnostic::new(
        DiagnosticCode::E0815PathFieldNotFound,
        format!("Unknown nested field '{field}' under '{parent}'. Known fields: {known}"),
        id,
    )
}

fn unknown_field_error(artifact: ArtifactType, field: &str, id: &str) -> Diagnostic {
    let known = edit_rules::root_field_names(artifact.rule_key()).join(", ");
    let (code, msg, source) = match artifact {
        ArtifactType::Rfc => (
            DiagnosticCode::E0101RfcSchemaInvalid,
            format!("Unknown field: {field}. Known fields: {known}"),
            "",
        ),
        ArtifactType::Clause => (
            DiagnosticCode::E0201ClauseSchemaInvalid,
            format!("Unknown field: {field}. Known fields: {known}"),
            "",
        ),
        ArtifactType::Adr => (
            DiagnosticCode::E0803UnknownField,
            format!("Unknown ADR field: {field}. Known fields: {known}"),
            id,
        ),
        ArtifactType::WorkItem => (
            DiagnosticCode::E0803UnknownField,
            format!("Unknown work item field: {field}. Known fields: {known}"),
            id,
        ),
        ArtifactType::Guard => (
            DiagnosticCode::E0803UnknownField,
            format!("Unknown guard field: {field}. Known fields: {known}"),
            id,
        ),
        ArtifactType::Conformance => (
            DiagnosticCode::E0803UnknownField,
            format!("Unknown Conformance Case field: {field}. Known fields: {known}"),
            id,
        ),
    };
    Diagnostic::new(code, msg, source)
}
