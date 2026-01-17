//! CLI snapshot tests using insta.
//!
//! These tests run govctl commands against fixture projects and snapshot
//! the combined stdout/stderr output for regression testing.

use std::path::PathBuf;
use std::process::Command;

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

        output.push_str(&format!(
            "exit: {}\n\n",
            result.status.code().unwrap_or(-1)
        ));
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
