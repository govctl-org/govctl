use super::super::ArtifactType;
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
