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

/// Test: custom gov_root in config is respected
/// gov/config.toml location is fixed, but gov_root inside points to data directory
#[test]
fn test_init_custom_gov_root() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Run init first (creates gov/config.toml and gov/ structure)
    run_commands(temp_dir.path(), &[&["init"]]);

    // Update gov/config.toml to use custom gov_root
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
gov_root = "governance"
docs_output = "docs"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create the custom gov_root directory
    fs::create_dir_all(temp_dir.path().join("governance/rfc")).unwrap();
    fs::create_dir_all(temp_dir.path().join("governance/adr")).unwrap();
    fs::create_dir_all(temp_dir.path().join("governance/work")).unwrap();
    fs::create_dir_all(temp_dir.path().join("governance/schema")).unwrap();

    // Create a work item - should go to governance/work
    let output = run_commands(
        temp_dir.path(),
        &[&["work", "new", "Test item", "--active"]],
    );
    assert!(output.contains("Created work item"), "output: {}", output);

    // Work item should be under governance/work/
    let work_dir = temp_dir.path().join("governance/work");
    assert!(
        work_dir.exists() && work_dir.read_dir().unwrap().count() > 0,
        "work dir should have items under governance/"
    );
    // Note: Lock file is created under governance/ at runtime
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
gov_root = "gov"
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

/// Test: custom gov_root and docs_output together
#[test]
fn test_init_custom_paths_combined() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");

    // Run init first (creates gov/)
    run_commands(temp_dir.path(), &[&["init"]]);

    // Update gov/config.toml with both custom paths
    let config_path = temp_dir.path().join("gov/config.toml");
    let config_content = r#"[project]
name = "test-project"

[paths]
gov_root = "data/gov"
docs_output = "output/docs"
"#;
    fs::write(&config_path, config_content).unwrap();

    // Create custom gov_root structure
    fs::create_dir_all(temp_dir.path().join("data/gov/rfc")).unwrap();
    fs::create_dir_all(temp_dir.path().join("data/gov/adr")).unwrap();
    fs::create_dir_all(temp_dir.path().join("data/gov/work")).unwrap();
    fs::create_dir_all(temp_dir.path().join("data/gov/schema")).unwrap();

    // Create an ADR
    let output = run_commands(temp_dir.path(), &[&["adr", "new", "Test ADR"]]);
    assert!(output.contains("Created ADR"), "output: {}", output);

    // ADR should be under data/gov/adr/
    let adr_dir = temp_dir.path().join("data/gov/adr");
    assert!(
        adr_dir.exists() && adr_dir.read_dir().unwrap().count() > 0,
        "ADR should be under data/gov/adr/"
    );

    // Render the ADR by ID (ADR-0001 by default)
    let _output = run_commands(temp_dir.path(), &[&["adr", "render", "ADR-0001"]]);
    let docs_dir = temp_dir.path().join("output/docs/adr");
    assert!(docs_dir.exists(), "docs should be under output/docs/adr/");
}
