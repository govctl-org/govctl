use super::TomlEditableEntry;
use super::work_dependencies::{is_work_dependency_target, validate_work_dependency_edit};
use crate::cmd::edit::adapter::{TomlAdapter, WorkTomlAdapter};
use crate::cmd::edit::engine as edit_engine;
use crate::cmd::edit::refs::{is_refs_target, validate_ref_edit};
use crate::cmd::edit::rules::NestedObjectSetMode;
use crate::cmd::edit::runtime as edit_runtime;
use crate::cmd::edit::{
    ArtifactType, deserialize_edit_doc, serialize_edit_doc, unexpected_edit_state,
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::WorkItemEntry;
use crate::write::WriteOp;

pub(in crate::cmd::edit) fn set_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    allow_forced_simple_set: bool,
) -> DiagnosticResult<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    apply_toml_target_to_entry(
        &mut entry,
        target,
        value,
        artifact,
        allow_forced_simple_set,
        id,
    )?;
    if is_refs_target(target) {
        validate_ref_edit(config, artifact, id, value)?;
    }
    A::write(config, &entry, op)?;
    Ok(())
}

pub(in crate::cmd::edit) fn set_work_toml_field(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    allow_forced_simple_set: bool,
) -> DiagnosticResult<()> {
    let mut entry = WorkTomlAdapter::load(config, id)?;
    if let Some(index) = acceptance_criterion_index(target) {
        set_acceptance_criterion(&mut entry, index, value, id)?;
    } else {
        apply_toml_target_to_entry(
            &mut entry,
            target,
            value,
            ArtifactType::WorkItem,
            allow_forced_simple_set,
            id,
        )?;
    }
    if is_refs_target(target) {
        validate_ref_edit(config, ArtifactType::WorkItem, id, value)?;
    }
    if is_work_dependency_target(target) {
        validate_work_dependency_edit(config, &entry)?;
    }
    WorkTomlAdapter::write(config, &entry, op)?;
    Ok(())
}

fn acceptance_criterion_index(target: &edit_engine::ResolvedTarget) -> Option<i32> {
    match target {
        edit_engine::ResolvedTarget::IndexedItem {
            index,
            item_kind: edit_engine::TargetKind::Object,
            object_set_mode: Some(NestedObjectSetMode::AcceptanceCriterion),
            ..
        } => Some(*index),
        _ => None,
    }
}

fn set_acceptance_criterion(
    entry: &mut WorkItemEntry,
    index: i32,
    value: &str,
    id: &str,
) -> DiagnosticResult<()> {
    let parsed = crate::write::parse_changelog_change_with_fallback(value)?;
    if parsed.message.is_empty() {
        return Err(empty_acceptance_criterion(id));
    }
    let criteria = &mut entry.spec.content.acceptance_criteria;
    let resolved = crate::cmd::edit::path::resolve_index(index, criteria.len())?;
    let criterion = &mut criteria[resolved];
    criterion.text = parsed.message;
    if parsed.explicit {
        criterion.category = parsed.category;
    }
    Ok(())
}

fn empty_acceptance_criterion(id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::E0805EmptyValue,
        "Acceptance criterion text cannot be empty",
        id,
    )
}

fn apply_toml_target_to_entry<E>(
    entry: &mut E,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    artifact: ArtifactType,
    allow_forced_simple_set: bool,
    id: &str,
) -> DiagnosticResult<()>
where
    E: TomlEditableEntry,
{
    let mut doc = serialize_edit_doc(entry.spec(), id)?;
    match target {
        edit_engine::ResolvedTarget::Node {
            path,
            kind: edit_engine::TargetKind::Scalar,
            origin,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => {
                let simple = path
                    .as_simple()
                    .ok_or_else(|| unexpected_edit_state(id, "simple target path expected"))?;
                if allow_forced_simple_set {
                    edit_runtime::set_simple_field_forced(artifact, &mut doc, simple, value, id)?;
                } else {
                    edit_runtime::set_simple_field(artifact, &mut doc, simple, value, id)?;
                }
            }
            edit_engine::TargetOrigin::Nested => {
                edit_runtime::set_nested_field(artifact, &mut doc, path, value, id)?;
            }
        },
        edit_engine::ResolvedTarget::IndexedItem {
            origin,
            path,
            container_path,
            index,
            item_kind: edit_engine::TargetKind::Scalar,
            ..
        } => match origin {
            edit_engine::TargetOrigin::Simple => {
                let simple = container_path.as_simple().ok_or_else(|| {
                    unexpected_edit_state(id, "simple indexed container expected")
                })?;
                edit_runtime::set_simple_list_item(artifact, &mut doc, simple, *index, value, id)?;
            }
            edit_engine::TargetOrigin::Nested => {
                edit_runtime::set_nested_field(artifact, &mut doc, path, value, id)?;
            }
        },
        _ => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0817PathTypeMismatch,
                "set requires a scalar field or indexed scalar list item",
                id,
            ));
        }
    }
    *entry.spec_mut() = deserialize_edit_doc(doc, id)?;
    Ok(())
}
