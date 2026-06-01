use super::TomlEditableEntry;
use crate::cmd::edit::adapter::TomlAdapter;
use crate::cmd::edit::engine as edit_engine;
use crate::cmd::edit::target_doc::{NestedGetMode, render_target_from_doc};
use crate::cmd::edit::{ArtifactType, serialize_edit_doc};
use crate::cmd::output::print_toml;
use crate::config::Config;
use crate::diagnostic::{DiagnosticCode, DiagnosticResult};

pub(in crate::cmd::edit) fn get_toml_field<A>(
    config: &Config,
    id: &str,
    target: Option<&edit_engine::ResolvedTarget>,
    artifact: ArtifactType,
) -> DiagnosticResult<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let entry = A::load(config, id)?;
    if let Some(target) = target {
        let doc = serialize_edit_doc(entry.spec(), id)?;
        println!(
            "{}",
            render_target_from_doc(artifact, &doc, target, id, NestedGetMode::Allow)?
        );
    } else {
        print_toml(
            entry.spec(),
            DiagnosticCode::E0903UnexpectedError,
            "Failed to serialize editable document TOML",
            id,
        )?;
    }
    Ok(())
}
