//! CLI snapshot tests using insta.
//!
//! These tests run govctl commands against dynamically created projects and snapshot
//! the combined stdout/stderr output for regression testing.
//!
//! All fixtures are created at test runtime using `govctl init` and CLI commands,
//! or by writing specific files to test validation of invalid states.

use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

// =============================================================================
// Test Helpers
// =============================================================================

/// Get today's date in YYYY-MM-DD format (same as govctl uses)
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Normalize output for stable snapshots:
/// - Replace temp directory paths with `<TEMPDIR>`
/// - Replace today's date with `<DATE>`
/// - Replace work item IDs (WI-YYYY-MM-DD-NNN) with WI-<DATE>-NNN
/// - Replace ADR IDs with date component normalized
fn normalize_output(output: &str, dir: &Path, date: &str) -> String {
    let dir_str = dir.display().to_string();
    let mut normalized = output.replace(&dir_str, "<TEMPDIR>");
    normalized = normalized.replace(date, "<DATE>");

    // Replace work item IDs
    let wi_pattern = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-(\d{3})").unwrap();
    normalized = wi_pattern
        .replace_all(&normalized, "WI-<DATE>-$1")
        .to_string();

    // Replace ADR filenames with dates
    let adr_file_pattern = regex::Regex::new(r"ADR-(\d{4})-").unwrap();
    normalized = adr_file_pattern
        .replace_all(&normalized, "ADR-XXXX-")
        .to_string();

    normalized
}

/// Run govctl commands in a directory and capture output.
fn run_commands(dir: &Path, commands: &[&[&str]]) -> String {
    let mut output = String::new();

    for args in commands {
        output.push_str(&format!("$ govctl {}\n", args.join(" ")));

        let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
            .args(*args)
            .current_dir(dir)
            .env("NO_COLOR", "1")
            .output()
            .expect("failed to run govctl");

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        if !stdout.is_empty() {
            output.push_str(&stdout);
            if !stdout.ends_with('\n') {
                output.push('\n');
            }
        }
        if !stderr.is_empty() {
            output.push_str(&stderr);
            if !stderr.ends_with('\n') {
                output.push('\n');
            }
        }

        output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    }

    output
}

/// Run commands with dynamic String arguments (for work item IDs with dates)
fn run_dynamic_commands(dir: &Path, commands: &[Vec<String>]) -> String {
    let mut output = String::new();

    for args in commands {
        let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        output.push_str(&format!("$ govctl {}\n", args_str.join(" ")));

        let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
            .args(&args_str)
            .current_dir(dir)
            .env("NO_COLOR", "1")
            .output()
            .expect("failed to run govctl");

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        if !stdout.is_empty() {
            output.push_str(&stdout);
            if !stdout.ends_with('\n') {
                output.push('\n');
            }
        }
        if !stderr.is_empty() {
            output.push_str(&stderr);
            if !stderr.ends_with('\n') {
                output.push('\n');
            }
        }

        output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    }

    output
}

/// Initialize a govctl project in a temp directory
fn init_project() -> TempDir {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(["init"])
        .current_dir(temp_dir.path())
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run govctl init");
    assert!(result.status.success(), "govctl init failed");
    temp_dir
}

// =============================================================================
// Happy Path Tests - Minimal Valid Project
// =============================================================================

/// Create a minimal valid project with RFC, clause, ADR, and work item
fn setup_minimal_valid(dir: &Path, date: &str) {
    let wi1 = format!("WI-{}-001", date);

    // Create RFC with clause
    let rfc_dir = dir.join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Test RFC",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Overview",
      "clauses": ["clauses/C-EXAMPLE.json"]
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
        rfc_dir.join("clauses/C-EXAMPLE.json"),
        r#"{
  "clause_id": "C-EXAMPLE",
  "title": "Example Clause",
  "kind": "normative",
  "status": "active",
  "text": "This is an example clause for testing.",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    // Create ADR
    fs::write(
        dir.join("gov/adr/ADR-0001-test-decision.toml"),
        r#"[govctl]
schema = "adr"
id = "ADR-0001"
status = "accepted"
refs = ["RFC-0001"]

[meta]
title = "Test Decision"
date = "2026-01-01"
deciders = ["test@example.com"]

[content]
context = "We need to test ADR functionality."
decision = "We will create a test ADR."
consequences = "Tests will pass."
"#,
    )
    .unwrap();

    // Create work item via commands
    let commands: Vec<Vec<String>> = vec![
        vec![
            "new".to_string(),
            "work".to_string(),
            "Test work item".to_string(),
            "--active".to_string(),
        ],
        vec![
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "add: Test criterion".to_string(),
        ],
    ];

    let _ = run_dynamic_commands(dir, &commands);
}

