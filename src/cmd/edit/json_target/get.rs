use super::super::{
    ArtifactType,
    adapter::DocAdapter,
    engine as edit_engine, serialize_edit_doc,
    target_doc::{NestedGetMode, render_target_from_doc},
};
use crate::cmd::output::print_json;
use crate::config::Config;
use crate::diagnostic::{DiagnosticCode, DiagnosticResult};

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
        print_json(
            &loaded.data,
            DiagnosticCode::E0903UnexpectedError,
            "Failed to serialize editable document JSON",
            id,
        )?;
    }
    Ok(())
}
