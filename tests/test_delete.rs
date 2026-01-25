//! Delete command tests - clause and work item deletion safeguards.

mod common;

use common::{init_project, normalize_output, run_commands, run_dynamic_commands, today};
use std::fs;

/// Test: Delete clause - safeguard prevents deleting from normative RFC
#[test]
fn test_delete_clause_safeguard_normative() {
    let temp_dir = init_project();
    let date = today();

    // Create normative RFC with clause
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Normative RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-LOCKED.json"]
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
        rfc_dir.join("clauses/C-LOCKED.json"),
        r#"{
  "clause_id": "C-LOCKED",
  "title": "Locked Clause",
  "kind": "normative",
  "status": "active",
  "text": "This clause cannot be deleted - RFC is normative.",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    // Try to delete the clause (should fail)
    let output = run_commands(
        temp_dir.path(),
        &[&["clause", "delete", "RFC-0001:C-LOCKED", "-f"]],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete clause - successful deletion from draft RFC
#[test]
fn test_delete_clause_success_draft() {
    let temp_dir = init_project();
    let date = today();

    // Create draft RFC with two clauses
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
      "clauses": ["clauses/C-KEEP.json", "clauses/C-DELETE.json"]
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
        rfc_dir.join("clauses/C-KEEP.json"),
        r#"{
  "clause_id": "C-KEEP",
  "title": "Keep This Clause",
  "kind": "normative",
  "status": "active",
  "text": "This clause will remain.",
  "since": "0.1.0"
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-DELETE.json"),
        r#"{
  "clause_id": "C-DELETE",
  "title": "Delete This Clause",
  "kind": "normative",
  "status": "active",
  "text": "This clause will be deleted.",
  "since": "0.1.0"
}"#,
    )
    .unwrap();

    // Delete the clause with force flag, then verify
    let output = run_commands(
        temp_dir.path(),
        &[
            &["clause", "delete", "RFC-0001:C-DELETE", "-f"],
            &["clause", "list", "RFC-0001"],
            &["check"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete work item - safeguard prevents deleting active work item
#[test]
fn test_delete_work_safeguard_active() {
    let temp_dir = init_project();
    let date = today();
    let wi1 = format!("WI-{}-001", date);

    // Create active work item
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Active work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "chore: Test criterion".to_string(),
        ],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete work item - safeguard prevents deleting done work item
#[test]
fn test_delete_work_safeguard_done() {
    let temp_dir = init_project();
    let date = today();
    let wi1 = format!("WI-{}-001", date);

    // Create and complete work item
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Completed work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "chore: Test criterion".to_string(),
        ],
        vec![
            "work".to_string(),
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "Test".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "move".to_string(),
            wi1.clone(),
            "done".to_string(),
        ],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete work item - successful deletion of queued work item
#[test]
fn test_delete_work_success_queue() {
    let temp_dir = init_project();
    let date = today();
    let wi1 = format!("WI-{}-001", date);

    // Create two queued work items
    let commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Keep this work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Delete this work item".to_string(),
        ],
        vec!["work".to_string(), "list".to_string()],
        vec![
            "work".to_string(),
            "delete".to_string(),
            wi1.clone(),
            "-f".to_string(),
        ],
        vec!["work".to_string(), "list".to_string()],
        vec!["check".to_string()],
    ];

    let output = run_dynamic_commands(temp_dir.path(), &commands);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: Delete work item - safeguard prevents deletion when referenced
#[test]
fn test_delete_work_safeguard_referenced() {
    let temp_dir = init_project();
    let date = today();
    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);

    // Create two work items where wi2 references wi1
    let setup_commands: Vec<Vec<String>> = vec![
        vec![
            "work".to_string(),
            "new".to_string(),
            "Referenced work item".to_string(),
        ],
        vec![
            "work".to_string(),
            "new".to_string(),
            "Work item with reference".to_string(),
        ],
        vec![
            "work".to_string(),
            "add".to_string(),
            wi2.clone(),
            "refs".to_string(),
            wi1.clone(),
        ],
    ];

    let _ = run_dynamic_commands(temp_dir.path(), &setup_commands);

    // Try to delete wi1 (should fail because wi2 references it)
    let delete_commands: Vec<Vec<String>> = vec![vec![
        "work".to_string(),
        "delete".to_string(),
        wi1.clone(),
        "-f".to_string(),
    ]];

    let output = run_dynamic_commands(temp_dir.path(), &delete_commands);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
