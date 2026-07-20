use super::*;
use crate::model::{
    ClauseEntry, ClauseKind, ClauseSpec, ClauseStatus, RfcIndex, RfcPhase, RfcSpec, RfcStatus,
    SectionSpec,
};
use std::path::PathBuf;

fn clause(status: ClauseStatus, superseded_by: Option<&str>) -> ClauseEntry {
    ClauseEntry {
        spec: ClauseSpec {
            clause_id: "C-OLD".to_string(),
            title: "Historical requirement".to_string(),
            kind: ClauseKind::Normative,
            status,
            text: "The implementation MUST preserve this behavior.".to_string(),
            anchors: vec![],
            superseded_by: superseded_by.map(str::to_string),
            since: Some("0.1.0".to_string()),
            tags: vec![],
        },
        path: PathBuf::new(),
    }
}

fn assert_before(output: &str, first: &str, second: &str) {
    let first_index = output.find(first);
    let second_index = output.find(second);
    assert!(first_index.is_some(), "missing marker {first}: {output}");
    assert!(second_index.is_some(), "missing marker {second}: {output}");
    assert!(first_index < second_index, "output: {output}");
}

#[test]
fn test_render_deprecated_clause_keeps_kind_and_shows_status_before_text() {
    let mut output = String::new();
    render_clause(
        &mut output,
        "RFC-0001",
        &clause(ClauseStatus::Deprecated, None),
    );

    assert!(output.contains("(Normative)"));
    assert_before(
        &output,
        "> **Status:** deprecated",
        "The implementation MUST preserve this behavior.",
    );

    let terminal = crate::terminal_md::strip_for_terminal(&output);
    assert!(terminal.contains("> **Status:** deprecated"));
    assert!(terminal.contains("(Normative)"));
    assert!(!terminal.contains("<del>"));
}

#[test]
fn test_render_superseded_clause_shows_status_and_replacement_before_text() {
    let mut output = String::new();
    render_clause(
        &mut output,
        "RFC-0001",
        &clause(ClauseStatus::Superseded, Some("RFC-0001:C-NEW")),
    );

    assert!(output.contains("(Normative)"));
    assert_before(
        &output,
        "> **Status:** superseded",
        "The implementation MUST preserve this behavior.",
    );
    assert_before(
        &output,
        "> **Superseded by:** RFC-0001:C-NEW",
        "The implementation MUST preserve this behavior.",
    );
}

#[test]
fn test_render_active_clause_omits_lifecycle_status() {
    let mut output = String::new();
    render_clause(&mut output, "RFC-0001", &clause(ClauseStatus::Active, None));

    assert!(!output.contains("**Status:**"));
    assert!(output.contains("(Normative)"));
}

#[test]
fn test_current_clause_projection_keeps_metadata_and_omits_obsolete_body() {
    let mut output = String::new();
    render_clause_with_projection(
        &mut output,
        "RFC-0001",
        &clause(ClauseStatus::Superseded, Some("RFC-0001:C-NEW")),
        RenderProjection::Current,
    );

    assert!(output.contains("RFC-0001:C-OLD"));
    assert!(output.contains("(Normative)"));
    assert!(output.contains("> **Status:** superseded"));
    assert!(output.contains("> **Superseded by:** RFC-0001:C-NEW"));
    assert!(output.contains("*Since: v0.1.0*"));
    assert!(!output.contains("The implementation MUST preserve this behavior."));
}

fn rfc(status: RfcStatus, clause_status: ClauseStatus) -> RfcIndex {
    let mut clause = clause(clause_status, Some("RFC-0001:C-NEW"));
    clause.path = PathBuf::from("clauses/C-OLD.toml");
    RfcIndex {
        rfc: RfcSpec {
            rfc_id: "RFC-0001".to_string(),
            title: "Projection test".to_string(),
            version: "0.2.0".to_string(),
            status,
            phase: RfcPhase::Stable,
            owners: vec!["@owner".to_string()],
            created: "2026-07-21".to_string(),
            updated: None,
            supersedes: None,
            refs: vec!["RFC-0002".to_string()],
            tags: vec!["cli".to_string()],
            sections: vec![SectionSpec {
                title: "Specification".to_string(),
                clauses: vec!["clauses/C-OLD.toml".to_string()],
            }],
            changelog: vec![],
            signature: None,
        },
        clauses: vec![clause],
        path: PathBuf::new(),
    }
}

#[test]
fn test_current_rfc_projection_suppresses_only_obsolete_nested_clause_bodies()
-> Result<(), Box<dyn std::error::Error>> {
    let current = render_rfc_with_projection(
        &rfc(RfcStatus::Normative, ClauseStatus::Superseded),
        RenderProjection::Current,
        None,
    )?;

    assert!(current.contains("## 1. Specification"));
    assert!(current.contains("> **Status:** superseded"));
    assert!(!current.contains("The implementation MUST preserve this behavior."));
    Ok(())
}

#[test]
fn test_deprecated_rfc_current_projection_is_metadata_only_but_archive_is_complete()
-> Result<(), Box<dyn std::error::Error>> {
    let rfc = rfc(RfcStatus::Deprecated, ClauseStatus::Active);
    let current = render_rfc_with_projection(&rfc, RenderProjection::Current, Some("RFC-0002"))?;
    let archive = render_rfc_with_projection(&rfc, RenderProjection::Archive, None)?;

    assert!(current.contains("# RFC-0001: Projection test"));
    assert!(current.contains("**Status:** deprecated"));
    assert!(current.contains("> **Owners:** @owner"));
    assert!(current.contains("> **Tags:** `cli`"));
    assert!(current.contains("> **Superseded by:** RFC-0002"));
    assert!(!current.contains("## 1. Specification"));
    assert!(!current.contains("The implementation MUST preserve this behavior."));
    assert!(archive.contains("## 1. Specification"));
    assert!(archive.contains("The implementation MUST preserve this behavior."));
    assert!(archive.contains("> **Owners:** @owner"));
    assert!(archive.contains("> **Tags:** `cli`"));
    assert!(archive.contains("> **Owners:** @owner\n> **Tags:** `cli`"));
    Ok(())
}

#[test]
fn test_rfc_archive_renders_its_direct_supersedes_relation()
-> Result<(), Box<dyn std::error::Error>> {
    let mut rfc = rfc(RfcStatus::Normative, ClauseStatus::Active);
    rfc.rfc.supersedes = Some("RFC-0000".to_string());

    let archive = render_rfc_with_projection(&rfc, RenderProjection::Archive, None)?;

    assert!(archive.contains("> **Supersedes:** RFC-0000"));
    Ok(())
}