#[test]
fn test_minimal_valid_check() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_rfc() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["list", "rfc"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_clause() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["list", "clause"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_adr() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["list", "adr"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_list_work() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["list", "work"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_status() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(temp_dir.path(), &[&["status"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

#[test]
fn test_minimal_valid_full_workflow() {
    let temp_dir = init_project();
    let date = today();
    setup_minimal_valid(temp_dir.path(), &date);

    let output = run_commands(
        temp_dir.path(),
        &[
            &["check"],
            &["list", "rfc"],
            &["list", "clause"],
            &["list", "adr"],
            &["list", "work"],
            &["status"],
        ],
    );
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// =============================================================================
// Error Case Tests - Invalid States
// =============================================================================

/// Test: Clause claims superseded_by a non-existent clause
#[test]
fn test_broken_superseded_check() {
    let temp_dir = init_project();
    let date = today();

    // Create RFC with broken supersession
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Broken Superseded Test",
  "version": "1.0.0",
  "status": "normative",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Clauses",
      "clauses": ["clauses/C-OLD.json", "clauses/C-NEW.json"]
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

    // C-OLD claims to be superseded by C-NONEXISTENT (which doesn't exist)
    fs::write(
        rfc_dir.join("clauses/C-OLD.json"),
        r#"{
  "clause_id": "C-OLD",
  "title": "Old Clause",
  "kind": "normative",
  "status": "superseded",
  "text": "This clause is superseded.",
  "superseded_by": "C-NONEXISTENT",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-NEW.json"),
        r#"{
  "clause_id": "C-NEW",
  "title": "New Clause",
  "kind": "normative",
  "status": "active",
  "text": "This is the new clause.",
  "since": "1.0.0"
}"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

/// Test: RFC has invalid status/phase combination (draft + stable)
#[test]
fn test_invalid_transition_check() {
    let temp_dir = init_project();
    let date = today();

    // Create RFC with invalid state
    let rfc_dir = temp_dir.path().join("gov/rfc/RFC-0001");
    fs::create_dir_all(rfc_dir.join("clauses")).unwrap();

    fs::write(
        rfc_dir.join("rfc.json"),
        r#"{
  "rfc_id": "RFC-0001",
  "title": "Invalid Transition Test",
  "version": "0.1.0",
  "status": "draft",
  "phase": "stable",
  "owners": ["test@example.com"],
  "created": "2026-01-01",
  "sections": [
    {
      "title": "Overview",
      "clauses": ["clauses/C-TEST.json"]
    }
  ],
  "changelog": [
    {
      "version": "0.1.0",
      "date": "2026-01-01",
      "added": ["Initial draft"]
    }
  ]
}"#,
    )
    .unwrap();

    fs::write(
        rfc_dir.join("clauses/C-TEST.json"),
        r#"{
  "clause_id": "C-TEST",
  "title": "Test Clause",
  "kind": "normative",
  "status": "active",
  "text": "A test clause in an invalid RFC.",
  "since": "0.1.0"
}"#,
    )
    .unwrap();

    let output = run_commands(temp_dir.path(), &[&["check"]]);
    insta::assert_snapshot!(normalize_output(&output, temp_dir.path(), &date));
}

// =============================================================================
// Source Scan Tests
// =============================================================================

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

// =============================================================================
// Changelog/Release Workflow Tests
// =============================================================================

/// Test the full changelog/release workflow:
/// 1. Initialize project
/// 2. Create work items with various categories
/// 3. Cut a release
/// 4. Create more unreleased work items
/// 5. Render changelog and test release command
#[test]
fn test_changelog_release_workflow() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let dir = temp_dir.path();
    let date = today();

    // Build work item IDs with actual date
    let wi1 = format!("WI-{}-001", date);
    let wi2 = format!("WI-{}-002", date);
    let wi3 = format!("WI-{}-003", date);
    let wi4 = format!("WI-{}-004", date);

    // Phase 1: Setup - Initialize and create first batch of work items
    let setup_commands: Vec<Vec<String>> = vec![
        vec!["init".to_string()],
        vec![
            "new".to_string(),
            "work".to_string(),
            "Initial setup".to_string(),
            "--active".to_string(),
        ],
        vec![
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "add: Project scaffolding complete".to_string(),
        ],
        vec![
            "add".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "add: Basic configuration".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "scaffolding".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi1.clone(),
            "acceptance_criteria".to_string(),
            "configuration".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec!["mv".to_string(), wi1.clone(), "done".to_string()],
        vec![
            "new".to_string(),
            "work".to_string(),
            "Bug fixes".to_string(),
            "--active".to_string(),
        ],
        vec![
            "add".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "fix: Memory leak in parser".to_string(),
        ],
        vec![
            "add".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "fix: Crash on empty input".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "Memory leak".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi2.clone(),
            "acceptance_criteria".to_string(),
            "Crash".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec!["mv".to_string(), wi2.clone(), "done".to_string()],
        vec![
            "release".to_string(),
            "0.1.0".to_string(),
            "--date".to_string(),
            "2026-01-15".to_string(),
        ],
    ];

    let setup_output = run_dynamic_commands(dir, &setup_commands);
    insta::assert_snapshot!(
        "changelog_setup",
        normalize_output(&setup_output, dir, &date)
    );

    // Phase 2: Create unreleased work items for v0.2.0
    let unreleased_commands: Vec<Vec<String>> = vec![
        vec![
            "new".to_string(),
            "work".to_string(),
            "New features".to_string(),
            "--active".to_string(),
        ],
        vec![
            "add".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "add: User authentication".to_string(),
        ],
        vec![
            "add".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "security: Password hashing".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "authentication".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi3.clone(),
            "acceptance_criteria".to_string(),
            "Password".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec!["mv".to_string(), wi3.clone(), "done".to_string()],
        vec![
            "new".to_string(),
            "work".to_string(),
            "API changes".to_string(),
            "--active".to_string(),
        ],
        vec![
            "add".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "changed: Response format to JSON".to_string(),
        ],
        vec![
            "add".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "deprecated: Legacy XML endpoint".to_string(),
        ],
        vec![
            "add".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "removed: Obsolete v1 API".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "Response".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "Legacy".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec![
            "tick".to_string(),
            wi4.clone(),
            "acceptance_criteria".to_string(),
            "Obsolete".to_string(),
            "-s".to_string(),
            "done".to_string(),
        ],
        vec!["mv".to_string(), wi4.clone(), "done".to_string()],
    ];

    let unreleased_output = run_dynamic_commands(dir, &unreleased_commands);
    insta::assert_snapshot!(
        "changelog_unreleased",
        normalize_output(&unreleased_output, dir, &date)
    );

    // Phase 3: Test changelog rendering and release preview
    let changelog_output = run_commands(
        dir,
        &[
            &["status"],
            &["render", "changelog", "--dry-run"],
            &["release", "0.2.0", "--dry-run"],
        ],
    );
    insta::assert_snapshot!(
        "changelog_render",
        normalize_output(&changelog_output, dir, &date)
    );

    // Phase 4: Test error cases
    let error_output = run_commands(
        dir,
        &[&["release", "invalid-version"], &["release", "0.1.0"]],
    );
    insta::assert_snapshot!(
        "changelog_errors",
        normalize_output(&error_output, dir, &date)
    );
}
