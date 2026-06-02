use super::spec::{SetMode, SimpleSetSpec};
use super::support::{ensure_path_mut_with_leaf, type_mismatch};
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use serde_json::Value;

pub(super) fn apply_set(
    doc: &mut Value,
    spec: SimpleSetSpec,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let slot = ensure_value_path_mut(doc, spec.path, id)?;

    match spec.mode {
        SetMode::String => *slot = Value::String(value.to_string()),
        SetMode::Integer => {
            let n: i64 = value.parse().map_err(|_| {
                Diagnostic::new(
                    DiagnosticCode::E0820InvalidFieldValue,
                    format!("Invalid integer value for {}: {value}", id),
                    id,
                )
            })?;
            *slot = Value::Number(serde_json::Number::from(n));
        }
        SetMode::Enum {
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

pub(super) fn array_items_mut<'a>(
    doc: &'a mut Value,
    path: &[&str],
    id: &str,
) -> DiagnosticResult<&'a mut Vec<Value>> {
    ensure_array_path_mut(doc, path, id)?
        .as_array_mut()
        .ok_or_else(|| type_mismatch("Expected an array value", id))
}

pub(super) fn ensure_value_path_mut<'a>(
    doc: &'a mut Value,
    path: &[&str],
    id: &str,
) -> DiagnosticResult<&'a mut Value> {
    ensure_path_mut_with_leaf(doc, path, id, || Value::Null)
}

pub(super) fn ensure_array_path_mut<'a>(
    doc: &'a mut Value,
    path: &[&str],
    id: &str,
) -> DiagnosticResult<&'a mut Value> {
    ensure_path_mut_with_leaf(doc, path, id, || Value::Array(Vec::new()))
}
