//! Trim and reject empty or duplicate Work Item owners before a file is written.
//!
//! Implements [[RFC-0002:C-WORK-OWNERS]].

use super::engine as edit_engine;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::WorkItemEntry;

enum OwnerEdit {
    Add,
    Replace(i32),
}

pub(super) fn prepare_owner_value(
    entry: &WorkItemEntry,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    id: &str,
) -> DiagnosticResult<Option<String>> {
    let edit = match target {
        edit_engine::ResolvedTarget::Node { path, .. } if path.as_simple() == Some("owners") => {
            OwnerEdit::Add
        }
        edit_engine::ResolvedTarget::IndexedItem {
            container_path,
            index,
            ..
        } if container_path.as_simple() == Some("owners") => OwnerEdit::Replace(*index),
        _ => return Ok(None),
    };

    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticCode::E0805EmptyValue,
            "Owner must not be empty",
            id,
        ));
    }

    let owners = &entry.spec.govctl.owners;
    match edit {
        OwnerEdit::Add => {
            if owners.iter().any(|owner| owner == trimmed) {
                return Err(duplicate_owner(trimmed, id));
            }
        }
        OwnerEdit::Replace(index) => {
            let resolved = super::path::resolve_index(index, owners.len())?;
            if owners
                .iter()
                .enumerate()
                .any(|(position, owner)| position != resolved && owner == trimmed)
            {
                return Err(duplicate_owner(trimmed, id));
            }
        }
    }
    Ok(Some(trimmed.to_string()))
}

fn duplicate_owner(owner: &str, id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0820InvalidFieldValue,
        format!("Duplicate owner '{owner}'"),
        id,
    )
}
