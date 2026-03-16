//! Tests for `govctl init` command behavior.

mod common;

use common::run_commands;
use std::fs;
use tempfile::TempDir;

/// Test: init creates .gitignore with .govctl.lock if not exists
#[test]
fn test_init_creates_gitignore() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Run init
    let output = run_commands(temp_dir.path(), &[&["init"]]);
    assert!(output.contains("Project initialized"));

    // .gitignore should exist and contain .govctl.lock
    let gitignore_path = temp_dir.path().join(".gitignore");
    assert!(gitignore_path.exists(), ".gitignore should be created");

    let content = fs::read_to_string(&gitignore_path).unwrap();
    assert!(
        content.contains(".govctl.lock"),
        ".gitignore should contain .govctl.lock"
    );
}

/// Test: init installs bundled artifact JSON Schemas
#[test]
fn test_init_creates_artifact_schema_files() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    let output = run_commands(temp_dir.path(), &[&["init"]]);
    assert!(output.contains("Project initialized"));

    for filename in [
        "rfc.schema.json",
        "clause.schema.json",
        "adr.schema.json",
        "work.schema.json",
        "release.schema.json",
        "guard.schema.json",
    ] {
        assert!(
            temp_dir.path().join("gov/schema").join(filename).exists(),
            "schema file should exist: {}",
            filename
        );
    }

    assert!(
        temp_dir.path().join("gov/guard").exists(),
        "guard directory should exist after init"
    );
}

/// Test: init appends .govctl.lock to existing .gitignore
#[test]
fn test_init_appends_to_existing_gitignore() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Create existing .gitignore
    let gitignore_path = temp_dir.path().join(".gitignore");
    fs::write(&gitignore_path, "# Existing content\ntarget/\n").unwrap();

    // Run init
    let output = run_commands(temp_dir.path(), &[&["init"]]);
    assert!(output.contains("Project initialized"));

    // .gitignore should still exist with both old and new content
    let content = fs::read_to_string(&gitignore_path).unwrap();
    assert!(
        content.contains("target/"),
        ".gitignore should retain existing content"
    );
    assert!(
        content.contains(".govctl.lock"),
        ".gitignore should have .govctl.lock appended"
    );
}

/// Test: init doesn't duplicate .govctl.lock if already present
#[test]
fn test_init_no_duplicate_gitignore_entry() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Create .gitignore with .govctl.lock already present
    let gitignore_path = temp_dir.path().join(".gitignore");
    fs::write(&gitignore_path, ".govctl.lock\n").unwrap();

    // Run init
    let output = run_commands(temp_dir.path(), &[&["init"]]);
    assert!(output.contains("Project initialized"));

    // Should not have duplicate entry
    let content = fs::read_to_string(&gitignore_path).unwrap();
    let count = content.matches(".govctl.lock").count();
    assert_eq!(
        count, 1,
        ".gitignore should not have duplicate .govctl.lock entries"
    );
}

/// Test: custom docs_output is respected for render
#[test]
fn test_init_custom_docs_output() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Run init first
    run_commands(temp_dir.path(), &[&["init"]]);

    // Update gov/config.toml with custom docs_output
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
docs_output = "documentation"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create an RFC
    let output = run_commands(temp_dir.path(), &[&["rfc", "new", "Test RFC"]]);
    assert!(output.contains("Created RFC"));

    // Render the first RFC (RFC-0001 by default on fresh init)
    let output = run_commands(temp_dir.path(), &[&["rfc", "render", "RFC-0001"]]);
    eprintln!("render output: {}", output);

    // Rendered output should be under documentation/rfc/
    let docs_dir = temp_dir.path().join("documentation/rfc");
    assert!(docs_dir.exists(), "docs should be under documentation/rfc/");
}

/// Test: custom docs_output with ADR render
#[test]
fn test_init_custom_paths_combined() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    run_commands(temp_dir.path(), &[&["init"]]);

    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
docs_output = "output/docs"
"#;
    fs::write(&config_path, config_content).unwrap();

    let output = run_commands(temp_dir.path(), &[&["adr", "new", "Test ADR"]]);
    assert!(output.contains("Created ADR"), "output: {}", output);

    let adr_dir = temp_dir.path().join("gov/adr");
    assert!(
        adr_dir.exists() && adr_dir.read_dir().unwrap().count() > 0,
        "ADR should be under gov/adr/"
    );

    let _output = run_commands(temp_dir.path(), &[&["adr", "render", "ADR-0001"]]);
    let docs_dir = temp_dir.path().join("output/docs/adr");
    assert!(docs_dir.exists(), "docs should be under output/docs/adr/");
}
