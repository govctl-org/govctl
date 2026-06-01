use super::super::{
    ArtifactType,
    adapter::DocAdapter,
    engine as edit_engine, serialize_edit_doc,
    target_doc::{NestedGetMode, render_target_from_doc},
};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

pub(in crate::cmd::edit) fn get_json_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
    nested_error: &str,
) -> DiagnosticResult<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let loaded = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serialize_edit_doc(&loaded.data, id)?;
        println!(
            "{}",
            render_target_from_doc(
                artifact,
                &doc,
                target,
                id,
                NestedGetMode::Reject(nested_error)
            )?
        );
    } else {
        let json = serde_json::to_string_pretty(&loaded.data).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                format!("Failed to serialize editable document JSON: {err}"),
                id,
            )
        })?;
        println!("{json}");
    }
    Ok(())
}
