mod get;
mod remove;
mod set;
mod tick;
mod work_dependencies;

use super::adapter::TomlAdapter;
use super::engine as edit_engine;
use super::matching::MatchOptions;
use super::{ArtifactType, deserialize_edit_doc, serialize_edit_doc};
use crate::config::Config;
use crate::diagnostic::DiagnosticResult;
use crate::model::{AdrEntry, AdrSpec, GuardEntry, GuardSpec, WorkItemEntry, WorkItemSpec};
use crate::write::WriteOp;
pub(super) use get::get_toml_field;
pub(super) use remove::remove_toml_field;
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
