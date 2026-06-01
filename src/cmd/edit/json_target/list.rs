use super::super::{
    ArtifactType,
    adapter::DocAdapter,
    engine as edit_engine,
    matching::MatchOptions,
    runtime as edit_runtime,
    target_doc::{cannot_add_to_field_error, notify_removed, remove_target_from_doc},
};
use super::require_simple_field;
use crate::config::Config;
use crate::diagnostic::{Diagnostic, DiagnosticCode};
use crate::write::WriteOp;

pub(in crate::cmd::edit) fn add_json_simple_list_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    value: &str,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let edit_engine::ResolvedTarget::Node {
        path,
        kind: edit_engine::TargetKind::List,
        origin: edit_engine::TargetOrigin::Simple,
        ..
    } = target
    else {
        return Err(
            Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, nested_error, id).into(),
        );
    };
    let simple = require_simple_field(path, id, nested_error)?;
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    if !edit_runtime::add_simple_list_value(artifact, &mut doc, simple, value, id)? {
        return Err(cannot_add_to_field_error(id, simple));
    }
    loaded.data = serde_json::from_value(doc)?;
    A::write(config, &loaded, op)?;
    Ok(())
}

pub(in crate::cmd::edit) fn remove_json_simple_list_field<A>(
    config: &Config,
    id: &str,
    target: &edit_engine::ResolvedTarget,
    opts: &MatchOptions,
    op: WriteOp,
    artifact: ArtifactType,
    nested_error: &str,
) -> anyhow::Result<()>
where
    A: DocAdapter,
    A::Data: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut loaded = A::load(config, id)?;
    let mut doc = serde_json::to_value(&loaded.data)?;
    let (display_field, removed) = remove_target_from_doc(artifact, &mut doc, id, target, opts)?;
    if !matches!(
        target,
        edit_engine::ResolvedTarget::Node {
            origin: edit_engine::TargetOrigin::Simple,
            ..
        } | edit_engine::ResolvedTarget::IndexedItem {
            origin: edit_engine::TargetOrigin::Simple,
            ..
        }
    ) {
        return Err(
            Diagnostic::new(DiagnosticCode::E0817PathTypeMismatch, nested_error, id).into(),
        );
    }
    loaded.data = serde_json::from_value(doc)?;
    A::write(config, &loaded, op)?;
    notify_removed(id, &display_field, &removed, op);
    Ok(())
}
