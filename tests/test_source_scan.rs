//! Source code scanning tests - [[artifact-id]] reference detection.

mod common;

use common::{init_project_with_date, normalize_output, run_commands};
use std::fs;

#[test]
fn test_source_scan_detects_refs() -> common::TestResult {
    let (temp_dir, date) = init_project_with_date()?;

    // Create RFC with a valid clause
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.toml"),
        r#"#:schema ../../schema/rfc.schema.json

[govctl]
schema = 1
id = "RFC-0001"
title = "Test RFC"
version = "1.0.0"
status = "normative"
phase = "stable"
owners = ["@test"]
created = "2026-01-17"

[[sections]]
title = "Specification"
clauses = ["clauses/C-VALID.toml"]

[[changelog]]
version = "1.0.0"
date = "2026-01-17"
notes = "Initial release"
"#,
    )?;

    fs::write(
        rfc_dir.join("clauses/C-VALID.toml"),
        r#"#:schema ../../../schema/clause.schema.json

[govctl]
schema = 1
id = "C-VALID"
title = "Valid Clause"
kind = "normative"
status = "active"
since = "1.0.0"

[content]
text = "This is a valid clause."
"#,
    )?;

    // Create source file with references
    let src_dir = temp_dir.path().join("src");
    fs::create_dir_all(&src_dir)?;

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
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    let normalized = normalize_output(&output, temp_dir.path(), &date)?;
    crate::assert_current_test_snapshot!("test_source_scan", normalized);
    Ok(())
}
