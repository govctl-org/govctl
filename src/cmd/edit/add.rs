use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, GuardTomlAdapter, RfcTomlAdapter, TomlAdapter,
    WorkTomlAdapter,
};
use super::engine as edit_engine;
use super::json_target::add_json_simple_list_field;
use super::rules as edit_rules;
use super::target_doc::add_to_target_doc;
use super::toml_target::{is_work_dependency_target, validate_work_dependency_edit};
use super::{ArtifactType, deserialize_edit_doc, plan_mutation_target, serialize_edit_doc};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::model::{AdrEntry, ChangelogCategory, WorkItemEntry};
use crate::ui;
use crate::write::WriteOp;

struct AdrAddContext {
    pros: Option<Vec<String>>,
    cons: Option<Vec<String>>,
    reject_reason: Option<String>,
}

struct WorkAddContext {
    category_override: Option<ChangelogCategory>,
}

pub(super) struct AddFieldRequest<'a> {
    pub(super) config: &'a Config,
    pub(super) id: &'a str,
    pub(super) field: &'a str,
    pub(super) value: &'a str,
    pub(super) category_override: Option<ChangelogCategory>,
    pub(super) pros: Option<Vec<String>>,
    pub(super) cons: Option<Vec<String>>,
    pub(super) reject_reason: Option<String>,
    pub(super) op: WriteOp,
}

fn adr_add_alternatives(
    entry: &mut AdrEntry,
    value: &str,
    ctx: &AdrAddContext,
) -> DiagnosticResult<()> {
    use crate::model::{Alternative, AlternativeStatus};
    if entry
        .spec
        .content
        .alternatives
        .iter()
        .any(|a| a.text == value)
    {
        return Ok(());
    }

    let status = if ctx.reject_reason.is_some() {
        AlternativeStatus::Rejected
    } else {
        AlternativeStatus::Considered
    };

    entry.spec.content.alternatives.push(Alternative {
        text: value.to_string(),
        status,
        pros: ctx.pros.clone().unwrap_or_default(),
        cons: ctx.cons.clone().unwrap_or_default(),
        rejection_reason: ctx.reject_reason.clone(),
    });
    Ok(())
}

fn work_add_acceptance_criteria(
    entry: &mut WorkItemEntry,
    value: &str,
    ctx: &WorkAddContext,
) -> DiagnosticResult<()> {
    use crate::model::ChecklistItem;
    use crate::write::parse_changelog_change;
    let parsed = parse_changelog_change(value)?;

    let final_category = if let Some(cat) = ctx.category_override {
        cat
    } else if parsed.explicit {
        parsed.category
    } else {
        return Err(Diagnostic::new(
            DiagnosticCode::E0408WorkCriteriaMissingCategory,
            format!(
                "Acceptance criteria requires category. Use prefix (e.g., 'fix: {}') or --category",
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
        category_override,
        pros,
        cons,
        reject_reason,
        op,
    } = request;

    let plan = plan_mutation_target(id, field, edit_rules::Verb::Add)?;
    let artifact = plan.artifact;
    let fp = &plan.field_path;
    let target = &plan.target;

    // Validate tags against controlled vocabulary at add time — [[RFC-0002:C-RESOURCES]]
    if fp.as_simple() == Some("tags") {
        crate::cmd::tag::validate_registered_tag(config, value, id)?;
    }

    match artifact {
        ArtifactType::Adr => {
            let mut entry = AdrTomlAdapter::load(config, id)?;
            if fp.as_simple() == Some("alternatives") {
                let ctx = AdrAddContext {
                    pros,
                    cons,
                    reject_reason,
                };
                adr_add_alternatives(&mut entry, value, &ctx)?;
            } else {
                add_to_serialized_doc(&mut entry.spec, ArtifactType::Adr, target, value, id)?;
            }
            AdrTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::WorkItem => {
            let mut entry = WorkTomlAdapter::load(config, id)?;
            if fp.as_simple() == Some("acceptance_criteria") {
                let ctx = WorkAddContext { category_override };
                work_add_acceptance_criteria(&mut entry, value, &ctx)?;
            } else {
                add_to_serialized_doc(&mut entry.spec, ArtifactType::WorkItem, target, value, id)?;
            }
            if is_work_dependency_target(target) {
                validate_work_dependency_edit(config, &entry)?;
            }
            WorkTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::Rfc => add_json_simple_list_field::<RfcTomlAdapter>(
            config,
            id,
            target,
            value,
            op,
            ArtifactType::Rfc,
            "RFC fields do not support nested paths for add",
        )?,
        ArtifactType::Clause => add_json_simple_list_field::<ClauseTomlAdapter>(
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
    }

    if !op.is_preview() {
        ui::field_added(id, &target.display_path(), value);
    }

    Ok(vec![])
}
