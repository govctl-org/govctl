//! Tests for versioned storage migration pipeline.

mod common;

use common::{init_project, run_commands};
use std::fs;

fn write_legacy_rfc_project(dir: &std::path::Path) {
    let rfc_dir = dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

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
    )
    .unwrap();

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
    )
    .unwrap();
}

#[test]
fn test_migrate_converts_json_rfc_and_upgrades_releases() {
    let temp_dir = init_project();
    write_legacy_rfc_project(temp_dir.path());
    fs::write(
        temp_dir.path().join("gov/releases.toml"),
        r#"[[releases]]
version = "1.0.0"
date = "2026-01-01"
"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["migrate"], &["check"]]);
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

    let releases = fs::read_to_string(temp_dir.path().join("gov/releases.toml")).unwrap();
    assert!(releases.contains("#:schema"));
    assert!(
        !releases.contains("schema = 1"),
        "govctl.schema should be stripped: {}",
        releases
    );

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml")).unwrap();
    assert!(
        config.contains("version = 2"),
        "schema version should be bumped to 2: {}",
        config
    );
}

#[test]
fn test_migrate_dry_run_preserves_legacy_files() {
    let temp_dir = init_project();
    write_legacy_rfc_project(temp_dir.path());

    let output = run_commands(temp_dir.path(), &[&["--dry-run", "migrate"]]);
    assert!(output.contains("Would write: gov/rfc/RFC-0001/rfc.toml"));
    assert!(output.contains("Would delete: gov/rfc/RFC-0001/rfc.json"));

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.json").exists());
    assert!(rfc_dir.join("clauses/C-LEGACY.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());
    assert!(!rfc_dir.join("clauses/C-LEGACY.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml")).unwrap();
    assert!(
        config.contains("version = 1"),
        "dry-run should not bump version: {}",
        config
    );
}

#[test]
fn test_migrate_is_noop_on_current_version() {
    let temp_dir = init_project();

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
    );

    // Second migrate should be a noop
    let output = run_commands(temp_dir.path(), &[&["migrate"]]);
    assert!(
        output.contains("already at schema version 2"),
        "output: {}",
        output
    );
}

#[test]
fn test_migrate_bumps_version_even_without_file_changes() {
    let temp_dir = init_project();

    // Create artifacts using govctl (already in new format with headers)
    run_commands(temp_dir.path(), &[&["rfc", "new", "New Format RFC"]]);

    // Config still says version = 1, but files are already in v2 format
    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml")).unwrap();
    assert!(config.contains("version = 1"));

    let output = run_commands(temp_dir.path(), &[&["migrate"]]);
    assert!(
        output.contains("Schema version bumped to 2"),
        "should bump version even with no file ops: {}",
        output
    );

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml")).unwrap();
    assert!(
        config.contains("version = 2"),
        "config should now be version 2: {}",
        config
    );
}

#[test]
fn test_migrate_failure_leaves_legacy_repo_unchanged() {
    let temp_dir = init_project();
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

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
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["migrate"]]);
    assert!(output.contains("error[E0202]"), "output: {}", output);

    assert!(rfc_dir.join("rfc.json").exists());
    assert!(!rfc_dir.join("rfc.toml").exists());

    let config = fs::read_to_string(temp_dir.path().join("gov/config.toml")).unwrap();
    assert!(
        config.contains("version = 1"),
        "version should not be bumped on failure: {}",
        config
    );
}
