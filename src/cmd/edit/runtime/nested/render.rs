use super::super::render::{render_scalar, render_status_lines};
use super::super::support::type_mismatch;
use crate::cmd::edit::rules::{NestedNodeKind, NestedNodeRule};
use crate::diagnostic::DiagnosticResult;
use serde_json::Value;

pub(super) fn render_nested_node(
    node: &'static NestedNodeRule,
    value: Option<&Value>,
    id: &str,
) -> DiagnosticResult<String> {
    match node.kind {
        NestedNodeKind::Scalar => Ok(render_scalar(value)),
        NestedNodeKind::List => render_nested_list(node, value, id),
        NestedNodeKind::Object => render_nested_object(node, value, id),
    }
}

fn render_nested_list(
    node: &'static NestedNodeRule,
    value: Option<&Value>,
    id: &str,
) -> DiagnosticResult<String> {
    let Some(value) = value else {
        return Ok(String::new());
    };
    let arr = value
        .as_array()
        .ok_or_else(|| type_mismatch("Expected array value", id))?;
    let item = node
        .item
        .ok_or_else(|| type_mismatch("List node missing item rule", id))?;
    if item.kind == NestedNodeKind::Object
        && node.text_key.is_some()
        && item.fields.iter().any(|field| field.name == "status")
    {
        return render_status_lines(Some(value), "status", node.text_key.unwrap_or("text"), id);
    }
    if item.kind == NestedNodeKind::Scalar {
        let rendered: Vec<String> = arr.iter().map(|item| render_scalar(Some(item))).collect();
        return Ok(rendered.join("\n"));
    }
    let mut rendered = Vec::new();
    for item_value in arr {
        rendered.push(render_nested_node(item, Some(item_value), id)?);
    }
    Ok(rendered.join("\n\n"))
}

fn render_nested_object(
    node: &'static NestedNodeRule,
    value: Option<&Value>,
    id: &str,
) -> DiagnosticResult<String> {
    let Some(value) = value else {
        return Ok(String::new());
    };
    let obj = value
        .as_object()
        .ok_or_else(|| type_mismatch("Expected object value", id))?;
    let mut lines = Vec::new();
    for field in node.fields {
        if let Some(field_value) = obj.get(field.name) {
            let rendered = if field.node.kind == NestedNodeKind::List
                && field
                    .node
                    .item
                    .is_some_and(|item| item.kind == NestedNodeKind::Scalar)
            {
                let items = field_value
                    .as_array()
                    .ok_or_else(|| type_mismatch("Expected array value", id))?;
                items
                    .iter()
                    .map(|item| render_scalar(Some(item)))
                    .collect::<Vec<_>>()
                    .join(", ")
            } else {
                render_nested_node(field.node, Some(field_value), id)?
            };
            lines.push(format!("{}: {}", field.name, rendered));
        }
    }
    Ok(lines.join("\n"))
}
