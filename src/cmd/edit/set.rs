use super::adapter::{AdrTomlAdapter, ClauseTomlAdapter, GuardTomlAdapter, RfcTomlAdapter};
use super::engine as edit_engine;
use super::json_target::{set_clause_field, set_rfc_field};
use super::path::FieldPath;
use super::rules as edit_rules;
use super::toml_target::{set_toml_field, set_work_toml_field};
use super::{ArtifactType, plan_edit_with_field_for_verb};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::WriteOp;

pub(crate) fn set_field_direct(
    config: &Config,
    id: &str,
    field: &str,
    value: &str,
    op: WriteOp,
) -> anyhow::Result<()> {
    let plan = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Set))?;
    let target = plan.target.as_ref().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "mutation planning should produce target",
            id,
        )
    })?;
    apply_set_field(config, id, target, plan.artifact, value, op, false)
}

pub(super) fn apply_set_field(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    artifact: ArtifactType,
    value: &str,
    op: WriteOp,
    enforce_verb_ownership: bool,
) -> anyhow::Result<()> {
    let fp = target.path();
    if enforce_verb_ownership {
        reject_verb_owned_set(artifact, fp, id)?;
    }
    // Implements [[ADR-0042]]: block setting `decision` without complete alternatives
    if artifact == ArtifactType::Adr && fp.as_simple() == Some("decision") {
        crate::cmd::lifecycle::validate_adr_completeness(config, id)?;
    }
    match artifact {
        ArtifactType::Adr => set_toml_field::<AdrTomlAdapter>(
            config,
            id,
            target,
            value,
            op,
            ArtifactType::Adr,
            !enforce_verb_ownership,
        )?,
        ArtifactType::WorkItem => {
            if fp.as_simple() == Some("notes") {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0804FieldNotEditable,
                    "Use 'add' to append notes and 'remove' to delete them",
                    id,
                )
                .into());
            }
            set_work_toml_field(config, id, target, value, op, !enforce_verb_ownership)?
        }
        ArtifactType::Rfc => {
            set_rfc_field::<RfcTomlAdapter>(config, id, target, value, op, !enforce_verb_ownership)?
        }
        ArtifactType::Clause => set_clause_field::<ClauseTomlAdapter>(
            config,
            id,
            target,
            value,
            op,
            !enforce_verb_ownership,
        )?,
        ArtifactType::Guard => set_toml_field::<GuardTomlAdapter>(
            config,
            id,
            target,
            value,
            op,
            ArtifactType::Guard,
            !enforce_verb_ownership,
        )?,
    }
    Ok(())
}

fn reject_verb_owned_set(artifact: ArtifactType, fp: &FieldPath, id: &str) -> anyhow::Result<()> {
    let path = fp.to_string();
    let msg = match artifact {
        ArtifactType::Rfc => match fp.as_simple() {
            Some("status") => Some(
                "RFC status is lifecycle-owned. Use `govctl rfc finalize`, `govctl rfc deprecate`, or `govctl rfc supersede`.",
            ),
            Some("phase") => Some("RFC phase is lifecycle-owned. Use `govctl rfc advance`."),
            Some("version") => Some("RFC version is lifecycle-owned. Use `govctl rfc bump`."),
            _ => None,
        },
        ArtifactType::Clause => match fp.as_simple() {
            Some("status") => Some(
                "Clause status is lifecycle-owned. Use `govctl clause deprecate` or `govctl clause supersede`.",
            ),
            Some("superseded_by") => {
                Some("Clause supersession is lifecycle-owned. Use `govctl clause supersede`.")
            }
            Some("since") => Some(
                "Clause 'since' is derived from RFC versioning. Use `govctl rfc bump` or `govctl rfc finalize`.",
            ),
            _ => None,
        },
        ArtifactType::Adr => {
            if fp.as_simple() == Some("status") || fp.as_simple() == Some("superseded_by") {
                Some(
                    "ADR lifecycle fields are verb-owned. Use `govctl adr accept`, `govctl adr reject`, or `govctl adr supersede`.",
                )
            } else if fp.segments.len() == 2
                && fp.segments[0].name == "alternatives"
                && fp.segments[1].name == "status"
            {
                Some(
                    "ADR alternative status is tick-owned. Use `govctl adr tick ... alternatives ...`.",
                )
            } else {
                None
            }
        }
        ArtifactType::WorkItem => {
            if fp.as_simple() == Some("status") {
                Some("Work item status is lifecycle-owned. Use `govctl work move`.")
            } else if fp.segments.len() == 2
                && fp.segments[0].name == "acceptance_criteria"
                && fp.segments[1].name == "status"
            {
                Some("Acceptance criteria status is tick-owned. Use `govctl work tick`.")
            } else {
                None
            }
        }
        ArtifactType::Guard => None,
    };

    if let Some(message) = msg {
        return Err(Diagnostic::new(
            DiagnosticCode::E0804FieldNotEditable,
            format!("{message} (field: `{path}`)"),
            id,
        )
        .into());
    }

    Ok(())
}
