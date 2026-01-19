//! Happy path integration tests - basic functionality validation.

mod common;

use common::{init_project, normalize_output, run_commands, run_dynamic_commands, today};
use std::fs;
use std::path::Path;

/// Create a minimal valid project with RFC, clause, ADR, and work item
fn setup_minimal_valid(dir: &Path, date: &str) {
    let wi1 = format!("WI-{}-001", date);

    // Create RFC with clause
    let rfc_dir = dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Test RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Overview",
      "clauses": ["clauses/C-EXAMPLE.json"]
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

    fs::write(
        rfc_dir.join("clauses/C-EXAMPLE.json"),
        r#"{
  "clause_id": "C-EXAMPLE",
  "title": "Example Clause",
  "kind": "normative",
  "status": "active",
  "text": "This is an example clause for testing.",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    // Create ADR
    fs::write(
        dir.join("gov/adr/ADR-0001-test-decision.toml"),
        r#"[govctl]
schema = 1
id = "ADR-0001"
title = "Test Decision"
status = "accepted"
date = "2026-01-01"
refs = ["RFC-0001"]

[content]
context = "We need to test ADR functionality."
decision = "We will create a test ADR."
consequences = "Tests will pass."
"#,
    )
    .unwrap();

    // Create work item via commands
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Test work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "add: Test criterion".to_string(),
        ],
    ];

    let _ = run_dynamic_commands(dir, &commands);
}

#[test]
fn test_minimal_valid_check() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_rfc() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["rfc", "list"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_clause() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["clause", "list"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_adr() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["adr", "list"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_work() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["work", "list"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_status() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["status"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_full_workflow() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["check"],
            &["rfc", "list"],
            &["clause", "list"],
            &["adr", "list"],
            &["work", "list"],
            &["status"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
