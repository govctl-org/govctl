//! CLI snapshot tests using insta.
//!
//! These tests run govctl commands against fixture projects and snapshot
//! the combined stdout/stderr output for regression testing.

use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// Get the path to a test fixture
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Run multiple govctl commands against a fixture and capture output.
///
/// Returns a normalized string suitable for snapshot comparison:
/// - Absolute paths replaced with `<FIXTURE>`
/// - ANSI colors stripped via NO_COLOR=1
/// - Each command's invocation, output, and exit code recorded
fn run_commands(fixture: &str, commands: &[&[&str]]) -> String {
    let fixture_dir = fixture_path(fixture);
    let mut output = String::new();

    for args in commands {
        // Record the command being run
        output.push_str(&format!("$ govctl {}\n", args.join(" ")));

        let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
            .args(*args)
            .current_dir(&fixture_dir)
            .env("NO_COLOR", "1")
            .output()
            .expect("failed to run govctl");

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        // Normalize paths in output
        let fixture_str = fixture_dir.display().to_string();
        let normalized_stdout = stdout.replace(&fixture_str, "<FIXTURE>");
        let normalized_stderr = stderr.replace(&fixture_str, "<FIXTURE>");

        if !normalized_stdout.is_empty() {
            output.push_str(&normalized_stdout);
            if !normalized_stdout.ends_with('\n') {
                output.push('\n');
            }
        }
        if !normalized_stderr.is_empty() {
            output.push_str(&normalized_stderr);
            if !normalized_stderr.ends_with('\n') {
                output.push('\n');
            }
        }

        output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    }

    output
}

// =============================================================================
// Happy Path Tests
// =============================================================================

