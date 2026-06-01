use super::TomlEditableEntry;
use crate::cmd::edit::adapter::TomlAdapter;
use crate::cmd::edit::engine as edit_engine;
use crate::cmd::edit::runtime as edit_runtime;
use crate::cmd::edit::{ArtifactType, serialize_edit_doc};
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};

pub(in crate::cmd::edit) fn get_toml_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let entry = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serialize_edit_doc(entry.spec(), id)?;
        println!("{}", render_resolved_target(artifact, &doc, target, id)?);
    } else {
        let toml = toml::to_string_pretty(entry.spec()).map_err(|err| {
            Diagnostic::new(
                DiagnosticCode::E0903UnexpectedError,
                format!("Failed to serialize editable document TOML: {err}"),
                id,
            )
        })?;
        println!("{toml}");
    }
    Ok(())
}

fn render_resolved_target(
    artifact: ArtifactType,
    doc: &serde_json::Value,
    target: &edit_engine::ResolvedTarget,
    id: &str,
) -> anyhow::Result<String> {
    match target {
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Simple,
            path,
            ..
        } => {
            let simple = path.as_simple().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "simple node target should use a simple path",
                    id,
                )
            })?;
            edit_runtime::get_simple_field(artifact, doc, simple, id)
        }
        edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Simple,
            container_path,
            index,
            ..
        } => {
            let simple = container_path.as_simple().ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticCode::E0901IoError,
                    "simple indexed target should use a simple container path",
                    id,
                )
            })?;
            edit_runtime::get_simple_list_item(artifact, doc, simple, *index, id)
        }
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Nested,
            path,
            ..
        }
        | edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Nested,
            path,
            ..
        } => edit_runtime::get_nested_field(artifact, doc, path, id),
    }
}
