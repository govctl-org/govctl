//! Display path tests - verify relative paths in output.
//!
//! These tests ensure that paths shown to users are relative to the project root,
//! not absolute paths that would vary across machines.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

/// Test: RFC render shows relative output path
#[test]
fn test_render_rfc_display_path() {
    let temp_dir = init_project();
    let date = today();

    // Create draft RFC with clause
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Test RFC",
  "version": "0.1.0",
  "status": "draft",
  "phase": "spec",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-TEST.json"]
    }
  ],
  "changelog": [
    {
      "version": "0.1.0",
      "date": "2026-01-01",
      "notes": "Initial draft"
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
  "text": "Test clause content."
}"#,
    )
    .unwrap();

    let output = run_commands(
        temp_dir.path(),
        &[&["rfc", "render", "RFC-0001", "--dry-run"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: ADR render shows relative output path
#[test]
fn test_render_adr_display_path() {
    let temp_dir = init_project();
    let date = today();

    // Create ADR
    let adr_dir = temp_dir.path().join("gov/adr");
    fs::create_dir_all(&adr_dir).unwrap();

    fs::write(
        adr_dir.join("ADR-0001-test-decision.toml"),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Test Decision"
status = "proposed"
date = "2026-01-01"
refs = []

[content]
context = "Test context"
decision = "Test decision"
consequences = "Test consequences"
alternatives = []
"#,
    )
    .unwrap();

    let output = run_commands(
        temp_dir.path(),
        &[&["adr", "render", "ADR-0001", "--dry-run"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Work item render shows relative output path
#[test]
fn test_render_work_display_path() {
    let temp_dir = init_project();
    let date = today();

    // Create work item
    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir).unwrap();

    let work_filename = format!("{}-test-work.toml", date);
    fs::write(
        work_dir.join(&work_filename),
        format!(
            r#"[govctl]
schema = 1
id = "WI-{}-001"
title = "Test Work"
status = "active"
created = "{}"
started = "{}"
refs = []

[content]
description = "Test description"
acceptance_criteria = []
notes = []
"#,
            date, date, date
        ),
    )
    .unwrap();

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "render", &format!("WI-{}-001", date), "--dry-run"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete work item dry-run shows relative path
#[test]
fn test_delete_work_dry_run_display_path() {
    let temp_dir = init_project();
    let date = today();

    // Create a queued work item
    let work_dir = temp_dir.path().join("gov/work");
    fs::create_dir_all(&work_dir).unwrap();

    let work_filename = format!("{}-test-work.toml", date);
    fs::write(
        work_dir.join(&work_filename),
        format!(
            r#"[govctl]
schema = 1
id = "WI-{}-001"
title = "Test Work to Delete"
status = "queue"
created = "{}"
refs = []

[content]
description = "Test description"
acceptance_criteria = []
notes = []
"#,
            date, date
        ),
    )
    .unwrap();

    let output = run_commands(
        temp_dir.path(),
        &[&["work", "delete", &format!("WI-{}-001", date), "--dry-run"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete clause dry-run shows relative path
#[test]
fn test_delete_clause_dry_run_display_path() {
    let temp_dir = init_project();
    let date = today();

    // Create draft RFC with clause (draft status allows deletion)
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Draft RFC",
  "version": "0.1.0",
  "status": "draft",
  "phase": "spec",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-TO-DELETE.json"]
    }
  ],
  "changelog": [
    {
      "version": "0.1.0",
      "date": "2026-01-01",
      "notes": "Initial draft"
    }
  ]
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-TO-DELETE.json"),
        r#"{
  "clause_id": "C-TO-DELETE",
  "title": "Clause To Delete",
  "kind": "normative",
  "status": "active",
  "text": "This clause will be deleted."
}"#,
    )
    .unwrap();

    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "delete", "RFC-0001:C-TO-DELETE", "--dry-run"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