#[test]
fn test_minimal_valid_check() {
    let output = run_commands("minimal_valid", &[&["check"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_minimal_valid_list_rfc() {
    let output = run_commands("minimal_valid", &[&["list", "rfc"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_minimal_valid_list_clause() {
    let output = run_commands("minimal_valid", &[&["list", "clause"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_minimal_valid_list_adr() {
    let output = run_commands("minimal_valid", &[&["list", "adr"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_minimal_valid_list_work() {
    let output = run_commands("minimal_valid", &[&["list", "work"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_minimal_valid_status() {
    let output = run_commands("minimal_valid", &[&["status"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_minimal_valid_full_workflow() {
    let output = run_commands(
        "minimal_valid",
        &[
            &["check"],
            &["list", "rfc"],
            &["list", "clause"],
            &["list", "adr"],
            &["list", "work"],
            &["status"],
        ],
    );
    insta::assert_snapshot!(output);
}

// =============================================================================
// Error Case Tests
// =============================================================================

#[test]
fn test_broken_superseded_check() {
    let output = run_commands("broken_superseded", &[&["check"]]);
    insta::assert_snapshot!(output);
}

#[test]
fn test_invalid_transition_check() {
    let output = run_commands("invalid_transition", &[&["check"]]);
    insta::assert_snapshot!(output);
}

// =============================================================================
// Source Scan Tests
// =============================================================================

#[test]
fn test_source_scan_detects_refs() {
    let output = run_commands("source_scan", &[&["check"]]);
    insta::assert_snapshot!(output);
}

// =============================================================================
// Dynamic Fixture Tests (Changelog/Release Workflow)
// =============================================================================

/// Run govctl commands in a given directory path.
/// Similar to run_commands but takes a Path instead of fixture name.
fn run_commands_in_dir(dir: &Path, commands: &[&[&str]]) -> String {
    let mut output = String::new();
    let dir_str = dir.display().to_string();

    for args in commands {
        // Record the command being run
        output.push_str(&format!("$ govctl {}\n", args.join(" ")));

        let result = Command::new(env!("CARGO_BIN_EXE_govctl"))
            .args(*args)
            .current_dir(dir)
            .env("NO_COLOR", "1")
            .output()
            .expect("failed to run govctl");

        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);

        // Normalize paths and dates in output
        let normalized_stdout = stdout
            .replace(&dir_str, "<TEMPDIR>")
            .replace("2026-01-17", "<DATE>"); // Normalize today's date
        let normalized_stderr = stderr
            .replace(&dir_str, "<TEMPDIR>")
            .replace("2026-01-17", "<DATE>");

        if !normalized_stdout.is_empty() {
            output.push_str(&normalized_stdout);
            if !normalized_stdout.ends_with('\n') {
                output.push('\n');
            }
        }
        if !normalized_stderr.is_empty() {
            output.push_str(&normalized_stderr);
            if !normalized_stderr.ends_with('\n') {
                output.push('\n');
            }
        }

        output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    }

    output
}

/// Get today's date in YYYY-MM-DD format (same as govctl uses)
fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Normalize work item IDs and dates in output for stable snapshots
fn normalize_output(output: &str, date: &str) -> String {
    // Replace actual date with placeholder
    let mut normalized = output.replace(date, "<DATE>");
    
    // Replace work item IDs (WI-YYYY-MM-DD-NNN pattern)
    let wi_pattern = regex::Regex::new(r"WI-\d{4}-\d{2}-\d{2}-(\d{3})").unwrap();
    normalized = wi_pattern.replace_all(&normalized, "WI-<DATE>-$1").to_string();
    
    normalized
}

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
        vec!["new".to_string(), "work".to_string(), "Initial setup".to_string(), "--active".to_string()],
        vec!["add".to_string(), wi1.clone(), "acceptance_criteria".to_string(), "add: Project scaffolding complete".to_string()],
        vec!["add".to_string(), wi1.clone(), "acceptance_criteria".to_string(), "add: Basic configuration".to_string()],
        vec!["tick".to_string(), wi1.clone(), "acceptance_criteria".to_string(), "scaffolding".to_string(), "-s".to_string(), "done".to_string()],
        vec!["tick".to_string(), wi1.clone(), "acceptance_criteria".to_string(), "configuration".to_string(), "-s".to_string(), "done".to_string()],
        vec!["mv".to_string(), wi1.clone(), "done".to_string()],
        vec!["new".to_string(), "work".to_string(), "Bug fixes".to_string(), "--active".to_string()],
        vec!["add".to_string(), wi2.clone(), "acceptance_criteria".to_string(), "fix: Memory leak in parser".to_string()],
        vec!["add".to_string(), wi2.clone(), "acceptance_criteria".to_string(), "fix: Crash on empty input".to_string()],
        vec!["tick".to_string(), wi2.clone(), "acceptance_criteria".to_string(), "Memory leak".to_string(), "-s".to_string(), "done".to_string()],
        vec!["tick".to_string(), wi2.clone(), "acceptance_criteria".to_string(), "Crash".to_string(), "-s".to_string(), "done".to_string()],
        vec!["mv".to_string(), wi2.clone(), "done".to_string()],
        vec!["release".to_string(), "0.1.0".to_string(), "--date".to_string(), "2026-01-15".to_string()],
    ];
    
    let setup_output = run_string_commands_in_dir(dir, &setup_commands);
    insta::assert_snapshot!("changelog_setup", normalize_output(&setup_output, &date));

    // Phase 2: Create unreleased work items for v0.2.0
    let unreleased_commands: Vec<Vec<String>> = vec![
        vec!["new".to_string(), "work".to_string(), "New features".to_string(), "--active".to_string()],
        vec!["add".to_string(), wi3.clone(), "acceptance_criteria".to_string(), "add: User authentication".to_string()],
        vec!["add".to_string(), wi3.clone(), "acceptance_criteria".to_string(), "security: Password hashing".to_string()],
        vec!["tick".to_string(), wi3.clone(), "acceptance_criteria".to_string(), "authentication".to_string(), "-s".to_string(), "done".to_string()],
        vec!["tick".to_string(), wi3.clone(), "acceptance_criteria".to_string(), "Password".to_string(), "-s".to_string(), "done".to_string()],
        vec!["mv".to_string(), wi3.clone(), "done".to_string()],
        vec!["new".to_string(), "work".to_string(), "API changes".to_string(), "--active".to_string()],
        vec!["add".to_string(), wi4.clone(), "acceptance_criteria".to_string(), "changed: Response format to JSON".to_string()],
        vec!["add".to_string(), wi4.clone(), "acceptance_criteria".to_string(), "deprecated: Legacy XML endpoint".to_string()],
        vec!["add".to_string(), wi4.clone(), "acceptance_criteria".to_string(), "removed: Obsolete v1 API".to_string()],
        vec!["tick".to_string(), wi4.clone(), "acceptance_criteria".to_string(), "Response".to_string(), "-s".to_string(), "done".to_string()],
        vec!["tick".to_string(), wi4.clone(), "acceptance_criteria".to_string(), "Legacy".to_string(), "-s".to_string(), "done".to_string()],
        vec!["tick".to_string(), wi4.clone(), "acceptance_criteria".to_string(), "Obsolete".to_string(), "-s".to_string(), "done".to_string()],
        vec!["mv".to_string(), wi4.clone(), "done".to_string()],
    ];
    
    let unreleased_output = run_string_commands_in_dir(dir, &unreleased_commands);
    insta::assert_snapshot!("changelog_unreleased", normalize_output(&unreleased_output, &date));

    // Phase 3: Test changelog rendering and release preview
    let changelog_output = run_commands_in_dir(
        dir,
        &[
            &["status"],
            &["render", "changelog", "--dry-run"],
            &["release", "0.2.0", "--dry-run"],
        ],
    );
    insta::assert_snapshot!("changelog_render", normalize_output(&changelog_output, &date));

    // Phase 4: Test error cases
    let error_output = run_commands_in_dir(
        dir,
        &[
            &["release", "invalid-version"],
            &["release", "0.1.0"],
        ],
    );
    insta::assert_snapshot!("changelog_errors", normalize_output(&error_output, &date));
}

/// Run commands with String arguments (for dynamic work item IDs)
fn run_string_commands_in_dir(dir: &Path, commands: &[Vec<String>]) -> String {
    let mut output = String::new();
    let dir_str = dir.display().to_string();

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

        let normalized_stdout = stdout.replace(&dir_str, "<TEMPDIR>");
        let normalized_stderr = stderr.replace(&dir_str, "<TEMPDIR>");

        if !normalized_stdout.is_empty() {
            output.push_str(&normalized_stdout);
            if !normalized_stdout.ends_with('\n') {
                output.push('\n');
            }
        }
        if !normalized_stderr.is_empty() {
            output.push_str(&normalized_stderr);
            if !normalized_stderr.ends_with('\n') {
                output.push('\n');
            }
        }

        output.push_str(&format!("exit: {}\n\n", result.status.code().unwrap_or(-1)));
    }

    output
}
