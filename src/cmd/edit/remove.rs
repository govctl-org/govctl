use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, GuardTomlAdapter, RfcTomlAdapter, WorkTomlAdapter,
};
use super::doc_target::remove_doc_simple_list_field;
use super::matching::MatchOptions;
use super::rules as edit_rules;
use super::toml_target::remove_toml_field;
use super::{ArtifactType, plan_mutation_target, reject_match_flags_for_indexed_target};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};
use crate::write::WriteOp;

pub fn remove_from_field(
    config: &Config,
    id: &str,
    field: &str,
    opts: &MatchOptions,
    op: WriteOp,
) -> DiagnosticResult<Vec<Diagnostic>> {
    let plan = plan_mutation_target(id, field, edit_rules::Verb::Remove)?;
    let artifact = plan.artifact;
    let target = &plan.target;
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
        ArtifactType::Rfc => remove_doc_simple_list_field::<RfcTomlAdapter>(
            config,
            id,
            target,
            opts,
            op,
            ArtifactType::Rfc,
            "RFC fields do not support nested paths for remove",
        )?,
        ArtifactType::Clause => remove_doc_simple_list_field::<ClauseTomlAdapter>(
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
