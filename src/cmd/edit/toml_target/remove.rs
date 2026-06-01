use super::TomlEditableEntry;
use crate::cmd::edit::ArtifactType;
use crate::cmd::edit::adapter::TomlAdapter;
use crate::cmd::edit::engine as edit_engine;
use crate::cmd::edit::matching::MatchOptions;
use crate::cmd::edit::target_doc::{notify_removed, remove_target_from_doc};
use crate::config::Config;
use crate::write::WriteOp;

pub(in crate::cmd::edit) fn remove_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
) -> anyhow::Result<()>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serde_json::to_value(entry.spec())?;
    let (display_field, removed) = remove_target_from_doc(artifact, &mut doc, id, target, opts)?;

    *entry.spec_mut() = serde_json::from_value(doc)?;
    A::write(config, &entry, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}
