use super::super::adapter::{DocAdapter, RfcTomlAdapter};
use super::super::engine as edit_engine;
use super::super::matching::MatchOptions;
use super::super::runtime as edit_runtime;
use super::super::target_doc::{NestedGetMode, add_to_target_doc, render_target_from_doc};
use super::super::target_doc_remove::{notify_removed, remove_target_from_doc};
use super::{ArtifactType, deserialize_edit_doc, serialize_edit_doc};
use crate::cmd::output::print_json;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::ChangelogEntry;
use crate::write::{WriteOp, current_changelog_entry, current_changelog_entry_mut};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct CurrentChangelogDoc {
    changelog: CurrentChangelogView,
}

#[derive(Debug, Serialize, Deserialize)]
struct CurrentChangelogView {
    version: String,
    date: String,
    summary: Option<String>,
    added: Vec<String>,
    changed: Vec<String>,
    deprecated: Vec<String>,
    removed: Vec<String>,
    fixed: Vec<String>,
    security: Vec<String>,
}

impl From<&ChangelogEntry> for CurrentChangelogView {
    fn from(entry: &ChangelogEntry) -> Self {
        Self {
            version: entry.version.clone(),
            date: entry.date.clone(),
            summary: entry.notes.clone(),
            added: entry.added.clone(),
            changed: entry.changed.clone(),
            deprecated: entry.deprecated.clone(),
            removed: entry.removed.clone(),
            fixed: entry.fixed.clone(),
            security: entry.security.clone(),
        }
    }
}

pub(in crate::cmd::edit) fn is_target(target: &edit_engine::ResolvedTarget) -> bool {
    target
        .path()
        .segments
        .first()
        .is_some_and(|segment| segment.name == "changelog")
}

pub(in crate::cmd::edit) fn get(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
) -> DiagnosticResult<()> {
    let loaded = RfcTomlAdapter::load(config, id)?;
    let doc = current_changelog_doc(&loaded.data, id)?;

    let root_only = target.path().segments.len() == 1 && target.path().segments[0].index.is_none();
    if !root_only {
        render_target_from_doc(ArtifactType::Rfc, &doc, target, id, NestedGetMode::Allow)?;
        return Err(Diagnostic::new(
            DiagnosticCode::E0804FieldNotEditable,
            "RFC changelog get supports only the current-entry root path",
            id,
        ));
    }

    print_json(
        doc.get("changelog").unwrap_or(&serde_json::Value::Null),
        DiagnosticCode::E0903UnexpectedError,
        "Failed to serialize current RFC changelog entry",
        id,
    )
}

pub(in crate::cmd::edit) fn set(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
) -> DiagnosticResult<()> {
    let mut loaded = RfcTomlAdapter::load(config, id)?;
    crate::cmd::lifecycle::require_changelog_update_ready(config, &loaded.path, id)?;
    let mut doc = current_changelog_doc(&loaded.data, id)?;
    edit_runtime::set_nested_field(ArtifactType::Rfc, &mut doc, target.path(), value, id)?;
    apply_current_changelog_doc(&mut loaded.data, doc, id)?;
    RfcTomlAdapter::write(config, &loaded, op)
}

pub(in crate::cmd::edit) fn add(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
) -> DiagnosticResult<()> {
    let mut loaded = RfcTomlAdapter::load(config, id)?;
    crate::cmd::lifecycle::require_changelog_update_ready(config, &loaded.path, id)?;
    let mut doc = current_changelog_doc(&loaded.data, id)?;
    add_to_target_doc(ArtifactType::Rfc, &mut doc, target, value, id)?;
    apply_current_changelog_doc(&mut loaded.data, doc, id)?;
    RfcTomlAdapter::write(config, &loaded, op)
}

pub(in crate::cmd::edit) fn remove(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
) -> DiagnosticResult<()> {
    if !matches!(
        target,
        edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Nested,
            ..
        }
    ) {
        return Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            "RFC changelog removal requires an indexed current-category path",
            id,
        ));
    }

    let mut loaded = RfcTomlAdapter::load(config, id)?;
    crate::cmd::lifecycle::require_changelog_update_ready(config, &loaded.path, id)?;
    let mut doc = current_changelog_doc(&loaded.data, id)?;
    let (display_field, removed) =
        remove_target_from_doc(ArtifactType::Rfc, &mut doc, id, target, opts)?;
    apply_current_changelog_doc(&mut loaded.data, doc, id)?;
    RfcTomlAdapter::write(config, &loaded, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}

fn current_changelog_doc(
    rfc: &crate::model::RfcSpec,
    id: &str,
) -> DiagnosticResult<serde_json::Value> {
    let entry = current_changelog_entry(rfc)?;
    serialize_edit_doc(
        &CurrentChangelogDoc {
            changelog: CurrentChangelogView::from(entry),
        },
        id,
    )
}

fn apply_current_changelog_doc(
    rfc: &mut crate::model::RfcSpec,
    doc: serde_json::Value,
    id: &str,
) -> DiagnosticResult<()> {
    let view: CurrentChangelogDoc = deserialize_edit_doc(doc, id)?;
    let entry = current_changelog_entry_mut(rfc)?;
    entry.notes = view.changelog.summary;
    entry.added = view.changelog.added;
    entry.changed = view.changelog.changed;
    entry.deprecated = view.changelog.deprecated;
    entry.removed = view.changelog.removed;
    entry.fixed = view.changelog.fixed;
    entry.security = view.changelog.security;
    Ok(())
}
