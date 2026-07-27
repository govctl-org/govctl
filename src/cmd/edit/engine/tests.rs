use super::super::ArtifactType;
use super::super::path::PathSegment;
use super::*;
use crate::diagnostic::DiagnosticCode;

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
            verbs: &["get", "set"],
        })
    );
    Ok(())
}

#[test]
fn test_plan_nested_path() -> Result<(), Box<dyn std::error::Error>> {
    let plan = plan_request("ADR-0001", Some("alternatives[0].pros[1]"))?;
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
fn test_plan_unknown_artifact_fails() -> Result<(), Box<dyn std::error::Error>> {
    let diag = match plan_request("UNKNOWN", Some("title")) {
        Ok(plan) => return Err(format!("unknown artifact should fail, got {plan:?}").into()),
        Err(diag) => diag,
    };
    assert_eq!(diag.code, DiagnosticCode::E0819UnknownArtifactType);
    assert!(diag.message.contains("Unknown artifact type"));
    Ok(())
}

#[test]
fn test_scope_aware_alias_only_applies_when_valid_for_artifact()
-> Result<(), Box<dyn std::error::Error>> {
    let diag = match plan_request("ADR-0001", Some("desc")) {
        Ok(plan) => return Err(format!("unknown ADR field should fail, got {plan:?}").into()),
        Err(diag) => diag,
    };
    assert_eq!(diag.code, DiagnosticCode::E0803UnknownField);
    assert!(diag.message.contains("Unknown ADR field"));
    Ok(())
}

#[test]
fn test_noncanonical_work_short_name_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let diag = match plan_request("WI-2026-01-01-001", Some("desc")) {
        Ok(plan) => return Err(format!("noncanonical field should fail, got {plan:?}").into()),
        Err(diag) => diag,
    };
    assert_eq!(diag.code, DiagnosticCode::E0803UnknownField);
    Ok(())
}

#[test]
fn test_legacy_storage_prefix_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let diag = match plan_request("WI-2026-01-01-001", Some("content.description")) {
        Ok(plan) => return Err(format!("storage-prefixed field should fail, got {plan:?}").into()),
        Err(diag) => diag,
    };
    assert_eq!(diag.code, DiagnosticCode::E0803UnknownField);
    Ok(())
}

#[test]
fn test_unknown_alias_in_scope_is_not_rewritten() -> Result<(), Box<dyn std::error::Error>> {
    let diag = match plan_request("WI-2026-01-01-001", Some("alt[0].pro[0]")) {
        Ok(plan) => {
            return Err(format!("unknown work item field should fail, got {plan:?}").into());
        }
        Err(diag) => diag,
    };
    assert_eq!(diag.code, DiagnosticCode::E0803UnknownField);
    assert!(diag.message.contains("Unknown work item field"));
    Ok(())
}

#[test]
fn test_plan_mutation_request_records_verb() -> Result<(), Box<dyn std::error::Error>> {
    let plan = plan_mutation_request("ADR-0001", "decision", Verb::Set)?;
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
            verbs: &["get", "set"],
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
            container_verbs: &["get", "add", "remove", "tick"],
            item_verbs: &["get"],
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
            container_verbs: &["get", "add", "remove"],
            item_verbs: &["get", "set"],
        })
    );
    Ok(())
}

#[test]
fn test_unknown_root_reports_registry_fields() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = match plan_request("CONF-CASE", Some("requirement")) {
        Err(diagnostic) => diagnostic,
        Ok(plan) => return Err(format!("unknown field should fail, got {plan:?}").into()),
    };

    assert_eq!(diagnostic.code, DiagnosticCode::E0803UnknownField);
    assert!(diagnostic.message.contains("Known fields:"));
    for field in [
        "guards",
        "path",
        "requirements",
        "selector",
        "tags",
        "title",
    ] {
        assert!(diagnostic.message.contains(field), "{diagnostic:?}");
    }
    Ok(())
}

#[test]
fn test_unknown_nested_field_reports_sibling_fields() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = match plan_request("WI-2026-01-01-001", Some("acceptance_criteria[0].bogus")) {
        Err(diagnostic) => diagnostic,
        Ok(plan) => return Err(format!("unknown nested field should fail, got {plan:?}").into()),
    };

    assert_eq!(diagnostic.code, DiagnosticCode::E0815PathFieldNotFound);
    assert!(
        diagnostic
            .message
            .contains("under 'acceptance_criteria[0]'")
    );
    assert!(
        diagnostic
            .message
            .contains("Known fields: category, status, text")
    );
    Ok(())
}

#[test]
fn test_unsupported_verb_reports_path_and_supported_operations()
-> Result<(), Box<dyn std::error::Error>> {
    let target = plan_mutation_request("CONF-CASE", "requirements", Verb::Set)?
        .target
        .ok_or("mutation target should exist")?;
    let diagnostic = match target.ensure_supports(Verb::Set, "CONF-CASE") {
        Err(diagnostic) => diagnostic,
        Ok(()) => return Err("requirements should reject set".into()),
    };

    assert_eq!(diagnostic.code, DiagnosticCode::E0817PathTypeMismatch);
    assert!(
        diagnostic
            .message
            .contains("Path 'requirements' does not support --set")
    );
    assert!(
        diagnostic
            .message
            .contains("Supported operations: --add, --remove")
    );
    Ok(())
}

#[test]
fn test_indexed_target_combines_item_and_container_operations()
-> Result<(), Box<dyn std::error::Error>> {
    let target = plan_mutation_request("ADR-0001", "alternatives[0].pros[1]", Verb::Tick)?
        .target
        .ok_or("mutation target should exist")?;
    let diagnostic = match target.ensure_supports(Verb::Tick, "ADR-0001") {
        Err(diagnostic) => diagnostic,
        Ok(()) => return Err("pros item should reject tick".into()),
    };

    assert!(
        diagnostic
            .message
            .contains("Supported operations: --set, --remove")
    );
    Ok(())
}
