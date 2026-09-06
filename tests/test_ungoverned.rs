//! Regression tests: read-only commands must not pollute or misrepresent
//! directories that are not governed by govctl.

mod common;

use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run(dir: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()
}

fn created_entries(dir: &Path) -> Result<Vec<String>, std::io::Error> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        entries.push(entry?.file_name().to_string_lossy().into_owned());
    }
    entries.sort();
    Ok(entries)
}

#[test]
fn status_reports_missing_governed_project() -> common::TestResult {
    let temp = TempDir::new()?;
    let output = run(temp.path(), &["status"])?;
    assert!(
        !output.status.success(),
        "status must fail outside a governed project"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E0502") && stderr.contains("No govctl project found"),
        "unexpected stderr: {stderr}"
    );
    assert_eq!(created_entries(temp.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn search_creates_no_local_state_outside_a_governed_project() -> common::TestResult {
    let temp = TempDir::new()?;
    let output = run(temp.path(), &["search", "anything"])?;
    assert!(
        !output.status.success(),
        "search must fail outside a governed project"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E0502") && stderr.contains("No govctl project found"),
        "unexpected stderr: {stderr}"
    );
    assert_eq!(created_entries(temp.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn show_creates_no_local_state_outside_a_governed_project() -> common::TestResult {
    let temp = TempDir::new()?;
    let output = run(temp.path(), &["work", "show", "WI-2026-01-01-001"])?;
    assert!(
        !output.status.success(),
        "work show must fail outside a governed project"
    );
    assert_eq!(created_entries(temp.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn governed_project_status_and_search_still_work() -> common::TestResult {
    let temp = common::init_project()?;
    let status = run(temp.path(), &["status"])?;
    assert!(
        status.status.success(),
        "status failed in a governed project:\n{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let search = run(temp.path(), &["search", "anything"])?;
    assert!(
        search.status.success(),
        "search failed in a governed project:\n{}",
        String::from_utf8_lossy(&search.stderr)
    );
    Ok(())
}
