use super::ArtifactType;
use super::adapter::{
    AdrTomlAdapter, ClauseTomlAdapter, ConformanceTomlAdapter, GuardTomlAdapter, RfcTomlAdapter,
    WorkTomlAdapter,
};
use super::doc_target::{get_doc_field, rfc_changelog};
use super::engine as edit_engine;
use super::toml_target::get_toml_field;
use crate::GetOutputFormat;
use crate::cmd::output::print_get_output;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

pub fn get_field(
    config: &Config,
    id: &str,
    field: Option<&str>,
    output: Option<GetOutputFormat>,
) -> DiagnosticResult<Vec<Diagnostic>> {
    let plan = edit_engine::plan_request(id, field)?;
    let value = match plan.artifact {
        ArtifactType::Adr => {
            get_toml_field::<AdrTomlAdapter>(config, id, plan.target.as_ref(), ArtifactType::Adr)?
        }
        ArtifactType::WorkItem => get_toml_field::<WorkTomlAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::WorkItem,
        )?,
        ArtifactType::Rfc => {
            if let Some(target) = plan
                .target
                .as_ref()
                .filter(|target| rfc_changelog::is_target(target))
            {
                rfc_changelog::get(config, id, target)?
            } else {
                get_doc_field::<RfcTomlAdapter>(
                    config,
                    id,
                    plan.target.as_ref(),
                    ArtifactType::Rfc,
                    "RFC fields do not support nested paths",
                )?
            }
        }
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
        ArtifactType::Conformance => get_toml_field::<ConformanceTomlAdapter>(
            config,
            id,
            plan.target.as_ref(),
            ArtifactType::Conformance,
        )?,
    };
    print_get_output(
        &value,
        field.is_some(),
        output,
        DiagnosticCode::E0903UnexpectedError,
        id,
    )?;

    Ok(vec![])
}
