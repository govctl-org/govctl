//! Source code scanning tests - [[artifact-id]] reference detection.

mod common;

use common::{init_project, normalize_output, run_commands, today};
use std::fs;

/// Test: Source code scanning detects valid and invalid [[RFC-XXX:C-XXX]] references
#[test]
fn test_source_scan_detects_refs() {
    let temp_dir = init_project();
    let date = today();

    // Create RFC with a valid clause
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Test RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["@test"],
  "created": "2026-01-17",
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-VALID.json"]
    }
  ],
  "changelog": [
    {
      "version": "1.0.0",
      "date": "2026-01-17",
      "notes": "Initial release"
    }
  ]
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-VALID.json"),
        r#"{
  "clause_id": "C-VALID",
  "title": "Valid Clause",
  "kind": "normative",
  "status": "active",
  "text": "This is a valid clause.",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    // Create source file with references
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        src_dir.join("example.rs"),
        r#"// This file contains references for source scan testing.

// Valid reference to existing clause
// See [[RFC-0001:C-VALID]] for specification.

// Valid reference to existing RFC
// Implements [[RFC-0001]].

// Invalid reference to non-existent clause
// Based on [[RFC-0001:C-MISSING]] design.

// Invalid reference to non-existent RFC
// See [[RFC-9999]] for future work.

fn main() {
    println!("Test file for source scanning");
}
"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}
