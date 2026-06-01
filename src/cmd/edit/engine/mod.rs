//! V2 edit engine planning pipeline (ADR-0031 foundation).
//!
//! This module introduces a single entry point for edit request planning:
//! `parse -> canonicalize -> resolve -> classify`.
//! Execution remains in the command-specific handlers; this module owns the
//! shared canonical planning step.

mod resolve;

use self::resolve::resolve_target;
use super::ArtifactType;
use super::path::{self, FieldPath};
use super::rules::{self as edit_rules, Verb};

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
    },
    IndexedItem {
        origin: TargetOrigin,
        path: FieldPath,
        container_path: FieldPath,
        index: i32,
        item_kind: TargetKind,
        status_list: bool,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPlan {
    pub artifact: ArtifactType,
    pub field_path: Option<FieldPath>,
    pub verb: Option<Verb>,
    pub target: Option<ResolvedTarget>,
}

/// Parse and canonicalize a user field expression using current path rules.
pub fn parse_and_canonicalize_field(
    artifact: ArtifactType,
    field: &str,
) -> anyhow::Result<FieldPath> {
    path::parse_raw_field_path(field).map(|fp| canonicalize_field_path(artifact, fp))
}

/// Build a command-handler-safe plan from command inputs.
///
/// This function intentionally does not enforce verb/field capability checks;
/// those remain in the command-specific execution path.
pub fn plan_request(id: &str, field: Option<&str>) -> anyhow::Result<TargetPlan> {
    plan_request_with_verb(id, field, None)
}

pub fn plan_mutation_request(id: &str, field: &str, verb: Verb) -> anyhow::Result<TargetPlan> {
    plan_request_with_verb(id, Some(field), Some(verb))
}

fn plan_request_with_verb(
    id: &str,
    field: Option<&str>,
    verb: Option<Verb>,
) -> anyhow::Result<TargetPlan> {
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

fn resolve_artifact(id: &str) -> anyhow::Result<ArtifactType> {
    ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))
}

fn canonicalize_field_path(artifact: ArtifactType, mut fp: FieldPath) -> FieldPath {
    let artifact_key = artifact.rule_key();
    // canonicalize_field_path intentionally canonicalizes root/second segments
    // both before and after collapse_legacy_prefixes() so paths like
    // content.alt[0].pro[0] still end up fully canonical after
    // collapse_legacy_prefixes, canonicalize_root_segment, and
    // canonicalize_subfield_segment interact.
    if let Some(seg0) = fp.segments.first_mut() {
        seg0.name = canonicalize_root_segment(artifact_key, &seg0.name);
    }
    if fp.segments.len() >= 2 {
        let root = fp.segments[0].name.clone();
        let seg1 = &mut fp.segments[1];
        seg1.name = canonicalize_subfield_segment(artifact_key, &root, &seg1.name);
    }
    fp = fp.collapse_legacy_prefixes();
    if let Some(seg0) = fp.segments.first_mut() {
        seg0.name = canonicalize_root_segment(artifact_key, &seg0.name);
    }
    if fp.segments.len() >= 2 {
        let root = fp.segments[0].name.clone();
        let seg1 = &mut fp.segments[1];
        seg1.name = canonicalize_subfield_segment(artifact_key, &root, &seg1.name);
    }
    fp
}

fn canonicalize_root_segment(artifact: &str, token: &str) -> String {
    if is_known_root_field(artifact, token) {
        return token.to_string();
    }
    let alias = edit_rules::normalize_alias(token);
    if alias != token && is_known_root_field(artifact, alias) {
        return alias.to_string();
    }
    token.to_string()
}

fn canonicalize_subfield_segment(artifact: &str, root: &str, token: &str) -> String {
    if is_known_subfield(artifact, root, token) {
        return token.to_string();
    }
    let alias = edit_rules::normalize_alias(token);
    if alias != token && is_known_subfield(artifact, root, alias) {
        return alias.to_string();
    }
    token.to_string()
}

fn is_known_root_field(artifact: &str, field: &str) -> bool {
    edit_rules::simple_field_rule(artifact, field).is_some()
        || edit_rules::nested_root_rule(artifact, field).is_some()
}

fn is_known_subfield(artifact: &str, root: &str, field: &str) -> bool {
    edit_rules::nested_field_rule(artifact, root, field).is_some()
        || edit_rules::can_collapse_legacy_prefix(root, field)
}

#[cfg(test)]
mod tests;
