mod list;
mod render;
mod traverse;

pub use self::list::{
    add_nested_list_value, remove_nested_list_values, set_nested_list_item,
    tick_nested_list_item_with_matcher,
};
use self::render::render_nested_node;
use self::traverse::{descend_get, descend_mut, ensure_node_path_mut};
use super::value_at_path;
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::path::FieldPath;
use crate::cmd::edit::rules::{
    self as edit_rules, NestedNodeKind, NestedRootRule, NestedScalarMode, Verb,
};
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use serde_json::Value;

fn resolve_nested_root(
    artifact: ArtifactType,
    root: &str,
    id: &str,
) -> anyhow::Result<&'static NestedRootRule> {
    edit_rules::nested_root_rule(artifact.rule_key(), root).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0815PathFieldNotFound,
            format!("Unknown nested root '{}' for {}", root, artifact.rule_key()),
            id,
        )
        .into()
    })
}

pub fn get_nested_field(
    artifact: ArtifactType,
    doc: &Value,
    fp: &FieldPath,
    id: &str,
) -> anyhow::Result<String> {
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
) -> anyhow::Result<()> {
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
        )
        .into()),
        NestedNodeKind::Object => Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            format!("Cannot set object path '{}' directly", fp),
            id,
        )
        .into()),
    }?;
    Ok(())
}

fn apply_nested_scalar_set(
    slot: &mut Value,
    mode: Option<NestedScalarMode>,
    value: &str,
    id: &str,
) -> anyhow::Result<()> {
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
                    return Err(Diagnostic::new(code, format!("{invalid_msg}: {value}"), id).into());
                }
                return Err(Diagnostic::new(
                    DiagnosticCode::E0820InvalidFieldValue,
                    format!("{invalid_msg}: {value}"),
                    id,
                )
                .into());
            }
            *slot = Value::String(value.to_string());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn path(input: &str) -> Result<FieldPath, Box<dyn std::error::Error>> {
        Ok(crate::cmd::edit::path::parse_field_path(input)?.collapse_legacy_prefixes())
    }

    #[test]
    fn test_add_nested_object_list_value_deduplicates_by_text()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = json!({
            "content": {
                "alternatives": [
                    { "text": "Option A", "status": "considered", "pros": [], "cons": [] }
                ]
            }
        });

        add_nested_list_value(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives")?,
            "Option A",
            "ADR-0001",
        )?;
        add_nested_list_value(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives")?,
            "Option B",
            "ADR-0001",
        )?;

        let alternatives = doc["content"]["alternatives"]
            .as_array()
            .ok_or("expected array")?;
        assert_eq!(alternatives.len(), 2);
        assert_eq!(alternatives[1]["text"], "Option B");
        Ok(())
    }

    #[test]
    fn test_set_nested_field_rejects_list_path_without_index()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut doc = json!({
            "content": {
                "alternatives": [
                    { "text": "Option A", "status": "considered", "pros": [], "cons": [] }
                ]
            }
        });

        let result = set_nested_field(
            ArtifactType::Adr,
            &mut doc,
            &path("alternatives[0].pros")?,
            "oops",
            "ADR-0001",
        );
        assert!(result.is_err());
        let err = result.err().ok_or("expected Err")?;
        let diag = err
            .downcast_ref::<Diagnostic>()
            .ok_or("expected Diagnostic")?;
        assert_eq!(diag.code, DiagnosticCode::E0817PathTypeMismatch);
        Ok(())
    }

    #[test]
    fn test_get_nested_field_renders_object_item_with_scalar_lists()
    -> Result<(), Box<dyn std::error::Error>> {
        let doc = json!({
            "content": {
                "alternatives": [
                    {
                        "text": "Option A",
                        "status": "accepted",
                        "pros": ["Readable", "Simple"],
                        "cons": ["More maintenance"],
                        "rejection_reason": null
                    }
                ]
            }
        });

        let rendered = get_nested_field(
            ArtifactType::Adr,
            &doc,
            &path("alternatives[0]")?,
            "ADR-0001",
        )?;

        assert!(rendered.contains("text: Option A"));
        assert!(rendered.contains("status: accepted"));
        assert!(rendered.contains("pros: Readable, Simple"));
        assert!(rendered.contains("cons: More maintenance"));
        Ok(())
    }
}
