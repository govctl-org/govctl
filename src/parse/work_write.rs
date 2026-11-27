//! Preserve unchanged Work Item text when an existing file is updated.
//!
//! Implements [[RFC-0002:C-WORK-FILE-STABILITY]]. A new file may be pretty-printed.
//! An existing file is patched in place. If that patch cannot keep the previous
//! text, the write fails and the file is left unchanged.

use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::model::{ChecklistItem, WorkItemSpec};
use crate::schema::ArtifactSchema;
use crate::write::{WriteOp, write_file};
use std::collections::BTreeSet;
use std::path::Path;

use super::toml_io;

pub(super) fn write_work_item(
    path: &Path,
    spec: &WorkItemSpec,
    op: WriteOp,
    display_path: Option<&Path>,
) -> Result<(), Diagnostic> {
    if !path.exists() {
        return toml_io::write_toml_spec(
            path,
            ArtifactSchema::WorkItem,
            spec,
            op,
            display_path,
            "Failed to serialize TOML",
        );
    }

    let location = display_path.unwrap_or(path).display().to_string();
    let original = std::fs::read_to_string(path)
        .map_err(|err| Diagnostic::io_error("read work item", err, &location))?;
    let mut document = original.parse::<toml_edit::DocumentMut>().map_err(|err| {
        preserve_error(
            &location,
            format!("existing work item text cannot be preserved: {err}"),
        )
    })?;
    let previous: toml::Table = toml::from_str(&document.to_string()).map_err(|err| {
        preserve_error(
            &location,
            format!("existing work item text cannot be preserved: {err}"),
        )
    })?;
    let desired_text = toml::to_string(spec).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {err}"),
            &location,
        )
    })?;
    let desired: toml::Table = toml::from_str(&desired_text).map_err(|err| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            format!("Failed to serialize TOML: {err}"),
            &location,
        )
    })?;

    let mut changed = false;
    patch_table(
        document.as_table_mut(),
        &previous,
        &desired,
        &location,
        &mut changed,
    )?;
    if !changed {
        return Ok(());
    }
    write_file(path, &document.to_string(), op, Some(Path::new(&location)))
}

fn patch_table(
    document: &mut toml_edit::Table,
    previous: &toml::Table,
    desired: &toml::Table,
    location: &str,
    changed: &mut bool,
) -> Result<(), Diagnostic> {
    if table_keys(document) != value_keys(previous) {
        return Err(cannot_preserve(location));
    }

    let stale = stale_keys(previous, desired);
    for key in &stale {
        let Some(previous_value) = previous.get(key) else {
            continue;
        };
        if omit_equivalent(previous_value) {
            continue;
        }
        document.remove(key);
        *changed = true;
    }

    for (key, desired_value) in desired {
        match document.get_mut(key) {
            None => {
                document.insert(key, value_to_item(desired_value, location)?);
                *changed = true;
            }
            Some(_) => {
                let Some(previous_value) = previous.get(key) else {
                    return Err(cannot_preserve(location));
                };
                if same_stored_value(previous_value, desired_value) {
                    continue;
                }
                let recursed = if let (Some(previous_table), Some(desired_table)) =
                    (previous_value.as_table(), desired_value.as_table())
                {
                    patch_child_table(
                        document,
                        key,
                        previous_table,
                        desired_table,
                        location,
                        changed,
                    )?
                } else {
                    false
                };
                if recursed {
                    continue;
                }
                document.insert(key, value_to_item(desired_value, location)?);
                *changed = true;
            }
        }
    }
    Ok(())
}

