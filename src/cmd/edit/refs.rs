use super::ArtifactType;
use super::engine as edit_engine;
use super::path::FieldPath;
use crate::config::Config;
use crate::diagnostic::DiagnosticResult;

pub(super) fn is_refs_target(target: &edit_engine::ResolvedTarget) -> bool {
    match target {
        edit_engine::ResolvedTarget::Node { path, .. } => is_refs_path(path),
        edit_engine::ResolvedTarget::IndexedItem { container_path, .. } => {
            is_refs_path(container_path)
        }
    }
}

pub(super) fn validate_ref_edit(
    config: &Config,
    artifact: ArtifactType,
    owner_id: &str,
    ref_id: &str,
) -> DiagnosticResult<()> {
    match artifact {
        ArtifactType::Rfc | ArtifactType::Adr | ArtifactType::WorkItem => {
            crate::validate::validate_artifact_ref_edit(config, owner_id, ref_id, owner_id)
        }
        ArtifactType::Clause | ArtifactType::Guard => Ok(()),
    }
}

fn is_refs_path(path: &FieldPath) -> bool {
    path.as_simple() == Some("refs") || path.to_string() == "govctl.refs"
}
