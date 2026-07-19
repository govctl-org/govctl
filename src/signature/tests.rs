use super::canonical_json::canonicalize_json;
use super::*;
use crate::model::{
    ChangelogEntry, ClauseEntry, ClauseKind, ClauseSpec, ClauseStatus, RfcIndex, RfcPhase, RfcSpec,
    RfcStatus, SectionSpec,
};
use serde_json::Value;
use std::path::PathBuf;

#[test]
fn test_canonicalize_sorts_keys() -> Result<(), Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(r#"{"z": 1, "a": 2, "m": 3}"#)?;
    let canonical = canonicalize_json(&json);
    assert_eq!(canonical, r#"{"a":2,"m":3,"z":1}"#);
    Ok(())
}

#[test]
fn test_canonicalize_nested_objects() -> Result<(), Box<dyn std::error::Error>> {
    let json: Value = serde_json::from_str(r#"{"outer": {"z": 1, "a": 2}, "inner": {"b": 3}}"#)?;
    let canonical = canonicalize_json(&json);
    assert_eq!(canonical, r#"{"inner":{"b":3},"outer":{"a":2,"z":1}}"#);
    Ok(())
}

#[test]
fn test_extract_signature() {
    let md = r#"---
status: normative
---

<!-- GENERATED: do not edit. Source: RFC-0000 -->
<!-- SIGNATURE: sha256:abcd1234 -->

# RFC-0000
"#;
    assert_eq!(extract_signature(md), Some("abcd1234".to_string()));
}

#[test]
fn test_extract_signature_not_found() {
    let md = "# Just a plain markdown file";
    assert_eq!(extract_signature(md), None);
}

#[test]
fn test_rfc_content_signature_ignores_bump_bookkeeping() -> Result<(), Diagnostic> {
    let mut rfc = test_rfc_index();
    let baseline = compute_rfc_content_signature(&rfc)?;

    rfc.rfc.version = "0.1.1".to_string();
    rfc.rfc.phase = RfcPhase::Stable;
    rfc.rfc.signature = Some("legacy-or-current-signature".to_string());
    rfc.rfc.changelog.push(ChangelogEntry {
        version: "0.1.1".to_string(),
        date: "2026-06-15".to_string(),
        notes: Some("Bookkeeping only".to_string()),
        added: vec![],
        changed: vec![],
        deprecated: vec![],
        removed: vec![],
        fixed: vec!["no content change".to_string()],
        security: vec![],
    });

    assert_eq!(compute_rfc_content_signature(&rfc)?, baseline);
    assert_ne!(compute_rfc_signature(&rfc)?, baseline);
    Ok(())
}

#[test]
fn test_rfc_content_signature_includes_clause_content() -> Result<(), Diagnostic> {
    let mut rfc = test_rfc_index();
    let baseline = compute_rfc_content_signature(&rfc)?;

    rfc.clauses[0].spec.text = "Updated normative behavior.".to_string();

    assert_ne!(compute_rfc_content_signature(&rfc)?, baseline);
    Ok(())
}

#[test]
fn test_rfc_amended_accepts_legacy_full_signature_baseline() -> Result<(), Diagnostic> {
    let mut rfc = test_rfc_index();
    rfc.rfc.signature = Some(compute_rfc_signature(&rfc)?);

    assert!(!is_rfc_amended(&rfc));
    Ok(())
}

#[test]
fn test_migrated_legacy_signature_ignores_later_phase_changes() -> Result<(), Diagnostic> {
    let mut rfc = test_rfc_index();
    rfc.rfc.signature = Some(compute_rfc_signature(&rfc)?);
    assert!(!is_rfc_amended(&rfc));

    rfc.rfc.signature = Some(compute_rfc_content_signature(&rfc)?);
    rfc.rfc.phase = RfcPhase::Test;

    assert!(!is_rfc_amended(&rfc));
    Ok(())
}

#[test]
fn test_rfc_amended_ignores_open_spec_candidate_changes() -> Result<(), Diagnostic> {
    let mut rfc = test_rfc_index();
    rfc.rfc.signature = Some(compute_rfc_content_signature(&rfc)?);
    rfc.rfc.phase = RfcPhase::Spec;
    rfc.rfc.title = "Open candidate title".to_string();

    assert!(!is_rfc_amended(&rfc));
    Ok(())
}

fn test_rfc_index() -> RfcIndex {
    RfcIndex {
        rfc: RfcSpec {
            rfc_id: "RFC-0001".to_string(),
            title: "Test RFC".to_string(),
            version: "0.1.0".to_string(),
            status: RfcStatus::Normative,
            phase: RfcPhase::Impl,
            owners: vec!["@test-user".to_string()],
            created: "2026-06-15".to_string(),
            updated: None,
            supersedes: None,
            refs: vec![],
            tags: vec![],
            sections: vec![SectionSpec {
                title: "Specification".to_string(),
                clauses: vec!["C-TEST".to_string()],
            }],
            changelog: vec![],
            signature: None,
        },
        clauses: vec![ClauseEntry {
            spec: ClauseSpec {
                clause_id: "C-TEST".to_string(),
                title: "Test Clause".to_string(),
                kind: ClauseKind::Normative,
                status: ClauseStatus::Active,
                text: "Original normative behavior.".to_string(),
                anchors: vec![],
                superseded_by: None,
                since: Some("0.1.0".to_string()),
                tags: vec![],
            },
            path: PathBuf::from("gov/rfc/RFC-0001/clauses/C-TEST.toml"),
        }],
        path: PathBuf::from("gov/rfc/RFC-0001/rfc.toml"),
    }
}