fn patch_inline_table(
    document: &mut toml_edit::InlineTable,
    previous: &toml::Table,
    desired: &toml::Table,
    location: &str,
    changed: &mut bool,
) -> Result<(), Diagnostic> {
    if inline_keys(document) != value_keys(previous) {
        return Err(cannot_preserve(location));
    }

    let stale = stale_keys(previous, desired);
    for key in &stale {
        let Some(previous_value) = previous.get(key) else {
            continue;
        };
        if omit_equivalent(previous_value) {
            continue;
        }
        document.remove(key);
        *changed = true;
    }

    for (key, desired_value) in desired {
        if !document.contains_key(key) {
            document.insert(key, value_to_inline(desired_value, location)?);
            *changed = true;
            continue;
        }
        let Some(previous_value) = previous.get(key) else {
            return Err(cannot_preserve(location));
        };
        if same_stored_value(previous_value, desired_value) {
            continue;
        }
        let nested = document
            .get(key)
            .and_then(toml_edit::Value::as_inline_table)
            .is_some();
        if nested
            && let (Some(previous_table), Some(desired_table)) =
                (previous_value.as_table(), desired_value.as_table())
        {
            let Some(child) = document
                .get_mut(key)
                .and_then(toml_edit::Value::as_inline_table_mut)
            else {
                return Err(cannot_preserve(location));
            };
            patch_inline_table(child, previous_table, desired_table, location, changed)?;
            continue;
        }
        document.insert(key, value_to_inline(desired_value, location)?);
        *changed = true;
    }
    Ok(())
}

/// Recurse into a standard table or an inline table. Returns false when the
/// stored value is neither, so the caller replaces that value.
fn patch_child_table(
    document: &mut toml_edit::Table,
    key: &str,
    previous: &toml::Table,
    desired: &toml::Table,
    location: &str,
    changed: &mut bool,
) -> Result<bool, Diagnostic> {
    let shape = match document.get(key) {
        Some(item) if item.is_table() => TableShape::Standard,
        Some(item) if item.is_inline_table() => TableShape::Inline,
        _ => TableShape::Other,
    };
    match shape {
        TableShape::Standard => {
            let Some(child) = document
                .get_mut(key)
                .and_then(toml_edit::Item::as_table_mut)
            else {
                return Ok(false);
            };
            patch_table(child, previous, desired, location, changed)?;
            Ok(true)
        }
        TableShape::Inline => {
            let Some(child) = document
                .get_mut(key)
                .and_then(toml_edit::Item::as_inline_table_mut)
            else {
                return Ok(false);
            };
            patch_inline_table(child, previous, desired, location, changed)?;
            Ok(true)
        }
        TableShape::Other => Ok(false),
    }
}

enum TableShape {
    Standard,
    Inline,
    Other,
}

fn table_keys(table: &toml_edit::Table) -> BTreeSet<String> {
    table.iter().map(|(key, _)| key.to_string()).collect()
}

fn inline_keys(table: &toml_edit::InlineTable) -> BTreeSet<String> {
    table.iter().map(|(key, _)| key.to_string()).collect()
}

/// Two stored values are the same when their typed meaning matches. A checklist
/// item that omits its default status or category is the same item as one that
/// stores those defaults.
fn same_stored_value(previous: &toml::Value, desired: &toml::Value) -> bool {
    match (previous, desired) {
        (toml::Value::Table(previous), toml::Value::Table(desired)) => {
            same_stored_table(previous, desired)
        }
        (toml::Value::Array(previous), toml::Value::Array(desired)) => {
            previous.len() == desired.len()
                && previous
                    .iter()
                    .zip(desired)
                    .all(|(previous, desired)| same_stored_value(previous, desired))
        }
        _ => previous == desired,
    }
}

fn same_stored_table(previous: &toml::Table, desired: &toml::Table) -> bool {
    if let Some(same) = same_checklist(previous, desired) {
        return same;
    }
    let mut keys = BTreeSet::new();
    keys.extend(previous.keys().map(String::as_str));
    keys.extend(desired.keys().map(String::as_str));
    keys.into_iter()
        .all(|key| match (previous.get(key), desired.get(key)) {
            (Some(previous), Some(desired)) => same_stored_value(previous, desired),
            _ => false,
        })
}

