//! Tests for source code reference scanning.
//!
//! Tests the scan_source_refs function which scans source files for
//! references to governance artifacts.

mod common;

use common::{init_project_with_date, run_commands, run_normalized_commands};
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

fn create_normative_rfc(dir: &Path, id: &str, title: &str) -> common::TestResult {
    run_commands(
        dir,
        &[
            &["rfc", "new", title],
            &["rfc", "finalize", id, "normative"],
        ],
    )?;
    Ok(())
}

macro_rules! assert_scan_check_snapshot {
    ($temp_dir:expr, $date:expr) => {{
        let value = run_normalized_commands($temp_dir.path(), $date, &[&["check"]])?;
        crate::assert_current_test_snapshot!("test_scan", value);
        Ok(())
    }};
}

#[test]
fn test_scan_no_references() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_valid_rfc_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Test RFC")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_uses_project_root_when_run_from_subdirectory() -> common::TestResult {
    let (temp_dir, _) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Test RFC")?;
    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    let docs_dir = temp_dir.path().join("docs");
    fs::create_dir_all(&docs_dir)?;
    let output = run_commands(&docs_dir, &[&["check"]])?;

    assert!(
        output.contains("  1 source files scanned"),
        "output: {}",
        output
    );
    assert!(
        output.contains("  1 references found"),
        "output: {}",
        output
    );
    Ok(())
}

#[test]
fn test_scan_valid_clause_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

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

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001:C-TEST]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_unknown_rfc_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-9999]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_unknown_clause_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Test RFC")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001:C-NONEXISTENT]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_deprecated_rfc_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Old RFC")?;
    run_commands(
        temp_dir.path(),
        &[&["rfc", "deprecate", "RFC-0001", "--force"]],
    )?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_valid_adr_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    run_commands(
        temp_dir.path(),
        &[
            &["adr", "new", "Test Decision"],
            &["adr", "accept", "ADR-0001"],
        ],
    )?;

    write_main_rs(temp_dir.path(), "// Follows [[ADR-0001]]\nfn main() {}\n")?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_valid_work_item_reference() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    run_commands(temp_dir.path(), &[&["work", "new", "Test task"]])?;

    let wi_output = run_commands(temp_dir.path(), &[&["work", "list", "all"]])?;
    let wi_id = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-\d{3}")?
        .find(&wi_output)
        .ok_or("No work item ID found")?
        .as_str()
        .to_string();

    write_main_rs(
        temp_dir.path(),
        format!("// Implements [[{}]]\nfn main() {{}}\n", wi_id),
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_multiple_references_in_file() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "RFC One")?;
    create_normative_rfc(temp_dir.path(), "RFC-0002", "RFC Two")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]] and [[RFC-0002]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}

#[test]
fn test_scan_mixed_valid_invalid_references() -> common::TestResult {
    let (temp_dir, date) = init_source_scan_project()?;

    create_normative_rfc(temp_dir.path(), "RFC-0001", "Valid RFC")?;

    write_main_rs(
        temp_dir.path(),
        "// Implements [[RFC-0001]] and [[RFC-9999]]\nfn main() {}\n",
    )?;

    assert_scan_check_snapshot!(temp_dir, &date)
}
