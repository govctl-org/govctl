//! V2 edit engine planning pipeline (ADR-0031 foundation).
//!
//! This module introduces a single entry point for edit request planning:
//! `parse -> resolve -> classify`.
//! Execution remains in the command-specific handlers; this module owns the
//! shared canonical planning step.

mod resolve;

use self::resolve::resolve_target;
use super::ArtifactType;
use super::path::{self, FieldPath};
use super::rules::Verb;
use crate::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Scalar,
    List,
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetOrigin {
    Simple,
    Nested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTarget {
    Node {
        origin: TargetOrigin,
        path: FieldPath,
        kind: TargetKind,
        status_list: bool,
        verbs: &'static [&'static str],
    },
    IndexedItem {
        origin: TargetOrigin,
        path: FieldPath,
        container_path: FieldPath,
        index: i32,
        item_kind: TargetKind,
        status_list: bool,
        container_verbs: &'static [&'static str],
        item_verbs: &'static [&'static str],
    },
}

impl ResolvedTarget {
    pub fn display_path(&self) -> String {
        match self {
            Self::Node { path, .. } | Self::IndexedItem { path, .. } => path.to_string(),
        }
    }

    pub fn path(&self) -> &FieldPath {
        match self {
            Self::Node { path, .. } | Self::IndexedItem { path, .. } => path,
        }
    }

    pub fn ensure_supports(&self, verb: Verb, id: &str) -> DiagnosticResult<()> {
        if self.supports(verb) {
            return Ok(());
        }

        let supported = [Verb::Set, Verb::Add, Verb::Remove, Verb::Tick]
            .into_iter()
            .filter(|candidate| self.supports(*candidate))
            .map(|candidate| format!("--{}", candidate.as_str()))
            .collect::<Vec<_>>();
        let message = if supported.is_empty() {
            format!("Path '{}' is read-only", self.display_path())
        } else {
            format!(
                "Path '{}' does not support --{}. Supported operations: {}",
                self.display_path(),
                verb.as_str(),
                supported.join(", ")
            )
        };
        Err(Diagnostic::new(
            DiagnosticCode::E0817PathTypeMismatch,
            message,
            id,
        ))
    }

    fn supports(&self, verb: Verb) -> bool {
        match self {
            Self::Node { verbs, .. } => verbs.contains(&verb.as_str()),
            Self::IndexedItem {
                container_verbs,
                item_verbs,
                ..
            } => match verb {
                Verb::Get | Verb::Set => item_verbs.contains(&verb.as_str()),
                Verb::Remove | Verb::Tick => container_verbs.contains(&verb.as_str()),
                Verb::Add => false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPlan {
    pub artifact: ArtifactType,
    pub field_path: Option<FieldPath>,
    pub verb: Option<Verb>,
    pub target: Option<ResolvedTarget>,
}

/// Parse a canonical user field expression.
pub fn parse_and_canonicalize_field(
    _artifact: ArtifactType,
    field: &str,
) -> DiagnosticResult<FieldPath> {
    path::parse_raw_field_path(field)
}

/// Build a command-handler-safe plan from command inputs.
///
/// This function intentionally does not enforce verb/field capability checks;
/// those remain in the command-specific execution path.
pub fn plan_request(id: &str, field: Option<&str>) -> DiagnosticResult<TargetPlan> {
    plan_request_with_verb(id, field, None)
}

pub fn plan_mutation_request(id: &str, field: &str, verb: Verb) -> DiagnosticResult<TargetPlan> {
    plan_request_with_verb(id, Some(field), Some(verb))
}

fn plan_request_with_verb(
    id: &str,
    field: Option<&str>,
    verb: Option<Verb>,
) -> DiagnosticResult<TargetPlan> {
    let artifact = resolve_artifact(id)?;
    let field_path = field
        .map(|path| parse_and_canonicalize_field(artifact, path))
        .transpose()?;
    let target = field_path
        .as_ref()
        .map(|field_path| resolve_target(artifact, field_path, id))
        .transpose()?;
    Ok(TargetPlan {
        artifact,
        field_path,
        verb,
        target,
    })
}

fn resolve_artifact(id: &str) -> DiagnosticResult<ArtifactType> {
    ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))
}

#[cfg(test)]
mod tests;
