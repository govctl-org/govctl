//! V2 edit engine planning pipeline (ADR-0031 foundation).
//!
//! This module introduces a single entry point for edit request planning:
//! `parse -> canonicalize -> resolve -> classify`.
//! Execution is still delegated to legacy handlers during migration.

use super::ArtifactType;
use super::rules as edit_rules;
use super::path::{self, FieldPath};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPlan {
    pub artifact: ArtifactType,
    pub field_path: Option<FieldPath>,
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
pub fn plan_request(id: &str, field: Option<&str>) -> anyhow::Result<EditPlan> {
    let artifact = resolve_artifact(id)?;
    let field_path = field
        .map(|path| parse_and_canonicalize_field(artifact, path))
        .transpose()?;
    Ok(EditPlan {
        artifact,
        field_path,
    })
}

fn resolve_artifact(id: &str) -> anyhow::Result<ArtifactType> {
    ArtifactType::from_id(id).ok_or_else(|| ArtifactType::unknown_error(id))
}

fn canonicalize_field_path(artifact: ArtifactType, mut fp: FieldPath) -> FieldPath {
    let artifact_key = artifact.rule_key();
    if let Some(seg0) = fp.segments.first_mut() {
        seg0.name = canonicalize_root_segment(artifact_key, &seg0.name);
    }
    if fp.segments.len() >= 2 {
        let root = fp.segments[0].name.clone();
        let seg1 = &mut fp.segments[1];
        seg1.name = canonicalize_subfield_segment(artifact_key, &root, &seg1.name);
    }
    fp.collapse_legacy_prefixes()
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
    use super::*;

    #[test]
    fn test_plan_simple_path() {
        let plan = plan_request("ADR-0001", Some("title")).unwrap();
        assert_eq!(plan.artifact, ArtifactType::Adr);
        assert_eq!(
            plan.field_path.as_ref().and_then(FieldPath::as_simple),
            Some("title")
        );
    }

    #[test]
    fn test_plan_nested_path() {
        let plan = plan_request("ADR-0001", Some("alt[0].pro[1]")).unwrap();
        let fp = plan.field_path.as_ref().expect("nested field should exist");
        assert_eq!(fp.segments[0].name, "alternatives");
        assert_eq!(fp.segments[1].name, "pros");
    }

    #[test]
    fn test_plan_without_field() {
        let plan = plan_request("ADR-0001", None).unwrap();
        assert_eq!(plan.artifact, ArtifactType::Adr);
        assert!(plan.field_path.is_none());
    }

    #[test]
    fn test_plan_unknown_artifact_fails() {
        let err = plan_request("UNKNOWN", Some("title")).unwrap_err();
        assert!(err.to_string().contains("Unknown artifact type"));
    }

    #[test]
    fn test_scope_aware_alias_only_applies_when_valid_for_artifact() {
        let plan = plan_request("ADR-0001", Some("desc")).unwrap();
        let fp = plan.field_path.expect("field path should exist");
        assert_eq!(fp.as_simple(), Some("desc"));
    }

    #[test]
    fn test_scope_aware_alias_keeps_work_short_name() {
        let plan = plan_request("WI-2026-01-01-001", Some("desc")).unwrap();
        let fp = plan.field_path.expect("field path should exist");
        assert_eq!(fp.as_simple(), Some("description"));
    }

    #[test]
    fn test_scope_aware_alias_under_legacy_prefix() {
        let plan = plan_request("WI-2026-01-01-001", Some("content.desc")).unwrap();
        let fp = plan.field_path.expect("field path should exist");
        assert_eq!(fp.as_simple(), Some("description"));
    }

    #[test]
    fn test_unknown_alias_in_scope_is_not_rewritten() {
        let plan = plan_request("WI-2026-01-01-001", Some("alt[0].pro[0]")).unwrap();
        let fp = plan.field_path.expect("field path should exist");
        assert_eq!(fp.segments[0].name, "alt");
        assert_eq!(fp.segments[1].name, "pro");
    }
}
