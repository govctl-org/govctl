use crate::diagnostic::{Diagnostic, DiagnosticCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    Clause,
    Rfc,
    Adr,
    WorkItem,
    Guard,
}

impl ArtifactType {
    pub fn from_id(id: &str) -> Option<Self> {
        if id.contains(':') {
            Some(Self::Clause)
        } else if id.starts_with("RFC-") {
            Some(Self::Rfc)
        } else if id.starts_with("ADR-") {
            Some(Self::Adr)
        } else if id.starts_with("GUARD-") {
            Some(Self::Guard)
        } else if id.starts_with("WI-") || id.contains('-') {
            Some(Self::WorkItem)
        } else {
            None
        }
    }

    pub fn unknown_error(id: &str) -> Diagnostic {
        Diagnostic::new(
            DiagnosticCode::E0819UnknownArtifactType,
            format!("Unknown artifact type: {id}"),
            id,
        )
    }

    pub fn rule_key(self) -> &'static str {
        match self {
            Self::Clause => "clause",
            Self::Rfc => "rfc",
            Self::Adr => "adr",
            Self::WorkItem => "work",
            Self::Guard => "guard",
        }
    }
}

#[cfg(test)]
#[path = "artifact_tests.rs"]
mod tests;
