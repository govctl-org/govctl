//! Tests for source code reference scanning.
//!
//! Tests the scan_source_refs function which scans source files for
//! references to governance artifacts.

mod common;

use common::{init_project_with_date, normalize_output, run_commands};
use std::fs;
use std::path::Path;

/// Helper to enable source scanning in a project.
/// Parses config as typed TOML, inserts an active `[source_scan]` table, and writes back.
fn enable_source_scan(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let config_path = dir.join("gov/config.toml");
    let content = fs::read_to_string(&config_path)?;
    let mut doc: toml::Table = toml::from_str(&content)?;
    let mut scan = toml::Table::new();
    scan.insert("enabled".into(), toml::Value::Boolean(true));
    scan.insert(
        "include".into(),
        toml::Value::Array(vec![toml::Value::String("src/**/*.rs".into())]),
    );
    scan.insert("exclude".into(), toml::Value::Array(vec![]));
    doc.insert("source_scan".into(), toml::Value::Table(scan));
    fs::write(&config_path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}

fn init_source_scan_project() -> Result<(tempfile::TempDir, String), Box<dyn std::error::Error>> {
    let (temp_dir, date) = init_project_with_date()?;
    enable_source_scan(temp_dir.path())?;
    Ok((temp_dir, date))
}

fn write_main_rs(dir: &Path, contents: impl AsRef<str>) -> Result<(), Box<dyn std::error::Error>> {
    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir)?;
    fs::write(src_dir.join("main.rs"), contents.as_ref())?;
    Ok(())
}

#[test]
fn test_scan_no_references() -> common::TestResult {
    // project with no source files should scan successfully
    let (temp_dir, date) = init_source_scan_project()?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_valid_rfc_reference() -> common::TestResult {
    // source file with valid RFC reference should pass
    let (temp_dir, date) = init_source_scan_project()?;

    // Create an RFC
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    // Create a source file with a reference to the RFC
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_valid_clause_reference() -> common::TestResult {
    // source file with valid clause reference should pass
    let (temp_dir, date) = init_source_scan_project()?;

    // Create an RFC with a clause
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-TEST",
                "Test Clause",
                "-s",
                "Specification",
                "-k",
                "normative",
            ],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    // Create a source file with a reference to the clause
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001:C-TEST]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_unknown_rfc_reference() -> common::TestResult {
    // source file with unknown RFC reference should error
    let (temp_dir, date) = init_source_scan_project()?;

    // Create a source file with a reference to non-existent RFC
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-9999]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_unknown_clause_reference() -> common::TestResult {
    // source file with unknown clause reference should error
    let (temp_dir, date) = init_source_scan_project()?;

    // Create an RFC but no clause
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Test RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    // Create a source file with a reference to non-existent clause
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001:C-NONEXISTENT]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_deprecated_rfc_reference() -> common::TestResult {
    // source file with deprecated RFC reference should warn
    let (temp_dir, date) = init_source_scan_project()?;

    // Create and deprecate an RFC
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Old RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "deprecate", "RFC-0001", "--force"],
        ],
    )?;

    // Create a source file with a reference to deprecated RFC
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_valid_adr_reference() -> common::TestResult {
    // source file with valid ADR reference should pass
    let (temp_dir, date) = init_source_scan_project()?;

    // Create an ADR
    run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;

    // Create a source file with a reference to the ADR
    write_main_rs(temp_dir.path(), "// Follows [[ADR-0001]]\nfn main() {}\n")?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_valid_work_item_reference() -> common::TestResult {
    // source file with valid work item reference should pass
    let (temp_dir, date) = init_source_scan_project()?;

    // Create a work item
    run_commands(temp_dir.path(), &[&["work", "new", "Test task"]])?;

    // Get the work item ID from the output
    let wi_output = run_commands(temp_dir.path(), &[&["work", "list", "all"]])?;
    let wi_id = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-\d{3}")?
        .find(&wi_output)
        .ok_or("No work item ID found")?
        .as_str()
        .to_string();

    // Create a source file with a reference to the work item
    write_main_rs(
        temp_dir.path(),
        format!("// Implements [[{}]]\nfn main() {{}}\n", wi_id),
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_multiple_references_in_file() -> common::TestResult {
    // source file with multiple references should check all
    let (temp_dir, date) = init_source_scan_project()?;

    // Create RFCs
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "RFC One"],
            &["rfc", "finalize", "RFC-0001", "normative"],
            &["rfc", "new", "RFC Two"],
            &["rfc", "finalize", "RFC-0002", "normative"],
        ],
    )?;

    // Create a source file with multiple references
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]] and [[RFC-0002]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}

#[test]
fn test_scan_mixed_valid_invalid_references() -> common::TestResult {
    // source file with mixed valid/invalid references should report errors
    let (temp_dir, date) = init_source_scan_project()?;

    // Create one RFC
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Valid RFC"],
            &["rfc", "finalize", "RFC-0001", "normative"],
        ],
    )?;

    // Create a source file with valid and invalid references
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]] and [[RFC-9999]]\nfn main() {}\n",
    )?;

    let output = run_commands(temp_dir.path(), &[&["check"]])?;
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date)?);
    Ok(())
}
