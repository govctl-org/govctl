//! Error case tests - validation of invalid states.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

/// Test: RFC files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_rfc_schema_check() {
    let temp_dir = init_project();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Invalid RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [],
  "unexpected": true
}"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    assert!(output.contains("error[E0101]"), "output: {}", output);
    assert!(output.contains("rfc.schema.json"), "output: {}", output);
}

/// Test: Clause files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_clause_schema_check() {
    let temp_dir = init_project();

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Clause Schema Test",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [{"title": "Test", "clauses": ["clauses/C-TEST.json"]}]
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Invalid Clause",
  "kind": "normative",
  "text": "Clause text",
  "unexpected": "should fail schema validation"
}"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    assert!(output.contains("error[E0201]"), "output: {}", output);
    assert!(output.contains("clause.schema.json"), "output: {}", output);
}

/// Test: Clause claims superseded_by a non-existent clause
#[test]
fn test_broken_superseded_check() {
    let temp_dir = init_project();
    let date = today();

    // Create RFC with broken supersession
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Broken Superseded Test",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Clauses",
      "clauses": ["clauses/C-OLD.json", "clauses/C-NEW.json"]
    }
  ],
  "changelog": [
    {
      "version": "1.0.0",
      "date": "2026-01-01",
      "added": ["Initial release"]
    }
  ]
}"#,
    )
    .unwrap();

    // C-OLD claims to be superseded by C-NONEXISTENT (which doesn't exist)
    fs::write(
        rfc_dir.join("clauses/C-OLD.json"),
        r#"{
  "clause_id": "C-OLD",
  "title": "Old Clause",
  "kind": "normative",
  "status": "superseded",
  "text": "This clause is superseded.",
  "superseded_by": "C-NONEXISTENT",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-NEW.json"),
        r#"{
  "clause_id": "C-NEW",
  "title": "New Clause",
  "kind": "normative",
  "status": "active",
  "text": "This is the new clause.",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: RFC has invalid status/phase combination (draft + stable)
#[test]
fn test_invalid_transition_check() {
    let temp_dir = init_project();
    let date = today();

    // Create RFC with invalid state
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Invalid Transition Test",
  "version": "0.1.0",
  "status": "draft",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Overview",
      "clauses": ["clauses/C-TEST.json"]
    }
  ],
  "changelog": [
    {
      "version": "0.1.0",
      "date": "2026-01-01",
      "added": ["Initial draft"]
    }
  ]
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Test Clause",
  "kind": "normative",
  "status": "active",
  "text": "A test clause in an invalid RFC.",
  "since": "0.1.0"
}"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: ADR files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_adr_schema_check() {
    let temp_dir = init_project();

    fs::write(
        temp_dir.path().join("gov/adr/ADR-0001-invalid.toml"),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Invalid ADR"
status = "accepted"
date = "2026-01-01"

[content]
context = "Context"
decision = "Decision"
consequences = "Consequences"
unexpected = "should fail schema validation"
"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    assert!(output.contains("error[E0301]"), "output: {}", output);
    assert!(output.contains("adr.schema.json"), "output: {}", output);
}

/// Test: Work item files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_work_schema_check() {
    let temp_dir = init_project();

    fs::write(
        temp_dir.path().join("gov/work/2026-01-01-invalid.toml"),
        r#"[govctl]
schema = 1
id = "WI-2026-01-01-001"
title = "Invalid Work Item"
status = "queue"
created = "2026-01-01"

[content]
description = "Work description"
unexpected = "should fail schema validation"
"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    assert!(output.contains("error[E0401]"), "output: {}", output);
    assert!(output.contains("work.schema.json"), "output: {}", output);
}

/// Test: Release files fail check when they contain unknown fields rejected by schema
#[test]
fn test_invalid_release_schema_check() {
    let temp_dir = init_project();

    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[govctl]
schema = 1

[[releases]]
version = "1.0.0"
date = "2026-01-01"
unexpected = "should fail schema validation"
"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    assert!(output.contains("error[E0704]"), "output: {}", output);
    assert!(output.contains("release.schema.json"), "output: {}", output);
}
