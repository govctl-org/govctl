//! Tests for deterministic storage migration.

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
    assert!(output.contains("Migrated 1 RFC(s), 1 clause file(s)"));
    assert!(output.contains("exit: 0"), "output: {}", output);

    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    assert!(rfc_dir.join("rfc.toml").exists());
    assert!(rfc_dir.join("clauses/C-LEGACY.toml").exists());
    assert!(!rfc_dir.join("rfc.json").exists());
    assert!(!rfc_dir.join("clauses/C-LEGACY.json").exists());

    let releases = fs::read_to_string(temp_dir.path().join("gov/releases.toml")).unwrap();
    assert!(releases.contains("[govctl]"));
    assert!(releases.contains("schema = 1"));
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
}

#[test]
fn test_migrate_is_noop_on_migrated_repo() {
    let temp_dir = init_project();

    let output = run_commands(
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

    assert!(
        output.contains("Repository already migrated"),
        "output: {}",
        output
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
}
