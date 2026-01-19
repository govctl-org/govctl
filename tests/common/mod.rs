//! Common test helpers for CLI snapshot tests.

#![allow(dead_code)] // Functions used across different test binaries

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

/// Get today's date in YYYY-MM-DD format (same as govctl uses)
pub fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Normalize output for stable snapshots:
/// - Replace temp directory paths with `<TEMPDIR>`
/// - Replace today's date with `<DATE>`
/// - Replace work item IDs (WI-YYYY-MM-DD-NNN) with WI-<DATE>-NNN
/// - Replace ADR IDs with date component normalized
pub fn normalize_output(output: &str, dir: &Path, date: &str) -> String {
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
pub fn run_commands(dir: &Path, commands: &[&[&str]]) -> String {
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
pub fn run_dynamic_commands(dir: &Path, commands: &[Vec<String>]) -> String {
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
pub fn init_project() -> TempDir {
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
