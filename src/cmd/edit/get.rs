use super::ArtifactType;
use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, GuardTomlAdapter, RfcTomlAdapter, WorkTomlAdapter,
};
use super::doc_target::get_doc_field;
use super::engine as edit_engine;
use super::toml_target::get_toml_field;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticResult};

pub fn get_field(
    config: &Config,
    id: &str,
    field: Option<&str>,
) -> DiagnosticResult<Vec<Diagnostic>> {
    let plan = edit_engine::plan_request(id, field)?;
    match plan.artifact {
        ArtifactType::Adr => {
            get_toml_field::<AdrTomlAdapter>(config, id, plan.target.as_ref(), ArtifactType::Adr)?
        }
        ArtifactType::WorkItem => get_toml_field::<WorkTomlAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::WorkItem,
        )?,
        ArtifactType::Rfc => get_doc_field::<RfcTomlAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::Rfc,
            "RFC fields do not support nested paths",
        )?,
        ArtifactType::Clause => get_doc_field::<ClauseTomlAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::Clause,
            "Clause fields do not support nested paths",
        )?,
        ArtifactType::Guard => get_toml_field::<GuardTomlAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::Guard,
        )?,
    }

    Ok(vec![])
}
