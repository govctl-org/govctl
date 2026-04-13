//! Tests for versioned storage migration pipeline.

mod common;

use common::{TestResult, init_project, init_project_v1, run_commands};
use std::fs;

fn write_legacy_rfc_project(dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let rfc_dir = dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Legacy RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["@test-user"],
  "created": "2026-01-01",
  "refs": [],
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-LEGACY.json"]
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
    )?;

    fs::write(
        rfc_dir.join("clauses/C-LEGACY.json"),
        r#"{
  "clause_id": "C-LEGACY",
  "title": "Legacy Clause",
  "kind": "normative",
  "status": "active",
  "text": "Legacy clause text.",
  "since": "1.0.0"
}"#,
    )?;

    Ok(())
}

#[test]
fn test_migrate_converts_json_rfc_and_upgrades_releases() -> TestResult {
    let temp_dir = init_project_v1()?;
    write_legacy_rfc_project(temp_dir.path())?;
    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[[releases]]
version = "1.0.0"
date = "2026-01-01"
"#,
    )?;

    let output = run_commands(temp_dir.path(), &[&["migrate"], &["check"]])?;
    assert!(
        output.contains("file(s) written"),
        "should report written files: {}",
        output
    );
    assert!(output.contains("exit: 0"), "output: {}", output);

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.toml").exists());
    assert!(rfc_dir.join("clauses/C-LEGACY.toml").exists());
    assert!(!rfc_dir.join("rfc.json").exists());
    assert!(!rfc_dir.join("clauses/C-LEGACY.json").exists());

    let releases = fs::read_to_string(temp_dir.path().join("gov/releases.toml"))?;
    assert!(releases.contains("#:schema"));
    assert!(
        !releases.contains("schema = 1"),
        "govctl.schema should be stripped: {}",
        releases
    );

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 2"),
        "schema version should be bumped to 2: {}",
        config
    );

    Ok(())
}

#[test]
fn test_migrate_dry_run_preserves_legacy_files() -> TestResult {
    let temp_dir = init_project_v1()?;
    write_legacy_rfc_project(temp_dir.path())?;

    let output = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]])?;
    assert!(output.contains("Would write: gov/rfc/RFC-0001/rfc.toml"));
    assert!(output.contains("Would delete: gov/rfc/RFC-0001/rfc.json"));

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.json").exists());
    assert!(rfc_dir.join("clauses/C-LEGACY.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());
    assert!(!rfc_dir.join("clauses/C-LEGACY.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "dry-run should not bump version: {}",
        config
    );

    Ok(())
}

#[test]
fn test_migrate_is_noop_on_current_version() -> TestResult {
    let temp_dir = init_project()?;

    // Create an RFC so the project isn't empty, then migrate to bump version
    run_commands(
        temp_dir.path(),
        &[
            &["rfc", "new", "Migrated RFC"],
            &[
                "clause",
                "new",
                "RFC-0001:C-SUMMARY",
                "Summary",
                "-s",
                "Summary",
            ],
            &["migrate"],
        ],
    )?;

    // Second migrate should be a noop
    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        output.contains("already at schema version 2"),
        "output: {}",
        output
    );

    Ok(())
}

#[test]
fn test_migrate_bumps_version_even_without_file_changes() -> TestResult {
    let temp_dir = init_project_v1()?;

    // Create artifacts using govctl (already in new format with headers)
    run_commands(temp_dir.path(), &[&["rfc", "new", "New Format RFC"]])?;

    // Config says version = 1, but files are already in v2 format
    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(config.contains("version = 1"));

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(
        output.contains("Schema version bumped to 2"),
        "should bump version even with no file ops: {}",
        output
    );

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 2"),
        "config should now be version 2: {}",
        config
    );

    Ok(())
}

#[test]
fn test_migrate_failure_leaves_legacy_repo_unchanged() -> TestResult {
    let temp_dir = init_project_v1()?;
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses"))?;

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Broken Legacy RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["@test-user"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Specification",
      "clauses": ["clauses/C-MISSING.json"]
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
    )?;

    let output = run_commands(temp_dir.path(), &[&["migrate"]])?;
    assert!(output.contains("error[E0202]"), "output: {}", output);

    assert!(rfc_dir.join("rfc.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml"))?;
    assert!(
        config.contains("version = 1"),
        "version should not be bumped on failure: {}",
        config
    );

    Ok(())
}
