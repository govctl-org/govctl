mod list;
mod render;
mod traverse;

pub use self::list::{
    add_nested_list_value, remove_nested_list_values, set_nested_list_item,
    tick_nested_list_item_with_matcher,
};
use self::render::render_nested_node;
use self::traverse::{descend_get, descend_mut, ensure_node_path_mut};
use super::support::value_at_path;
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::path::FieldPath;
use crate::cmd::edit::rules::{
    self as edit_rules, NestedNodeKind, NestedRootRule, NestedScalarMode, Verb,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

fn resolve_nested_root(
    artifact: ArtifactType,
    root: &str,
    id: &str,
) -> DiagnosticResult<&'static NestedRootRule> {
    edit_rules::nested_root_rule(artifact.rule_key(), root).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0815PathFieldNotFound,
            format!("Unknown nested root '{}' for {}", root, artifact.rule_key()),
            id,
        )
    })
}

pub fn get_nested_field(
    artifact: ArtifactType,
    doc: &Value,
    fp: &FieldPath,
    id: &str,
) -> DiagnosticResult<String> {
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
) -> DiagnosticResult<()> {
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
        )),
        NestedNodeKind::Object => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot set object path '{}' directly", fp),
            id,
        )),
    }?;
    Ok(())
}

fn apply_nested_scalar_set(
    slot: &mut Value,
    mode: Option<NestedScalarMode>,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    match mode.unwrap_or(NestedScalarMode::String) {
        NestedScalarMode::String => *slot = Value::String(value.to_string()),
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
                    return Err(Diagnostic::new(code, format!("{invalid_msg}: {value}"), id));
                }
                return Err(Diagnostic::new(
                    DiagnosticCode::E0820InvalidFieldValue,
                    format!("{invalid_msg}: {value}"),
                    id,
                ));
            }
            *slot = Value::String(value.to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
