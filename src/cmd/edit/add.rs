use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, ConformanceTomlAdapter, GuardTomlAdapter, RfcTomlAdapter,
    TomlAdapter, WorkTomlAdapter,
};
use super::doc_target::{add_doc_simple_list_field, rfc_changelog};
use super::engine as edit_engine;
use super::refs::{is_refs_target, validate_ref_edit};
use super::rules as edit_rules;
use super::tags::validate_tag_edit;
use super::target_doc::add_to_target_doc;
use super::toml_target::{is_work_dependency_target, validate_work_dependency_edit};
use super::{ArtifactType, deserialize_edit_doc, plan_mutation_target, serialize_edit_doc};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{AdrEntry, WorkItemEntry};
use crate::ui;
use crate::write::WriteOp;

pub(super) struct AddFieldRequest<'a> {
    pub(super) config: &'a Config,
    pub(super) id: &'a str,
    pub(super) field: &'a str,
    pub(super) value: &'a str,
    pub(super) op: WriteOp,
}

fn adr_add_alternatives(entry: &mut AdrEntry, value: &str) {
    use crate::model::{Alternative, AlternativeStatus};
    if entry
        .spec
        .content
        .alternatives
        .iter()
        .any(|a| a.text == value)
    {
        return;
    }

    entry.spec.content.alternatives.push(Alternative {
        text: value.to_string(),
        status: AlternativeStatus::Considered,
        pros: vec![],
        cons: vec![],
        rejection_reason: None,
    });
}

fn work_add_acceptance_criteria(entry: &mut WorkItemEntry, value: &str) -> DiagnosticResult<()> {
    use crate::model::ChecklistItem;
    use crate::write::parse_changelog_change;
    let parsed = parse_changelog_change(value)?;

    let final_category = if parsed.explicit {
        parsed.category
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0408WorkCriteriaMissingCategory,
            format!(
                "Acceptance criteria requires a category prefix (e.g., 'fix: {}')",
                parsed.message
            ),
            &entry.spec.govctl.id,
        ));
    };

    if !entry
        .spec
        .content
        .acceptance_criteria
        .iter()
        .any(|c| c.text == parsed.message)
    {
        entry
            .spec
            .content
            .acceptance_criteria
            .push(ChecklistItem::with_category(
                &parsed.message,
                final_category,
            ));
    }
    Ok(())
}

fn add_to_serialized_doc<T>(
    spec: &mut T,
    artifact: ArtifactType,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    id: &str,
) -> DiagnosticResult<()>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut doc = serialize_edit_doc(spec, id)?;
    add_to_target_doc(artifact, &mut doc, target, value, id)?;
    *spec = deserialize_edit_doc(doc, id)?;
    Ok(())
}

pub fn add_to_field(request: AddFieldRequest<'_>) -> DiagnosticResult<Vec<Diagnostic>> {
    let AddFieldRequest {
        config,
        id,
        field,
        value,
        op,
    } = request;

    let plan = plan_mutation_target(id, field, edit_rules::Verb::Add)?;
    let artifact = plan.artifact;
    let fp = &plan.field_path;
    let target = &plan.target;
    target.ensure_supports(edit_rules::Verb::Add, id)?;

    // [[RFC-0002:C-EDIT-FIELD-CONTRACT]] requires equivalent add/set validation.
    validate_tag_edit(config, target, value, id)?;
    if is_refs_target(target) {
        validate_ref_edit(config, artifact, id, value)?;
    }

    match artifact {
        ArtifactType::Adr => {
            let mut entry = AdrTomlAdapter::load(config, id)?;
            if fp.as_simple() == Some("alternatives") {
                adr_add_alternatives(&mut entry, value);
            } else {
                add_to_serialized_doc(&mut entry.spec, ArtifactType::Adr, target, value, id)?;
            }
            AdrTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::WorkItem => {
            let mut entry = WorkTomlAdapter::load(config, id)?;
            if fp.as_simple() == Some("acceptance_criteria") {
                work_add_acceptance_criteria(&mut entry, value)?;
            } else {
                add_to_serialized_doc(&mut entry.spec, ArtifactType::WorkItem, target, value, id)?;
            }
            if is_work_dependency_target(target) {
                validate_work_dependency_edit(config, &entry)?;
            }
            WorkTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::Rfc => {
            if rfc_changelog::is_target(target) {
                rfc_changelog::add(config, id, target, value, op)?;
            } else {
                add_doc_simple_list_field::<RfcTomlAdapter>(
                    config,
                    id,
                    target,
                    value,
                    op,
                    ArtifactType::Rfc,
                    "RFC fields do not support nested paths for add",
                )?;
            }
        }
        ArtifactType::Clause => add_doc_simple_list_field::<ClauseTomlAdapter>(
            config,
            id,
            target,
            value,
            op,
            ArtifactType::Clause,
            "Clause fields do not support nested paths for add",
        )?,
        ArtifactType::Guard => {
            let mut entry = GuardTomlAdapter::load(config, id)?;
            add_to_serialized_doc(&mut entry.spec, ArtifactType::Guard, target, value, id)?;
            GuardTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::Conformance => {
            let mut entry = ConformanceTomlAdapter::load(config, id)?;
            add_to_serialized_doc(
                &mut entry.spec,
                ArtifactType::Conformance,
                target,
                value,
                id,
            )?;
            ConformanceTomlAdapter::write(config, &entry, op)?;
        }
    }

    if !op.is_preview() {
        ui::field_added(id, &target.display_path(), value);
    }

    Ok(vec![])
}
