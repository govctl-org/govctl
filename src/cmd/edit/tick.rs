use super::adapter::{AdrTomlAdapter, WorkTomlAdapter};
use super::matching::MatchOptions;
use super::rules as edit_rules;
use super::toml_target::tick_toml_field;
use super::{
    ArtifactType, plan_edit_with_field_for_verb, reject_match_flags_for_indexed_target,
    unexpected_edit_state,
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};
use crate::write::WriteOp;

const TICK_UNSUPPORTED_ARTIFACT_ERROR: &str = "Tick only works for work items and ADRs: {id}";
const ADR_TICK_STATUS_ERROR: &str =
    "ADR tick status must be one of: accepted, considered, rejected";
const WORK_TICK_STATUS_ERROR: &str =
    "Work item tick status must be one of: done, pending, cancelled";

pub fn tick_item(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    status: crate::TickStatus,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    let plan = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Tick))?;
    let artifact = plan.artifact;
    let target = plan
        .target
        .as_ref()
        .ok_or_else(|| unexpected_edit_state(id, "mutation planning should produce target"))?;
    reject_match_flags_for_indexed_target(id, target, opts)?;

    let status_str = match (artifact, status) {
        (ArtifactType::Adr, crate::TickStatus::Accepted) => "accepted",
        (ArtifactType::Adr, crate::TickStatus::Considered) => "considered",
        (ArtifactType::Adr, crate::TickStatus::Rejected) => "rejected",
        (ArtifactType::Adr, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0820InvalidFieldValue,
                ADR_TICK_STATUS_ERROR,
                id,
            ));
        }
        (ArtifactType::WorkItem, crate::TickStatus::Done) => "done",
        (ArtifactType::WorkItem, crate::TickStatus::Pending) => "pending",
        (ArtifactType::WorkItem, crate::TickStatus::Cancelled) => "cancelled",
        (ArtifactType::WorkItem, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0820InvalidFieldValue,
                WORK_TICK_STATUS_ERROR,
                id,
            ));
        }
        (ArtifactType::Rfc | ArtifactType::Clause | ArtifactType::Guard, _) => {
            return Err(Diagnostic::new(
                DiagnosticCode::E0813SupersedeNotSupported,
                TICK_UNSUPPORTED_ARTIFACT_ERROR.replace("{id}", id),
                id,
            ));
        }
    };
    let ticked_text = match artifact {
        ArtifactType::Adr => tick_toml_field::<AdrTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::Adr,
            status_str,
        )?,
        ArtifactType::WorkItem => tick_toml_field::<WorkTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::WorkItem,
            status_str,
        )?,
        ArtifactType::Rfc | ArtifactType::Clause | ArtifactType::Guard => {
            unreachable!("handled above")
        }
    };

    if !op.is_preview() {
        crate::ui::ticked(&ticked_text, status_str);
    }

    Ok(vec![])
}
