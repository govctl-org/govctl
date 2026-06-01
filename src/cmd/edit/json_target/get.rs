use super::super::{
    ArtifactType, adapter::DocAdapter, engine as edit_engine, runtime as edit_runtime,
};
use super::require_simple_field;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};

pub(in crate::cmd::edit) fn get_json_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let loaded = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serde_json::to_value(&loaded.data)?;
        match target {
            edit_engine::ResolvedTarget::Node {
                origin: edit_engine::TargetOrigin::Simple,
                path,
                ..
            } => {
                let simple = require_simple_field(path, id, nested_error)?;
                println!(
                    "{}",
                    edit_runtime::get_simple_field(artifact, &doc, simple, id)?
                );
            }
            edit_engine::ResolvedTarget::IndexedItem {
                origin: edit_engine::TargetOrigin::Simple,
                container_path,
                index,
                ..
            } => {
                let simple = require_simple_field(container_path, id, nested_error)?;
                println!(
                    "{}",
                    edit_runtime::get_simple_list_item(artifact, &doc, simple, *index, id)?
                );
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticCode::E0817PathTypeMismatch,
                    nested_error,
                    id,
                )
                .into());
            }
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&loaded.data)?);
    }
    Ok(())
}
