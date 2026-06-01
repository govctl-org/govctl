use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, GuardTomlAdapter, RfcTomlAdapter, TomlAdapter,
    WorkTomlAdapter,
};
use super::json_target::add_json_simple_list_field;
use super::rules as edit_rules;
use super::target_doc::add_to_target_doc;
use super::toml_target::{is_work_dependency_target, validate_work_dependency_edit};
use super::{
    ArtifactType, deserialize_edit_doc, plan_edit_with_field_for_verb, resolve_owned_value,
    serialize_edit_doc, unexpected_edit_state,
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
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

pub struct AddFieldRequest<'a> {
    pub config: &'a Config,
    pub id: &'a str,
    pub field: &'a str,
    pub value: Option<&'a Option<String>>,
    pub stdin: bool,
    pub category_override: Option<ChangelogCategory>,
    pub pros: Option<Vec<String>>,
    pub cons: Option<Vec<String>>,
    pub reject_reason: Option<String>,
    pub op: WriteOp,
}

fn adr_add_alternatives(
    entry: &mut AdrEntry,
    value: &str,
    ctx: &AdrAddContext,
) -> anyhow::Result<()> {
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
) -> anyhow::Result<()> {
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
        )
        .into());
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

pub fn add_to_field(request: AddFieldRequest<'_>) -> anyhow::Result<Vec<Diagnostic>> {
    let AddFieldRequest {
        config,
        id,
        field,
        value,
        stdin,
        category_override,
        pros,
        cons,
        reject_reason,
        op,
    } = request;

    let plan = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Add))?;
    let artifact = plan.artifact;
    let fp = plan
        .field_path
        .as_ref()
        .ok_or_else(|| unexpected_edit_state(id, "validated above: field path must be present"))?;
    let target = plan
        .target
        .as_ref()
        .ok_or_else(|| unexpected_edit_state(id, "mutation planning should produce target"))?;
    let value = resolve_owned_value(value, stdin)?;
    let value = value.as_str();

    // Validate tags against controlled vocabulary at add time — [[RFC-0002:C-RESOURCES]]
    if fp.as_simple() == Some("tags") {
        let tag_re = crate::cmd::tag::tag_re().map_err(|e| {
            Diagnostic::new(
                DiagnosticCode::E0806InvalidPattern,
                format!("Failed to compile tag regex: {e}"),
                id,
            )
        })?;
        if !tag_re.is_match(value) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1101TagInvalidFormat,
                format!("Invalid tag format '{value}': must match ^[a-z][a-z0-9-]*$"),
                id,
            )
            .into());
        }
        let allowed = &config.tags.allowed;
        if !allowed.iter().any(|t| t == value) {
            return Err(Diagnostic::new(
                DiagnosticCode::E1105TagUnknown,
                format!("Tag '{value}' is not in config.toml [tags] allowed. Register it first with: govctl tag new {value}"),
                id,
            )
            .into());
        }
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
                let mut doc = serialize_edit_doc(&entry.spec, id)?;
                add_to_target_doc(ArtifactType::Adr, &mut doc, target, value, id)?;
                entry.spec = deserialize_edit_doc(doc, id)?;
            }
            AdrTomlAdapter::write(config, &entry, op)?;
        }
        ArtifactType::WorkItem => {
            let mut entry = WorkTomlAdapter::load(config, id)?;
            if fp.as_simple() == Some("acceptance_criteria") {
                let ctx = WorkAddContext { category_override };
                work_add_acceptance_criteria(&mut entry, value, &ctx)?;
            } else {
                let mut doc = serialize_edit_doc(&entry.spec, id)?;
                add_to_target_doc(ArtifactType::WorkItem, &mut doc, target, value, id)?;
                entry.spec = deserialize_edit_doc(doc, id)?;
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
            let mut doc = serialize_edit_doc(&entry.spec, id)?;
            add_to_target_doc(ArtifactType::Guard, &mut doc, target, value, id)?;
            entry.spec = deserialize_edit_doc(doc, id)?;
            GuardTomlAdapter::write(config, &entry, op)?;
        }
    }

    if !op.is_preview() {
        ui::field_added(id, &target.display_path(), value);
    }

    Ok(vec![])
}
