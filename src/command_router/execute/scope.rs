use super::super::Scope;
use crate::ListTarget;
use crate::cmd;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShowKind {
    Rfc,
    Clause,
    Adr,
    Work,
    Guard,
}

impl ShowKind {
    pub(super) fn from_artifact(artifact: cmd::edit::ArtifactType) -> Self {
        match artifact {
            cmd::edit::ArtifactType::Rfc => Self::Rfc,
            cmd::edit::ArtifactType::Clause => Self::Clause,
            cmd::edit::ArtifactType::Adr => Self::Adr,
            cmd::edit::ArtifactType::WorkItem => Self::Work,
            cmd::edit::ArtifactType::Guard => Self::Guard,
        }
    }
}

pub(super) fn extract_artifact_scope(
    scope: &Scope,
) -> DiagnosticResult<(cmd::edit::ArtifactType, &str)> {
    match scope {
        Scope::Artifact { artifact, id } => Ok((*artifact, id.as_str())),
        Scope::Target { artifact, id, .. } => Ok((*artifact, id.as_str())),
        Scope::Global | Scope::Collection { .. } => Err(Diagnostic::new(
            DiagnosticCode::E0821InvalidCommandScope,
            "Expected artifact scope",
            "command router",
        )),
    }
}

pub(super) fn extract_target_scope(
    scope: &Scope,
) -> DiagnosticResult<(
    cmd::edit::ArtifactType,
    &str,
    &cmd::edit::engine::ResolvedTarget,
)> {
    match scope {
        Scope::Target {
            artifact,
            id,
            target,
        } => Ok((*artifact, id.as_str(), target)),
        Scope::Global | Scope::Collection { .. } | Scope::Artifact { .. } => Err(Diagnostic::new(
            DiagnosticCode::E0821InvalidCommandScope,
            "Expected target scope",
            "command router",
        )),
    }
}

pub(super) fn extract_collection_scope(scope: &Scope) -> DiagnosticResult<ListTarget> {
    match scope {
        Scope::Collection { target } => Ok(*target),
        Scope::Global | Scope::Artifact { .. } | Scope::Target { .. } => Err(Diagnostic::new(
            DiagnosticCode::E0821InvalidCommandScope,
            "Expected collection scope",
            "command router",
        )),
    }
}
