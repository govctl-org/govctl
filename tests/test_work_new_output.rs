//! `work new` writes only the new Work Item ID on stdout.
//!
//! Implements [[RFC-0002:C-WORK-NEW-OUTPUT]].

mod common;

use common::{TestResult, init_project};
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn govctl(dir: &Path, args: &[&str]) -> Result<Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_govctl"))
        .args(args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GOVCTL_DEFAULT_OWNER", "@test-user")
        .output()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn success_writes_only_the_id_on_stdout() -> TestResult {
    let temp = init_project()?;
    let output = govctl(temp.path(), &["work", "new", "Scriptable item"])?;
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    let id = stdout.trim_end_matches('\n');
    assert_eq!(stdout, format!("{id}\n"));
    assert!(id.starts_with("WI-"), "stdout: {stdout}");
    assert!(!stdout.contains("Created"), "stdout: {stdout}");
    assert!(stderr.contains("Created work item"), "stderr: {stderr}");
    assert!(stderr.contains(&format!("ID: {id}")), "stderr: {stderr}");
    assert!(temp.path().join("gov/work").read_dir()?.next().is_some());
    Ok(())
}

#[test]
fn dry_run_writes_no_stdout_and_creates_no_file() -> TestResult {
    let temp = init_project()?;
    let output = govctl(temp.path(), &["--dry-run", "work", "new", "Planned item"])?;
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "stdout: {}", text(&output.stdout));
    assert!(stderr.contains("ID: WI-"), "stderr: {stderr}");
    assert!(stderr.contains("Would write"), "stderr: {stderr}");
    let created = temp
        .path()
        .join("gov/work")
        .read_dir()?
        .filter_map(Result::ok)
        .any(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"));
    assert!(!created, "dry-run created a work item file");
    Ok(())
}

#[test]
fn failure_writes_no_stdout_and_creates_no_file() -> TestResult {
    let temp = init_project()?;
    let work_dir = temp.path().join("gov/work");
    fs::remove_dir_all(&work_dir)?;
    fs::write(&work_dir, "not a directory")?;
    let output = govctl(temp.path(), &["work", "new", "Blocked item"])?;
    assert!(!output.status.success(), "failure should be non-zero");
    assert!(output.stdout.is_empty(), "stdout: {}", text(&output.stdout));
    assert_eq!(fs::read(&work_dir)?, b"not a directory");
    Ok(())
}
