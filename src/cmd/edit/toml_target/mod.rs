mod set;
mod tick;
mod work_dependencies;

use super::adapter::TomlAdapter;
use super::engine as edit_engine;
use super::matching::MatchOptions;
use super::target_doc::{NestedGetMode, render_target_from_doc};
use super::target_doc_remove::{notify_removed, remove_target_from_doc};
use super::{ArtifactType, deserialize_edit_doc, serialize_edit_doc};
use crate::cmd::output::print_toml;
use crate::config::Config;
use crate::diagnostic::{DiagnosticCode, DiagnosticResult};
use crate::model::{AdrEntry, AdrSpec, GuardEntry, GuardSpec, WorkItemEntry, WorkItemSpec};
use crate::write::WriteOp;
pub(super) use set::{set_toml_field, set_work_toml_field};
use tick::tick_target_in_doc;
pub(super) use work_dependencies::{is_work_dependency_target, validate_work_dependency_edit};

pub(super) trait TomlEditableEntry {
    type Spec: serde::Serialize + serde::de::DeserializeOwned;
    fn spec(&self) -> &Self::Spec;
    fn spec_mut(&mut self) -> &mut Self::Spec;
}

impl TomlEditableEntry for AdrEntry {
    type Spec = AdrSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

impl TomlEditableEntry for WorkItemEntry {
    type Spec = WorkItemSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

impl TomlEditableEntry for GuardEntry {
    type Spec = GuardSpec;
    fn spec(&self) -> &Self::Spec {
        &self.spec
    }
    fn spec_mut(&mut self) -> &mut Self::Spec {
        &mut self.spec
    }
}

pub(super) fn get_toml_field<A>(
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

pub(super) fn remove_toml_field<A>(
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

pub(super) fn tick_toml_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    status_str: &str,
) -> DiagnosticResult<String>
where
    A: TomlAdapter,
    A::Entry: TomlEditableEntry,
{
    let mut entry = A::load(config, id)?;
    let mut doc = serialize_edit_doc(entry.spec(), id)?;
    let ticked_text = tick_target_in_doc(artifact, &mut doc, id, target, opts, status_str)?;
    *entry.spec_mut() = deserialize_edit_doc(doc, id)?;
    A::write(config, &entry, op)?;
    Ok(ticked_text)
}
