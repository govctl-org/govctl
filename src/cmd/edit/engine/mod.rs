//! V2 edit engine planning pipeline (ADR-0031 foundation).
//!
//! This module introduces a single entry point for edit request planning:
//! `parse -> canonicalize -> resolve -> classify`.
//! Execution is still delegated to legacy handlers during migration.

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

/// Build a migration-safe V2 plan from command inputs.
///
/// During migration this function intentionally does not enforce verb/field
/// capability checks; those remain in the existing execution path.
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
mod tests {
    use super::super::path::PathSegment;
    use super::*;

    #[test]
    fn test_plan_simple_path() -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_request("ADR-0001", Some("title"))?;
        assert_eq!(plan.artifact, ArtifactType::Adr);
        assert_eq!(
            plan.field_path.as_ref().and_then(FieldPath::as_simple),
            Some("title")
        );
        assert_eq!(plan.verb, None);
        assert_eq!(
            plan.target,
            Some(ResolvedTarget::Node {
                origin: TargetOrigin::Simple,
                path: FieldPath {
                    segments: vec![PathSegment {
                        name: "title".to_string(),
                        index: None,
                    }],
                },
                kind: TargetKind::Scalar,
                status_list: false,
            })
        );
        Ok(())
    }

    #[test]
    fn test_plan_nested_path() -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_request("ADR-0001", Some("alt[0].pro[1]"))?;
        let fp = plan
            .field_path
            .as_ref()
            .ok_or("nested field should exist")?;
        assert_eq!(fp.segments[0].name, "alternatives");
        assert_eq!(fp.segments[1].name, "pros");
        assert_eq!(plan.verb, None);
        Ok(())
    }

    #[test]
    fn test_plan_without_field() -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_request("ADR-0001", None)?;
        assert_eq!(plan.artifact, ArtifactType::Adr);
        assert!(plan.field_path.is_none());
        assert_eq!(plan.verb, None);
        assert_eq!(plan.target, None);
        Ok(())
    }

    #[test]
    fn test_plan_unknown_artifact_fails() {
        let err = plan_request("UNKNOWN", Some("title"));
        assert!(err.is_err());
        assert!(
            err.err()
                .map(|e| e.to_string())
                .unwrap_or_default()
                .contains("Unknown artifact type")
        );
    }

    #[test]
    fn test_scope_aware_alias_only_applies_when_valid_for_artifact() {
        let err = plan_request("ADR-0001", Some("desc"));
        assert!(err.is_err());
        assert!(
            err.err()
                .map(|e| e.to_string())
                .unwrap_or_default()
                .contains("Unknown ADR field")
        );
    }

    #[test]
    fn test_scope_aware_alias_keeps_work_short_name() -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_request("WI-2026-01-01-001", Some("desc"))?;
        let fp = plan.field_path.ok_or("field path should exist")?;
        assert_eq!(fp.as_simple(), Some("description"));
        Ok(())
    }

    #[test]
    fn test_scope_aware_alias_under_legacy_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_request("WI-2026-01-01-001", Some("content.desc"))?;
        let fp = plan.field_path.ok_or("field path should exist")?;
        assert_eq!(fp.as_simple(), Some("description"));
        Ok(())
    }

    #[test]
    fn test_unknown_alias_in_scope_is_not_rewritten() {
        let err = plan_request("WI-2026-01-01-001", Some("alt[0].pro[0]"));
        assert!(err.is_err());
        assert!(
            err.err()
                .map(|e| e.to_string())
                .unwrap_or_default()
                .contains("Unknown work item field")
        );
    }

    #[test]
    fn test_plan_mutation_request_records_verb() -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_mutation_request("ADR-0001", "content.decision", Verb::Set)?;
        assert_eq!(plan.verb, Some(Verb::Set));
        assert_eq!(
            plan.field_path
                .and_then(|fp| fp.as_simple().map(str::to_owned)),
            Some("decision".to_string())
        );
        assert_eq!(
            plan.target,
            Some(ResolvedTarget::Node {
                origin: TargetOrigin::Simple,
                path: FieldPath {
                    segments: vec![PathSegment {
                        name: "decision".to_string(),
                        index: None,
                    }],
                },
                kind: TargetKind::Scalar,
                status_list: false,
            })
        );
        Ok(())
    }

    #[test]
    fn test_plan_mutation_request_classifies_nested_root_item_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_mutation_request("ADR-0001", "alternatives[0]", Verb::Remove)?;
        assert_eq!(
            plan.target,
            Some(ResolvedTarget::IndexedItem {
                origin: TargetOrigin::Nested,
                path: FieldPath {
                    segments: vec![PathSegment {
                        name: "alternatives".to_string(),
                        index: Some(0),
                    }],
                },
                container_path: FieldPath {
                    segments: vec![PathSegment {
                        name: "alternatives".to_string(),
                        index: None,
                    }],
                },
                index: 0,
                item_kind: TargetKind::Object,
                status_list: true,
            })
        );
        Ok(())
    }

    #[test]
    fn test_plan_mutation_request_classifies_nested_list_item_target()
    -> Result<(), Box<dyn std::error::Error>> {
        let plan = plan_mutation_request("ADR-0001", "alternatives[0].pros[1]", Verb::Remove)?;
        assert_eq!(
            plan.target,
            Some(ResolvedTarget::IndexedItem {
                origin: TargetOrigin::Nested,
                path: FieldPath {
                    segments: vec![
                        PathSegment {
                            name: "alternatives".to_string(),
                            index: Some(0),
                        },
                        PathSegment {
                            name: "pros".to_string(),
                            index: Some(1),
                        },
                    ],
                },
                container_path: FieldPath {
                    segments: vec![
                        PathSegment {
                            name: "alternatives".to_string(),
                            index: Some(0),
                        },
                        PathSegment {
                            name: "pros".to_string(),
                            index: None,
                        },
                    ],
                },
                index: 1,
                item_kind: TargetKind::Scalar,
                status_list: false,
            })
        );
        Ok(())
    }
}
