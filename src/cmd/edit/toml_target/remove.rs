use super::TomlEditableEntry;
use crate::cmd::edit::adapter::TomlAdapter;
use crate::cmd::edit::engine as edit_engine;
use crate::cmd::edit::matching::MatchOptions;
use crate::cmd::edit::target_doc_remove::{notify_removed, remove_target_from_doc};
use crate::cmd::edit::{ArtifactType, deserialize_edit_doc, serialize_edit_doc};
use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::write::WriteOp;

pub(in crate::cmd::edit) fn remove_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
) -> DiagnosticResult<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serialize_edit_doc(entry.spec(), id)?;
    let (display_field, removed) = remove_target_from_doc(artifact, &mut doc, id, target, opts)?;

    *entry.spec_mut() = deserialize_edit_doc(doc, id)?;
    A::write(config, &entry, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}
