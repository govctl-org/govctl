use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, GuardTomlAdapter, RfcTomlAdapter, WorkTomlAdapter,
};
use super::json_target::remove_json_simple_list_field;
use super::matching::MatchOptions;
use super::rules as edit_rules;
use super::toml_target::remove_toml_field;
use super::{ArtifactType, plan_edit_with_field_for_verb, reject_match_flags_for_indexed_target};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::WriteOp;

pub fn remove_from_field(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
) -> anyhow::Result<Vec<Diagnostic>> {
    let plan = plan_edit_with_field_for_verb(id, field, Some(edit_rules::Verb::Remove))?;
    let artifact = plan.artifact;
    let target = plan.target.as_ref().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::E0901IoError,
            "mutation planning should produce target",
            id,
        )
    })?;
    reject_match_flags_for_indexed_target(id, target, opts)?;

    match artifact {
        ArtifactType::Adr => {
            remove_toml_field::<AdrTomlAdapter>(config, id, target, opts, op, ArtifactType::Adr)?
        }
        ArtifactType::WorkItem => remove_toml_field::<WorkTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::WorkItem,
        )?,
        ArtifactType::Rfc => remove_json_simple_list_field::<RfcTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::Rfc,
            "RFC fields do not support nested paths for remove",
        )?,
        ArtifactType::Clause => remove_json_simple_list_field::<ClauseTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::Clause,
            "Clause fields do not support nested paths for remove",
        )?,
        ArtifactType::Guard => remove_toml_field::<GuardTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::Guard,
        )?,
    }

    Ok(vec![])
}