/// Omitted checklist `status` and `category` deserialize as the same item as
/// the explicit defaults. Compare that typed item, not the stored key text.
fn same_checklist(previous: &toml::Table, desired: &toml::Table) -> Option<bool> {
    if !checklist_shape(previous) || !checklist_shape(desired) {
        return None;
    }
    let previous = checklist_item(previous)?;
    let desired = checklist_item(desired)?;
    Some(previous == desired)
}

fn checklist_shape(table: &toml::Table) -> bool {
    table.contains_key("text")
        && table
            .keys()
            .all(|key| matches!(key.as_str(), "text" | "status" | "category"))
}

fn checklist_item(table: &toml::Table) -> Option<ChecklistItem> {
    toml::Value::Table(table.clone()).try_into().ok()
}

fn value_keys(table: &toml::Table) -> BTreeSet<String> {
    table.keys().cloned().collect()
}

/// Serde omits empty lists and tables. An empty stored value is the same field.
fn omit_equivalent(value: &toml::Value) -> bool {
    match value {
        toml::Value::Array(items) => items.is_empty(),
        toml::Value::Table(table) => table.values().all(omit_equivalent),
        _ => false,
    }
}

fn value_to_item(value: &toml::Value, location: &str) -> Result<toml_edit::Item, Diagnostic> {
    if let Some(items) = value.as_array()
        && !items.is_empty()
        && items.iter().all(|item| item.as_table().is_some())
    {
        let mut tables = toml_edit::ArrayOfTables::new();
        for item in items {
            let Some(table) = item.as_table() else {
                return Err(preserve_error(location, "failed to format a changed field"));
            };
            tables.push(table_to_edit(table, location)?);
        }
        return Ok(toml_edit::Item::ArrayOfTables(tables));
    }
    if let Some(table) = value.as_table() {
        return Ok(toml_edit::Item::Table(table_to_edit(table, location)?));
    }

    let mut wrapper = toml::Table::new();
    wrapper.insert("v".to_string(), value.clone());
    let text = toml::to_string(&wrapper).map_err(|err| {
        preserve_error(location, format!("failed to format a changed field: {err}"))
    })?;
    let mut document = text.parse::<toml_edit::DocumentMut>().map_err(|err| {
        preserve_error(location, format!("failed to format a changed field: {err}"))
    })?;
    document
        .remove("v")
        .ok_or_else(|| preserve_error(location, "failed to format a changed field"))
}

fn value_to_inline(value: &toml::Value, location: &str) -> Result<toml_edit::Value, Diagnostic> {
    match value {
        toml::Value::Table(table) => {
            let mut inline = toml_edit::InlineTable::new();
            for (key, child) in table {
                inline.insert(key, value_to_inline(child, location)?);
            }
            Ok(toml_edit::Value::InlineTable(inline))
        }
        toml::Value::Array(items) => {
            let mut array = toml_edit::Array::new();
            for item in items {
                array.push(value_to_inline(item, location)?);
            }
            Ok(toml_edit::Value::Array(array))
        }
        _ => {
            let item = value_to_item(value, location)?;
            item.into_value()
                .map_err(|_| preserve_error(location, "failed to format a changed field"))
        }
    }
}

fn table_to_edit(table: &toml::Table, location: &str) -> Result<toml_edit::Table, Diagnostic> {
    let mut edited = toml_edit::Table::new();
    for (key, value) in table {
        edited.insert(key, value_to_item(value, location)?);
    }
    Ok(edited)
}

fn stale_keys(previous: &toml::Table, desired: &toml::Table) -> Vec<String> {
    previous
        .keys()
        .filter(|key| !desired.contains_key(key.as_str()))
        .cloned()
        .collect()
}

fn cannot_preserve(location: &str) -> Diagnostic {
    preserve_error(location, "existing work item text cannot be preserved")
}

fn preserve_error(location: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::E0901IoError, message, location)
}
