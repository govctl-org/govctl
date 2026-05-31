use super::ArtifactType;
use super::adapter::{
    AdrTomlAdapter, ClauseJsonAdapter, GuardTomlAdapter, RfcJsonAdapter, WorkTomlAdapter,
};
use super::engine as edit_engine;
use super::json_target::get_json_field;
use super::toml_target::get_toml_field;
use crate::config::Config;
use crate::diagnostic::Diagnostic;

pub fn get_field(
    config: &Config,
    id: &str,
    field: Option<&str>,
) -> anyhow::Result<Vec<Diagnostic>> {
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
        ArtifactType::Rfc => get_json_field::<RfcJsonAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::Rfc,
            "RFC fields do not support nested paths",
        )?,
        ArtifactType::Clause => get_json_field::<ClauseJsonAdapter>(
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
